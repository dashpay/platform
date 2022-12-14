# syntax = docker/dockerfile:1.4
FROM dashpay/node:16-alpine as builder

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

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

# Enable corepack https://github.com/nodejs/corepack
RUN corepack enable

# Print build output
RUN yarn config set enableInlineBuilds true

WORKDIR /platform

# Copy yarn and Cargo files
COPY .yarn /platform/.yarn
COPY .cargo /platform/.cargo
COPY package.json yarn.lock .yarnrc.yml .pnp.* Cargo.toml Cargo.lock rust-toolchain.toml ./

# Copy only necessary packages from monorepo
COPY packages/js-drive packages/js-drive
COPY packages/rs-dpp packages/rs-dpp
COPY packages/wasm-dpp packages/wasm-dpp
COPY packages/rs-drive packages/rs-drive
COPY packages/rs-drive-abci packages/rs-drive-abci
COPY packages/rs-drive-nodejs packages/rs-drive-nodejs
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/dashpay-contract packages/dashpay-contract

# Build RS Drive Node.JS binding
RUN --mount=type=cache,target=target \
    --mount=type=cache,target=$CARGO_HOME/git \
    --mount=type=cache,target=$CARGO_HOME/registry \
    yarn workspace @dashevo/rs-drive build

# Install Drive-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production @dashevo/drive && \
    cp -R /platform/.yarn/unplugged /tmp/

FROM dashpay/node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

WORKDIR /platform

COPY --from=builder /platform /platform

# Remove Rust sources
RUN rm -rf packages/rs-drive packages/rs-drive-abci packages/rs-dpp

RUN cp /platform/packages/js-drive/.env.example /platform/packages/js-drive/.env

EXPOSE 26658
