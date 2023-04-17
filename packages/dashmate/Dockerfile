# syntax = docker/dockerfile:1.5
FROM strophy/buildbase:0.0.6 as builder

# TODO: Probably should be release by default
ARG CARGO_BUILD_PROFILE=debug
ENV CARGO_BUILD_PROFILE ${CARGO_BUILD_PROFILE}

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

ARG SCCACHE_GHA_ENABLED
ARG ACTIONS_CACHE_URL
ARG ACTIONS_RUNTIME_TOKEN

WORKDIR /platform

# Copy yarn and Cargo files
COPY .yarn /platform/.yarn
COPY .cargo /platform/.cargo
COPY package.json yarn.lock .yarnrc.yml .pnp.* Cargo.lock rust-toolchain.toml ./
# Use Cargo.toml.template instead of Cargo.toml from project root to avoid copying unnecessary Rust packages
COPY packages/dashmate/Cargo.toml.template ./Cargo.toml

# Print build output
RUN yarn config set enableInlineBuilds true

# Copy only necessary packages from monorepo
COPY packages/dashmate packages/dashmate
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/wallet-lib packages/wallet-lib
COPY packages/js-dash-sdk packages/js-dash-sdk
COPY packages/js-dapi-client packages/js-dapi-client
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/dash-spv packages/dash-spv
COPY packages/withdrawals-contract packages/withdrawals-contract
COPY packages/rs-platform-value packages/rs-platform-value
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/data-contracts packages/data-contracts
COPY packages/rs-dpp packages/rs-dpp
COPY packages/wasm-dpp packages/wasm-dpp

# Build WASM DPP
RUN --mount=type=cache,sharing=private,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    yarn workspace @dashevo/wasm-dpp build && \
    cargo clean && \
    sccache --show-stats

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production dashmate && \
    cp -R /platform/.yarn/unplugged /tmp/

# Remove Rust sources
RUN rm -rf packages/rs-dpp packages/rs-platform-value

FROM node:16-alpine3.16

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache docker-cli docker-cli-compose curl

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dashmate Helper Node.JS"

WORKDIR /platform

COPY --from=builder /platform /platform

ENTRYPOINT ["/platform/packages/dashmate/docker/entrypoint.sh"]
