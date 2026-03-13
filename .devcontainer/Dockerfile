# Dash Platform Dev Container with Claude Code sandbox support
FROM mcr.microsoft.com/devcontainers/base:ubuntu

ARG TZ=UTC
ENV TZ="$TZ"

# System dependencies for Rust, RocksDB, and protobuf
RUN apt-get update && apt-get install -y --no-install-recommends \
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
    unzip \
    libsnappy-dev \
    # Developer tools
    less \
    procps \
    fzf \
    man-db \
    ripgrep \
    fd-find \
    nano \
    vim \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Use clang as default C compiler
RUN rm -f /usr/bin/cc && ln -s /usr/bin/clang /usr/bin/cc

# Install protoc (pinned to 32.0)
ARG TARGETARCH
ARG PROTOC_VERSION=32.0
RUN if [ "$TARGETARCH" = "arm64" ]; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip" \
    -o /tmp/protoc.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

# Install git-delta for better diffs
ARG GIT_DELTA_VERSION=0.18.2
RUN ARCH=$(dpkg --print-architecture) && \
    curl -L "https://github.com/dandavison/delta/releases/download/${GIT_DELTA_VERSION}/git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" \
    -o /tmp/git-delta.deb && \
    dpkg -i /tmp/git-delta.deb && \
    rm /tmp/git-delta.deb

# Switch to vscode user for Rust and cargo tools
USER vscode

ENV CARGO_HOME=/home/vscode/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH

# Install Rust toolchain from rust-toolchain.toml
COPY --chown=vscode:vscode rust-toolchain.toml /tmp/rust-toolchain.toml
RUN TOOLCHAIN_VERSION="$(grep channel /tmp/rust-toolchain.toml | awk '{print $3}' | tr -d '"')" && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y \
    --default-toolchain "${TOOLCHAIN_VERSION}" \
    --target wasm32-unknown-unknown && \
    rm /tmp/rust-toolchain.toml

# Install cargo-binstall for fast binary installs
ARG BINSTALL_VERSION=1.10.11
RUN set -ex; \
    if [ "$(uname -m)" = "x86_64" ]; then \
        CARGO_BINSTALL_ARCH="x86_64-unknown-linux-musl"; \
    elif [ "$(uname -m)" = "aarch64" ]; then \
        CARGO_BINSTALL_ARCH="aarch64-unknown-linux-musl"; \
    else \
        echo "Unsupported architecture: $(uname -m)"; exit 1; \
    fi; \
    curl -L --fail --show-error \
        "https://github.com/cargo-bins/cargo-binstall/releases/download/v${BINSTALL_VERSION}/cargo-binstall-${CARGO_BINSTALL_ARCH}.tgz" \
        -o /tmp/cargo-binstall.tgz && \
    tar -xzf /tmp/cargo-binstall.tgz -C /tmp cargo-binstall && \
    chmod +x /tmp/cargo-binstall && \
    /tmp/cargo-binstall -y --force cargo-binstall && \
    rm /tmp/cargo-binstall.tgz /tmp/cargo-binstall

# Install wasm tools
RUN cargo binstall wasm-bindgen-cli@0.2.108 --locked \
        --no-discover-github-token --disable-telemetry --no-track --no-confirm && \
    cargo binstall wasm-pack --locked \
        --no-discover-github-token --disable-telemetry --no-track --no-confirm

# Prepare directories (need root for /workspace)
USER root
RUN mkdir -p /home/vscode/.claude /workspace && \
    chown -R vscode:vscode /home/vscode/.claude /workspace
USER vscode

ENV SHELL=/bin/zsh
ENV EDITOR=nano
ENV VISUAL=nano
