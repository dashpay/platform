# syntax = docker/dockerfile:1.5
FROM strophy/buildbase:0.0.4 as builder

WORKDIR /platform

# Copy yarn and Cargo files
COPY .yarn /platform/.yarn
COPY .cargo /platform/.cargo
COPY package.json yarn.lock .yarnrc.yml .pnp.* Cargo.lock rust-toolchain.toml ./
# Use Cargo.toml.template instead of Cargo.toml from project root to avoid copying unnecessary Rust packages
COPY packages/dapi/Cargo.toml.template ./Cargo.toml

# Print build output
RUN yarn config set enableInlineBuilds true

# Copy only necessary packages from monorepo
COPY packages/dapi packages/dapi
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/js-dpp packages/js-dpp
COPY packages/rs-dpp packages/rs-dpp
COPY packages/wasm-dpp packages/wasm-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/dpns-contract packages/dpns-contract
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/data-contracts packages/data-contracts
COPY packages/rs-platform-value packages/rs-platform-value
COPY packages/withdrawals-contract packages/withdrawals-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/feature-flags-contract packages/feature-flags-contract

# Build WASM DPP

# TODO(wasm-dpp): revisit. This cache seem to be breaking installation of GitHub hosted dependencies
#RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
#    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
#    yarn workspace @dashevo/wasm-dpp build && \
#    cargo clean

RUN --mount=type=cache,target=target \
    --mount=type=cache,target=$CARGO_HOME/git \
    --mount=type=cache,target=$CARGO_HOME/registry \
    yarn workspace @dashevo/wasm-dpp build

# Install DAPI-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
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

# Remove Rust sources
RUN rm -rf packages/rs-dpp packages/rs-platform-value

RUN cp /platform/packages/dapi/.env.example /platform/packages/dapi/.env

EXPOSE 2500 2501 2510
