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

# Install protoc - protobuf compiler
# The one shipped with Alpine does not work
ARG TARGETARCH
ARG PROTOC_VERSION=27.3
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip \
    -o /tmp/protoc.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

# Install protoc
RUN curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip \
    && unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip -d /usr/local \
    && rm protoc-${PROTOC_VERSION}-linux-x86_64.zip

# Switch to vscode user
USER vscode

ENV CARGO_HOME=/home/vscode/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH

# TODO: It doesn't sharing PATH between stages, so we need "source $HOME/.cargo/env" everywhere
COPY rust-toolchain.toml .
RUN TOOLCHAIN_VERSION="$(grep channel rust-toolchain.toml | awk '{print $3}' | tr -d '"')" && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y \
    --default-toolchain "${TOOLCHAIN_VERSION}" \
    --target wasm32-unknown-unknown

# Download and install cargo-binstall
ENV BINSTALL_VERSION=1.10.11
RUN set -ex; \
    if [ "$TARGETARCH" = "amd64" ]; then \
        CARGO_BINSTALL_ARCH="x86_64-unknown-linux-musl"; \
    elif [ "$TARGETARCH" = "arm64" ] || [ "$TARGETARCH" = "aarch64" ]; then \
        CARGO_BINSTALL_ARCH="aarch64-unknown-linux-musl"; \
    else \
        echo "Unsupported architecture: $TARGETARCH"; exit 1; \
    fi; \
    DOWNLOAD_URL="https://github.com/cargo-bins/cargo-binstall/releases/download/v${BINSTALL_VERSION}/cargo-binstall-${CARGO_BINSTALL_ARCH}.tgz"; \
    curl -L --fail --show-error "$DOWNLOAD_URL" -o /tmp/cargo-binstall.tgz; \
    tar -xzf /tmp/cargo-binstall.tgz -C /tmp cargo-binstall; \
    chmod +x /tmp/cargo-binstall; \
    /tmp/cargo-binstall -y --force cargo-binstall; \
    rm /tmp/cargo-binstall; \
    cargo binstall -V

RUN cargo binstall wasm-bindgen-cli@0.2.100 --locked \
                                           --no-discover-github-token \
                                           --disable-telemetry \
                                           --no-track \
                                           --no-confirm
