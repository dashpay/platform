# Use the official VS Code base image for dev containers
FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    clang \
    cmake \
    llvm \
    curl \
    gnupg \
    lsb-release \
    software-properties-common \
    unzip

# Switch to clang
RUN rm /usr/bin/cc && ln -s /usr/bin/clang /usr/bin/cc

ENV CARGO_HOME=/root/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH

# TODO: It doesn't sharing PATH between stages, so we need "source $HOME/.cargo/env" everywhere
COPY rust-toolchain.toml .
RUN TOOLCHAIN_VERSION="$(grep channel rust-toolchain.toml | awk '{print $3}' | tr -d '"')" && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    --profile minimal \
    -y \
    --default-toolchain "${TOOLCHAIN_VERSION}" \
    --target wasm32-unknown-unknown

# Install protoc - protobuf compiler
# The one shipped with Alpine does not work
ARG TARGETARCH
ARG PROTOC_VERSION=25.2
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip \
    -o /tmp/protoc.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

# Install protoc v25.2+
RUN curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v25.2/protoc-25.2-linux-x86_64.zip \
    && unzip protoc-25.2-linux-x86_64.zip -d /usr/local \
    && rm protoc-25.2-linux-x86_64.zip

# Install wasm-bindgen-cli in the same profile as other components, to sacrifice some performance & disk space to gain
# better build caching
RUN if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    RUSTFLAGS="-C target-feature=-crt-static" \
    # Meanwhile if you want to update wasm-bindgen you also need to update version in:
    #  - packages/wasm-dpp/Cargo.toml
    #  - packages/wasm-dpp/scripts/build-wasm.sh
    cargo install wasm-bindgen-cli@0.2.86 --locked
