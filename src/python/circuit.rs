use std::sync::Arc;

use crate::ir::operation::Operation;
use crate::ir::circuit::Circuit;
use crate::ir::gates::Gradient;
use crate::ir::gates::Unitary;
use crate::ir::gates::*;

use ndarray::Array2;
use ndarray_linalg::c64;

use numpy::IntoPyArray;
use numpy::PyArray2;
use numpy::PyArray3;
use pyo3::exceptions;
use pyo3::types::PyTuple;
use pyo3::{prelude::*, types::PyIterator};

use super::gate::PyGate;

fn pygate_to_native(pygate: &PyAny, constant_gates: &mut Vec<Array2<c64>>) -> PyResult<Gate> {
    let cls = pygate.getattr("__class__")?;
    let dunder_name = cls.getattr("__name__")?;
    let name = dunder_name.extract::<&str>()?;
    match name {
        "CRXGate" => Ok(CRXGate::new().into()),
        "CRYGate" => Ok(CRYGate::new().into()),
        "CRZGate" => Ok(CRZGate::new().into()),
        "RXGate" => Ok(RXGate::new().into()),
        "RYGate" => Ok(RYGate::new().into()),
        "RZGate" => Ok(RZGate::new().into()),
        "RXXGate" => Ok(RXXGate::new().into()),
        "RYYGate" => Ok(RYYGate::new().into()),
        "RZZGate" => Ok(RZZGate::new().into()),
        "U1Gate" => Ok(U1Gate::new().into()),
        "U2Gate" => Ok(U2Gate::new().into()),
        "U3Gate" => Ok(U3Gate::new().into()),
        "U8Gate" => Ok(U8Gate::new().into()),
        "VariableUnitaryGate" => {
            let size = pygate.getattr("num_qudits")?.extract::<usize>()?;
            let radixes = pygate.getattr("radixes")?.extract::<Vec<usize>>()?;
            Ok(VariableUnitaryGate::new(size, radixes).into())
        }
        _ => {
            if pygate.getattr("num_params")?.extract::<usize>()? == 0 {
                let args: Vec<f64> = vec![];
                let pyobj = pygate.call_method("get_unitary", (args,), None)?;
                let pymat = pyobj.getattr("numpy")?.extract::<&PyArray2<c64>>()?;
                let mat = pymat.to_owned_array();
                let gate_size = pygate.getattr("num_qudits")?.extract::<usize>()?;
                let index = constant_gates.len();
                constant_gates.push(mat);
                Ok(ConstantGate::new(index, gate_size).into())
            } else if pygate.hasattr("get_unitary")?
                && ((pygate.hasattr("get_grad")? && pygate.hasattr("get_unitary_and_grad")?)
                    || pygate.hasattr("optimize")?)
            {
                let dynamic: Arc<dyn DynGate + Send + Sync> = Arc::new(PyGate::new(pygate.into()));
                Ok(Gate::Dynamic(dynamic))
            } else {
                Err(exceptions::PyValueError::new_err(format!(
                    "Gate {} does not implement the necessary methods for optimization.",
                    name
                )))
            }
        }
    }
}

impl<'source> FromPyObject<'source> for Circuit {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let size = ob.getattr("num_qudits")?.extract::<usize>()?;
        let radixes = ob.getattr("radixes")?.extract::<Vec<usize>>()?;
        let circ_iter = ob.call_method0("operations_with_cycles")?;
        let iter = PyIterator::from_object(py, circ_iter)?;
        let mut ops = vec![];
        let mut constant_gates = vec![];
        for cycle_with_operation in iter {
            let tup = cycle_with_operation?.downcast::<PyTuple>()?;
            let py_cycle = tup.get_item(0)?;
            let op = tup.get_item(1)?;
            let cycle = py_cycle.extract::<usize>()?;
            let pygate = op.getattr("gate")?;
            let location = op.getattr("location")?.extract::<Vec<usize>>()?;
            let params = op.getattr("params")?.extract::<Vec<f64>>()?;
            let gate = pygate_to_native(pygate, &mut constant_gates)?;
            ops.push((cycle, Operation::new(gate, location, params)));
        }
        Ok(Circuit::new(
            size,
            radixes,
            ops,
            constant_gates,
        ))
    }
}

#[pyclass(name = "Circuit", subclass, unsendable, module = "bqskitrs")]
pub struct PyCircuit {
    circ: Circuit,
}

#[pymethods]
impl PyCircuit {
    #[new]
    pub fn new(circ: Circuit) -> Self {
        PyCircuit { circ }
    }

    pub fn get_unitary(&self, py: Python, params: Vec<f64>) -> Py<PyArray2<c64>> {
        self.circ
            .get_utry(&params, &self.circ.constant_gates)
            .into_pyarray(py)
            .to_owned()
    }

    pub fn get_grad(&self, py: Python, params: Vec<f64>) -> Py<PyArray3<c64>> {
        let grad = self.circ.get_grad(&params, &self.circ.constant_gates);
        grad.into_pyarray(py).to_owned()
    }

    pub fn get_unitary_and_grad(
        &self,
        py: Python,
        params: Vec<f64>,
    ) -> (Py<PyArray2<c64>>, Py<PyArray3<c64>>) {
        let (utry, grad) = self
            .circ
            .get_utry_and_grad(&params, &self.circ.constant_gates);
        (
            utry.into_pyarray(py).to_owned(),
            grad.into_pyarray(py).to_owned(),
        )
    }
}
