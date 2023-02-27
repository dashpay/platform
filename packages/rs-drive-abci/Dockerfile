# syntax = docker/dockerfile:1.3
FROM node:16-alpine3.16 as builder

ARG CARGO_BUILD_PROFILE=debug
ENV CARGO_BUILD_PROFILE ${CARGO_BUILD_PROFILE}

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       linux-headers \
                       python3 \
                       alpine-sdk \
                       cmake \
                       zeromq-dev \
                       ca-certificates \
                       gcc \
                       clang \
                       libc-dev \
                       binutils \
                       bash

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=stable

RUN set -eux; \
    apkArch="$(apk --print-arch)"; \
    case "$apkArch" in \
        x86_64) rustArch='x86_64-unknown-linux-musl'; rustupSha256='bdf022eb7cba403d0285bb62cbc47211f610caec24589a72af70e1e900663be9' ;; \
        aarch64) rustArch='aarch64-unknown-linux-musl'; rustupSha256='89ce657fe41e83186f5a6cdca4e0fd40edab4fd41b0f9161ac6241d49fbdbbbe' ;; \
        *) echo >&2 "unsupported architecture: $apkArch"; exit 1 ;; \
    esac; \
    url="https://static.rust-lang.org/rustup/archive/1.24.3/${rustArch}/rustup-init"; \
    wget "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;

WORKDIR /platform

# Copy yarn and Cargo files
COPY .cargo /platform/.cargo
COPY .pnp.* Cargo.toml Cargo.lock rust-toolchain.toml ./

# Copy only necessary packages from monorepo
COPY packages/rs-dpp packages/rs-dpp
COPY packages/rs-drive packages/rs-drive
COPY packages/rs-drive-abci packages/rs-drive-abci
COPY packages/rs-platform-value packages/rs-platform-value
COPY packages/rs-drive-nodejs packages/rs-drive-nodejs
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/withdrawals-contract packages/withdrawals-contract
COPY packages/data-contracts packages/data-contracts

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive ABCI Rust"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

WORKDIR /platform

COPY --from=builder /platform /platform

# Remove Rust sources
RUN rm -rf packages/rs-drive packages/rs-drive-abci packages/rs-dpp packages/rs-platform-value

RUN cp /platform/packages/rs-drive-abci/.env.example /platform/packages/rs-drive-abci/.env

EXPOSE 26658
