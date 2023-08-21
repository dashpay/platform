# syntax = docker/dockerfile:1.5

# Docker image for rs-drive-abci
#
# This image is divided multiple parts:
# - deps-base - includes all base dependencies and some libraries
# - deps-sccache - deps image with sccache included
# - deps - all deps, including wasm-bindgen-cli; built on top of either deps-base or deps-sccache
# - sources - includes full source code
# - build-* - actual build process of given image
# - drive-abci, dashmate-helper, test-suite, dapi - final images
#
# The following build arguments can be provided using --build-arg:
# - CARGO_BUILD_PROFILE - set to `release` to build final binary, without debugging information
# - NODE_ENV - node.js environment name to use to build the library
# - RUSTC_WRAPPER - set to `sccache` to enable sccache support and make the following variables avaialable:
#   - SCCACHE_GHA_ENABLED, ACTIONS_CACHE_URL, ACTIONS_RUNTIME_TOKEN - store sccache caches inside github actions
#   - SCCACHE_MEMCACHED - set to memcache server URI (eg. tcp://172.17.0.1:11211) to enable sccache memcached backend
# - ALPINE_VERSION - use different version of Alpine base image; requires also rust:apline...
#   image to be available
# - USERNAME, USER_UID, USER_GID - specification of user used to run the binary
#
# BUILD PROCESS
#
# 1. All these --mount... are to cache reusable info between runs.
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
# 2. We add `--config net.git-fetch-with-cli=true` to address ARM build issue,
# see https://github.com/rust-lang/cargo/issues/10781#issuecomment-1441071052
# 3. Github Actions have shared networking configured, so we need to set a random
# SCCACHE_SERVER_PORT port to avoid conflicts in case of parallel compilation

ARG ALPINE_VERSION=3.18

# Set RUSTC_WRAPPER to `sccache` to enable sccache caching
ARG RUSTC_WRAPPER

#
# DEPS: INSTALL AND CACHE DEPENDENCIES
#
FROM rust:alpine${ALPINE_VERSION} as deps-base

#
# Install some dependencies
#
RUN apk add --no-cache \
        alpine-sdk \
        bash \
        binutils \
        ca-certificates \
        clang-static clang-dev \
        cmake \
        git \
        libc-dev \
        linux-headers \
        llvm-static llvm-dev  \
        'nodejs~=18' \
        npm \
        openssl-dev \
        perl \
        python3 \
        unzip \
        wget \
        xz \
        zeromq-dev

SHELL ["/bin/bash", "-c"]

ARG TARGETARCH

RUN rustup install stable && \
    rustup target add wasm32-unknown-unknown --toolchain stable

# Install protoc - protobuf compiler
# The one shipped with Alpine does not work
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls https://github.com/protocolbuffers/protobuf/releases/download/v22.4/protoc-22.4-linux-${PROTOC_ARCH}.zip \
        -o /tmp/protoc.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

# Configure Node.js
RUN npm config set audit false && \
    npm install -g npm@9.6.6 && \
    npm install -g corepack@latest && \
    corepack enable

# Switch to clang
RUN rm /usr/bin/cc && ln -s /usr/bin/clang /usr/bin/cc


# Select whether we want dev or release
ARG CARGO_BUILD_PROFILE=dev
ENV CARGO_BUILD_PROFILE ${CARGO_BUILD_PROFILE}

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

ARG RUSTC_WRAPPER

FROM deps-base AS deps-sccache

# Install sccache for caching
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export SCC_ARCH=aarch64; else export SCC_ARCH=x86_64; fi; \
    curl -Ls \
        https://github.com/mozilla/sccache/releases/download/v0.4.1/sccache-v0.4.1-${SCC_ARCH}-unknown-linux-musl.tar.gz | \
        tar -C /tmp -xz && \
        mv /tmp/sccache-*/sccache /usr/bin/

#
# Configure sccache
#
# Set args below to use Github Actions cache; see https://github.com/mozilla/sccache/blob/main/docs/GHA.md
ARG SCCACHE_GHA_ENABLED
ARG ACTIONS_CACHE_URL
ARG ACTIONS_RUNTIME_TOKEN
# Alternative solution is to use memcache
ARG SCCACHE_MEMCACHED

# Disable incremental buildings, not supported by sccache
ARG CARGO_INCREMENTAL=false

#
# DEPS: FULL DEPENCIES LIST
#
# This is separate from `deps` to use sccache for caching
FROM deps-${RUSTC_WRAPPER:-base} AS deps

# Install wasm-bindgen-cli in the same profile as other components, to sacrifice some performance & disk space to gain
# better build caching
WORKDIR /platform
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_${TARGETARCH},target=/platform/target \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    RUSTFLAGS="-C target-feature=-crt-static" \
    CARGO_TARGET_DIR="/platform/target" \
    # TODO: Build wasm with build.rs
    # Meanwhile if you want to update wasm-bindgen you also need to update version in:
    #  - packages/wasm-dpp/Cargo.toml
    #  - packages/wasm-dpp/scripts/build-wasm.sh
    cargo install --profile "$CARGO_BUILD_PROFILE" wasm-bindgen-cli@0.2.86

#
# LOAD SOURCES
#
FROM deps as sources


WORKDIR /platform

COPY . .

# print the JS build output
RUN yarn config set enableInlineBuilds true

#
# STAGE: BUILD RS-DRIVE-ABCI
#
# This will prebuild majority of dependencies
FROM sources AS build-drive-abci

RUN mkdir /artifacts

RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_${TARGETARCH},target=/platform/target \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    cargo build \
        --profile "$CARGO_BUILD_PROFILE" \
        --package drive-abci && \
    cp /platform/target/*/drive-abci /artifacts/ && \
    if [[ "${RUSTC_WRAPPER}" == "sccache" ]] ; then sccache --show-stats; fi

#
# STAGE: BUILD JAVASCRIPT INTERMEDIATE IMAGE
#
FROM sources AS build-js

RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_wasm,target=/platform/target \
    --mount=type=cache,sharing=shared,id=unplugged_${TARGETARCH},target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn install && \
    cp -R /platform/.yarn/unplugged /tmp/ && \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    export SKIP_GRPC_PROTO_BUILD=1 && \
    yarn build && \
    if [[ "${RUSTC_WRAPPER}" == "sccache" ]]; then sccache --show-stats; fi

#
# STAGE: FINAL DRIVE-ABCI IMAGE
#
FROM alpine:${ALPINE_VERSION} AS drive-abci

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive ABCI Rust"

WORKDIR /var/lib/dash

RUN apk add --no-cache libgcc libstdc++

COPY --from=build-drive-abci /artifacts/drive-abci /usr/bin/drive-abci
COPY --from=build-drive-abci /platform/packages/rs-drive-abci/.env.example /var/lib/dash/rs-drive-abci/.env

# Double-check that we don't have missing deps
RUN ldd /usr/bin/drive-abci

# Create a volume
VOLUME /var/lib/dash
VOLUME /var/log/dash

RUN mkdir -p /var/log/dash

ENV DB_PATH=/var/lib/dash/rs-drive-abci/db

#
# Create new non-root user
#
ARG USERNAME=dash
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN addgroup -g $USER_GID $USERNAME && \
    adduser -D -u $USER_UID -G $USERNAME -h /var/lib/dash/rs-drive-abci $USERNAME && \
    chown -R $USER_UID:$USER_GID /var/lib/dash/rs-drive-abci /var/log/dash

USER $USERNAME

ENV RUST_BACKTRACE=1
WORKDIR /var/lib/dash/rs-drive-abci
ENTRYPOINT ["/usr/bin/drive-abci"]
CMD ["start"]

# ABCI interface
EXPOSE 26658
# Prometheus port
EXPOSE 29090

#
# STAGE: DASHMATE HELPER BUILD
#
FROM build-js AS build-dashmate-helper

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN yarn workspaces focus --production dashmate

#
#  STAGE: FINAL DASHMATE HELPER IMAGE
#
FROM node:18-alpine${ALPINE_VERSION} AS dashmate-helper

RUN apk add --no-cache docker-cli docker-cli-compose curl

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dashmate Helper Node.JS"

WORKDIR /platform

COPY --from=build-dashmate-helper /platform/.yarn /platform/.yarn
COPY --from=build-dashmate-helper /platform/package.json /platform/yarn.lock /platform/.yarnrc.yml /platform/.pnp* /platform/

# Copy only necessary packages from monorepo
COPY --from=build-dashmate-helper /platform/packages/dashmate packages/dashmate
COPY --from=build-dashmate-helper /platform/packages/dashpay-contract packages/dashpay-contract
COPY --from=build-dashmate-helper /platform/packages/js-dpp packages/js-dpp
COPY --from=build-dashmate-helper /platform/packages/wallet-lib packages/wallet-lib
COPY --from=build-dashmate-helper /platform/packages/js-dash-sdk packages/js-dash-sdk
COPY --from=build-dashmate-helper /platform/packages/js-dapi-client packages/js-dapi-client
COPY --from=build-dashmate-helper /platform/packages/js-grpc-common packages/js-grpc-common
COPY --from=build-dashmate-helper /platform/packages/dapi-grpc packages/dapi-grpc
COPY --from=build-dashmate-helper /platform/packages/dash-spv packages/dash-spv
COPY --from=build-dashmate-helper /platform/packages/withdrawals-contract packages/withdrawals-contract
COPY --from=build-dashmate-helper /platform/packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY --from=build-dashmate-helper /platform/packages/feature-flags-contract packages/feature-flags-contract
COPY --from=build-dashmate-helper /platform/packages/dpns-contract packages/dpns-contract
COPY --from=build-dashmate-helper /platform/packages/data-contracts packages/data-contracts
COPY --from=build-dashmate-helper /platform/packages/wasm-dpp packages/wasm-dpp

USER node

ENV DASHMATE_HOME_DIR=/home/dashmate/.dashmate
ENV DASHMATE_HELPER=1

ENTRYPOINT ["/platform/packages/dashmate/docker/entrypoint.sh"]

#
# STAGE: TEST SUITE BUILD
#
FROM build-js AS build-test-suite

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN yarn workspaces focus --production @dashevo/platform-test-suite

#
#  STAGE: FINAL TEST SUITE IMAGE
#
FROM node:18-alpine${ALPINE_VERSION} AS test-suite

RUN apk add --no-cache bash

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dash Platform test suite"

WORKDIR /platform

COPY --from=build-test-suite /platform /platform


# Copy yarn and Cargo files
COPY --from=build-test-suite /platform/.yarn /platform/.yarn
COPY --from=build-test-suite /platform/package.json /platform/yarn.lock \
    /platform/.yarnrc.yml /platform/.pnp.* /platform/Cargo.lock /platform/rust-toolchain.toml ./
# Use Cargo.toml.template instead of Cargo.toml from project root to avoid copying unnecessary Rust packages
COPY --from=build-test-suite /platform/packages/platform-test-suite/Cargo.toml.template ./Cargo.toml

# Copy only necessary packages from monorepo
COPY --from=build-test-suite /platform/packages/platform-test-suite packages/platform-test-suite
COPY --from=build-test-suite /platform/packages/dashpay-contract packages/dashpay-contract
COPY --from=build-test-suite /platform/packages/js-dpp packages/js-dpp
COPY --from=build-test-suite /platform/packages/wallet-lib packages/wallet-lib
COPY --from=build-test-suite /platform/packages/js-dash-sdk packages/js-dash-sdk
COPY --from=build-test-suite /platform/packages/js-dapi-client packages/js-dapi-client
COPY --from=build-test-suite /platform/packages/js-grpc-common packages/js-grpc-common
COPY --from=build-test-suite /platform/packages/dapi-grpc packages/dapi-grpc
COPY --from=build-test-suite /platform/packages/dash-spv packages/dash-spv
COPY --from=build-test-suite /platform/packages/withdrawals-contract packages/withdrawals-contract
COPY --from=build-test-suite /platform/packages/rs-platform-value packages/rs-platform-value
COPY --from=build-test-suite /platform/packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY --from=build-test-suite /platform/packages/feature-flags-contract packages/feature-flags-contract
COPY --from=build-test-suite /platform/packages/dpns-contract packages/dpns-contract
COPY --from=build-test-suite /platform/packages/data-contracts packages/data-contracts
COPY --from=build-test-suite /platform/packages/rs-platform-serialization packages/rs-platform-serialization
COPY --from=build-test-suite /platform/packages/rs-platform-value-convertible packages/rs-platform-value-convertible
COPY --from=build-test-suite /platform/packages/rs-dpp packages/rs-dpp
COPY --from=build-test-suite /platform/packages/wasm-dpp packages/wasm-dpp

COPY --from=build-test-suite /platform/packages/platform-test-suite/.env.example /platform/packages/platform-test-suite/.env

EXPOSE 2500 2501 2510
USER node
ENTRYPOINT ["/platform/packages/platform-test-suite/bin/test.sh"]

#
# STAGE: DAPI BUILD
#
FROM build-js AS build-dapi

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN yarn workspaces focus --production @dashevo/dapi

#
# STAGE: FINAL DAPI IMAGE
#
FROM node:18-alpine${ALPINE_VERSION} AS dapi

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk add --no-cache zeromq-dev

WORKDIR /platform

COPY --from=build-dapi /platform/.yarn /platform/.yarn
COPY --from=build-dapi /platform/package.json /platform/yarn.lock /platform/.yarnrc.yml /platform/.pnp* /platform/
# List of required dependencies. Based on:
# yarn run ultra --info --filter '@dashevo/dapi' |  sed -E 's/.*@dashevo\/(.*)/COPY --from=build-dapi \/platform\/packages\/\1 \/platform\/packages\/\1/'
COPY --from=build-dapi /platform/packages/dapi /platform/packages/dapi
COPY --from=build-dapi /platform/packages/dapi-grpc /platform/packages/dapi-grpc
COPY --from=build-dapi /platform/packages/js-dpp /platform/packages/js-dpp
COPY --from=build-dapi /platform/packages/js-grpc-common /platform/packages/js-grpc-common
COPY --from=build-dapi /platform/packages/wasm-dpp /platform/packages/wasm-dpp
COPY --from=build-dapi /platform/packages/js-dapi-client /platform/packages/js-dapi-client

RUN cp /platform/packages/dapi/.env.example /platform/packages/dapi/.env

EXPOSE 2500 2501 2510
USER node
