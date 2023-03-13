# syntax = docker/dockerfile:1.3

# Build wasm-dpp
FROM rust as builder

RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y clang

WORKDIR /platform

COPY .cargo /platform/.cargo
COPY Cargo.lock rust-toolchain.toml ./
# Use Cargo.toml.template instead of Cargo.toml from project root to avoid copying unnecessary Rust packages
COPY packages/dapi/Cargo.toml.template ./Cargo.toml

COPY packages/withdrawals-contract packages/withdrawals-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/data-contracts packages/data-contracts
COPY packages/rs-dpp packages/rs-dpp
COPY packages/wasm-dpp packages/wasm-dpp

RUN --mount=type=cache,target=target \
    --mount=type=cache,target=$CARGO_HOME/git \
    --mount=type=cache,target=$CARGO_HOME/registry \
    rustup target add wasm32-unknown-unknown

# Build Rust part of wasm-dpp
RUN cd packages/wasm-dpp && ./scripts/build.sh

# Build for DAPI
FROM node:16-alpine3.16 as builder

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       python3 \
                       alpine-sdk \
                       zeromq-dev

# Enable corepack https://github.com/nodejs/corepack
RUN corepack enable

WORKDIR /platform

# Copy yarn files
COPY .yarn ./.yarn
COPY package.json yarn.lock .yarnrc.yml .pnp.* ./

# Copy only necessary packages from monorepo
COPY packages/dapi packages/dapi
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/dashpay-contract packages/dashpay-contract
COPY --from=0 /platform/packages/wasm-dpp packages/wasm-dpp

# Print build output
RUN yarn config set enableInlineBuilds true

# Install DAPI-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus @dashevo/wasm-dpp && \
    yarn workspace @dashevo/wasm-dpp webpack && \
    yarn workspaces focus --production @dashevo/dapi && \
    cp -R /platform/.yarn/unplugged /tmp/


FROM node:16-alpine3.16

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

WORKDIR /platform

COPY --from=builder /platform /platform

RUN cp /platform/packages/dapi/.env.example /platform/packages/dapi/.env

EXPOSE 2500 2501 2510
