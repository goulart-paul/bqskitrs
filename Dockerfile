FROM quay.io/pypa/manylinux2014_x86_64
# This docker file is based on the official maturin docker file https://github.com/PyO3/maturin/blob/master/Dockerfile
ENV PATH /root/.cargo/bin:$PATH
# Add all supported python versions
ENV PATH /opt/python/cp38-cp38/bin/:/opt/python/cp39-cp39/bin/:/opt/python/cp310-cp310/bin/:$PATH
# Otherwise `cargo new` errors
ENV USER root

ENV CMAKE cmake3

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    && rustup set profile minimal \
    && python3 -m pip install --no-cache-dir cffi \
    && mkdir /io

RUN git clone https://github.com/PyO3/maturin /maturin/

RUN cargo rustc --bin maturin --manifest-path /maturin/Cargo.toml -- -C link-arg=-s \
    && mv /maturin/target/debug/maturin /usr/bin/maturin \
    && rm -rf /maturin

RUN yum install -y git cmake3 eigen3-devel llvm-toolset-7 && \
    yum clean all && \
    rm -rf /var/cache/yum

WORKDIR /io

ENTRYPOINT ["/usr/bin/maturin"]
