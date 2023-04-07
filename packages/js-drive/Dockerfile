# syntax = docker/dockerfile:1.5
FROM fuzz1986/builder:latest as builder

WORKDIR /platform

# Copy yarn and Cargo files
COPY .yarn /platform/.yarn
COPY .cargo /platform/.cargo
COPY package.json yarn.lock .yarnrc.yml .pnp.* Cargo.toml Cargo.lock rust-toolchain.toml ./

# Print build output
#RUN yarn config set enableInlineBuilds true

# Copy only necessary packages from monorepo
COPY packages/js-drive packages/js-drive
COPY packages/rs-dpp packages/rs-dpp
COPY packages/wasm-dpp packages/wasm-dpp
COPY packages/rs-drive packages/rs-drive
COPY packages/rs-drive-abci packages/rs-drive-abci
COPY packages/rs-platform-value packages/rs-platform-value
COPY packages/rs-drive-nodejs packages/rs-drive-nodejs
COPY packages/rs-drive-verify-c-binding packages/rs-drive-verify-c-binding
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/withdrawals-contract packages/withdrawals-contract
COPY packages/data-contracts packages/data-contracts

# Build RS Drive Node.JS binding
#RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
RUN yarn workspace @dashevo/rs-drive build

# Build WASM DPP
#RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
RUN yarn workspace @dashevo/wasm-dpp build
#    cargo clean

# Install Drive-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production @dashevo/drive && \
    cp -R /platform/.yarn/unplugged /tmp/

FROM node:16-alpine3.16

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

WORKDIR /platform

COPY --from=builder /platform /platform

# Remove Rust sources
RUN rm -rf packages/rs-drive packages/rs-drive-abci packages/rs-dpp packages/rs-platform-value

RUN cp /platform/packages/js-drive/.env.example /platform/packages/js-drive/.env

EXPOSE 26658
