# Build wasm-dpp
FROM rust as builder

RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y clang

WORKDIR /platform

COPY .cargo /platform/.cargo
COPY Cargo.lock rust-toolchain.toml ./

RUN --mount=type=cache,target=target \
    --mount=type=cache,target=$CARGO_HOME/git \
    --mount=type=cache,target=$CARGO_HOME/registry \
    rustup target add wasm32-unknown-unknown && \
    cargo install -f wasm-bindgen-cli

# Use Cargo.toml.template instead of Cargo.toml from project root to avoid copying unnecessary Rust packages
COPY packages/dashmate/Cargo.toml.template ./Cargo.toml

COPY packages/rs-platform-value packages/rs-platform-value
COPY packages/withdrawals-contract packages/withdrawals-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/data-contracts packages/data-contracts
COPY packages/rs-dpp packages/rs-dpp
COPY packages/wasm-dpp packages/wasm-dpp

# Build Rust part of wasm-dpp
RUN cd packages/wasm-dpp && ./scripts/build.sh

FROM node:16-alpine3.16 as builder

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       python3 \
                       alpine-sdk \
                       make \
                       cmake \
                       g++

# Enable corepack https://github.com/nodejs/corepack
RUN corepack enable

WORKDIR /platform

# Copy yarn files
COPY .yarn ./.yarn
COPY package.json yarn.lock .yarnrc.yml .pnp.* ./

# Copy only necessary packages from monorepo
COPY packages/dashmate packages/dashmate
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/wallet-lib packages/wallet-lib
COPY packages/js-dash-sdk packages/js-dash-sdk
COPY packages/js-dapi-client packages/js-dapi-client
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/dash-spv packages/dash-spv
COPY packages/withdrawals-contract packages/withdrawals-contract
COPY --from=0 /platform/packages/wasm-dpp packages/wasm-dpp

# Print build output
RUN yarn config set enableInlineBuilds true

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus @dashevo/wasm-dpp && \
    yarn workspace @dashevo/wasm-dpp tsc && \
    yarn workspaces focus --production dashmate && \
    cp -R /platform/.yarn/unplugged /tmp/

FROM node:16-alpine3.16

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}
ENV DASHMATE_HELPER=1

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache docker-cli docker-cli-compose

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dashmate Helper Node.JS"


WORKDIR /platform

COPY --from=builder /platform /platform

ENTRYPOINT ["/platform/packages/dashmate/docker/entrypoint.sh"]
