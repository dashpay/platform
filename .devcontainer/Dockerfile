# Use the official VS Code base image for dev containers
FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

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

# Install protoc v25.2+
RUN curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v25.2/protoc-25.2-linux-x86_64.zip \
    && unzip protoc-25.2-linux-x86_64.zip -d /usr/local \
    && rm protoc-25.2-linux-x86_64.zip

# Install wasm-bindgen-cli@0.2.85
RUN cargo install wasm-bindgen-cli@0.2.85
