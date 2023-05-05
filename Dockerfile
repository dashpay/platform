# syntax = docker/dockerfile:1.5

# Docker image for rs-drive-abci
#
# This image is divided into 3 stages, for better layer caching:
# - deps - includes all dependencies and some libraries
# - build - actual build process
# - release - final image
#
# The following build arguments can be provided using --build-arg:
# - CARGO_BUILD_PROFILE - set to `release` to build final binary, without debugging information
# - SCCACHE_GHA_ENABLED, ACTIONS_CACHE_URL, ACTIONS_RUNTIME_TOKEN - store sccache caches inside github actions
#   cache instead of Docker cache mounts (not tested yet)
# - PROTOC_ARCH - select architecture of protobuf compiler; one of: `x86_64` (default), `aarch_64`
# - ALPINE_VERSION - use different version of Alpine base image; requires also rust:apline... 
#   image to be available
# - USERNAME, USER_UID, USER_GID - specification of user used to run the binary
#
ARG ALPINE_VERSION=3.16

#
# DEPS: INSTALL AND CACHE DEPENDENCIES
#
FROM rust:alpine${ALPINE_VERSION} as deps

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
        nodejs \
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

# Install sccache for caching
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export SCC_ARCH=aarch64; else export SCC_ARCH=x86_64; fi; \
    curl -Ls \
        https://github.com/mozilla/sccache/releases/download/v0.4.1/sccache-v0.4.1-${SCC_ARCH}-unknown-linux-musl.tar.gz | \
        tar -C /tmp -xz && \
        mv /tmp/sccache-*/sccache /usr/bin/

# Install emcmake, dependency of bls-signatures -> bls-dash-sys
# TODO: Build ARM image to check if this is still needed
# RUN curl -Ls \
#         https://github.com/emscripten-core/emsdk/archive/refs/tags/3.1.36.tar.gz | \
#         tar -C /opt -xz && \
#         ln -s /opt/emsdk-* /opt/emsdk && \
#         /opt/emsdk/emsdk install latest && \
#         /opt/emsdk/emsdk activate latest

# Configure Node.js
RUN npm install -g npm@latest && \
    npm install -g corepack@latest && \
    corepack prepare yarn@stable --activate && \
    corepack enable

# TODO: Move above, where we call rustup


# Switch to clang
RUN rm /usr/bin/cc && ln -s /usr/bin/clang /usr/bin/cc

#
# Configure sccache
#
# Activate sccache for Rust code
ENV RUSTC_WRAPPER=/usr/bin/sccache
# Set args below to use Github Actions cache; see https://github.com/mozilla/sccache/blob/main/docs/GHA.md
ARG SCCACHE_GHA_ENABLED
ARG ACTIONS_CACHE_URL
ARG ACTIONS_RUNTIME_TOKEN
# Alternative solution is to use memcache
ARG SCCACHE_MEMCACHED

# Disable incremental buildings, not supported by sccache
ARG CARGO_INCREMENTAL=false

# Select whether we want dev or release
ARG CARGO_BUILD_PROFILE=debug
ENV CARGO_BUILD_PROFILE ${CARGO_BUILD_PROFILE}

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

# Install wasm-bindgen-cli
WORKDIR /platform
RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,target=/platform/target \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    cargo install wasm-bindgen-cli

#
# EXECUTE BUILD
#
FROM deps as sources


# We run builds with extensive caching.
# 
# Note:
# 1. All these --mount... are to cache reusable info between runs.
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
# 2. We add `--config net.git-fetch-with-cli=true` to address ARM build issue,
# see https://github.com/rust-lang/cargo/issues/10781#issuecomment-1441071052
# 3. Github Actions have shared networking configured, so we need to set a random
# SCCACHE_SERVER_PORT port to avoid conflicts in case of parallel compilation
# 4. We also set RUSTC to include exact toolchain name in compilation command, and
# include this in cache key

WORKDIR /platform

COPY . .

RUN yarn config set enableInlineBuilds true

#
# STAGE: BUILD RS-DRIVE-ABCI
#
# This will prebuild majority of dependencies
FROM sources AS build-drive-abci

RUN mkdir /artifacts

RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,target=/platform/target \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    cargo build -p drive-abci \
       --config net.git-fetch-with-cli=true && \
    cp /platform/target/*/drive-abci /artifacts/drive-abci && \
    sccache --show-stats && \
    du -sh /platform/target/*/*

#     yarn workspace @dashevo/wasm-dpp build && \

#
# STAGE: BUILD WASM-DPP
#
FROM sources AS build-wasm-dpp

RUN mkdir /artifacts

RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=wasm_dpp_target,target=/platform/target \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    if [[ -z "${SCCACHE_MEMCACHED}" ]] ; then unset SCCACHE_MEMCACHED ; fi ; \
    yarn workspace @dashevo/wasm-dpp build && \
    sccache --show-stats && \
    du -sh /platform/target/*/*

#     yarn workspace @dashevo/wasm-dpp build && \

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

#
# CONFIGURATION 
# Hint: generated with:
# ``
#   sed -E 's/^([A-Z].+$)/ENV \1/g' packages/rs-drive-abci/.env.example 
# ``
#

# ABCI host and port to listen
ENV ABCI_BIND_ADDRESS="tcp://0.0.0.0:26658"

ENV DB_PATH=/tmp/db

# GroveDB database file
ENV GROVEDB_LATEST_FILE=${DB_PATH}/latest_state

# Cache size for Data Contracts
ENV DATA_CONTRACTS_GLOBAL_CACHE_SIZE=500
ENV DATA_CONTRACTS_BLOCK_CACHE_SIZE=200

# DashCore JSON-RPC host, port and credentials
# Read more: https://dashcore.readme.io/docs/core-api-ref-remote-procedure-calls
ENV CORE_JSON_RPC_HOST=127.0.0.1
ENV CORE_JSON_RPC_PORT=9998
ENV CORE_JSON_RPC_USERNAME=dashrpc
ENV CORE_JSON_RPC_PASSWORD=password

# DashCore ZMQ host and port
ENV CORE_ZMQ_HOST=127.0.0.1
ENV CORE_ZMQ_PORT=29998
ENV CORE_ZMQ_CONNECTION_RETRIES=16

ENV NETWORK=testnet

ENV INITIAL_CORE_CHAINLOCKED_HEIGHT=1243

# https://github.com/dashevo/dashcore-lib/blob/286c33a9d29d33f05d874c47a9b33764a0be0cf1/lib/constants/index.js#L42-L57
ENV VALIDATOR_SET_LLMQ_TYPE=100
ENV VALIDATOR_SET_QUORUM_ROTATION_BLOCK_COUNT=64

ENV DKG_INTERVAL=24
ENV MIN_QUORUM_VALID_MEMBERS=3

# DPNS Contract

ENV DPNS_MASTER_PUBLIC_KEY=02649a81b760e8635dd3a4fad8911388ed09d7c1680558a890180d4edc8bcece7e
ENV DPNS_SECOND_PUBLIC_KEY=03f5ea3ab4bf594c28997eb8f83873532275ac2edd36e586b137ed42d15d510948

# Dashpay Contract

ENV DASHPAY_MASTER_PUBLIC_KEY=022d6d70c9d24d03904713db17fb74c9201801ba0e3aed0f5d91e89df388e94aa6
ENV DASHPAY_SECOND_PUBLIC_KEY=028c0a26c87b2e7f1aebbbeace9e687d774e037f5b50a6905b5f6fa24495b502cd

# Feature flags contract

ENV FEATURE_FLAGS_MASTER_PUBLIC_KEY=034ee04c509083ecd09e76fa53e0b5331b39120c19607cd04c4f167707dbb42302
ENV FEATURE_FLAGS_SECOND_PUBLIC_KEY=03c755ae1b79dbcc79020aad3ccdfcb142fc6e74f1afc220fca1e275a87aa12cf8

# Masternode reward shares contract

ENV MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY=02099cc210c7b6c7f566099046ddc92615342db326184940bf3811026ea328c85e
ENV MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY=02bf55f97f189895da29824781053140ee66b2bf47760246504fbe502985096af5

# Withdrawals contract

ENV WITHDRAWALS_MASTER_PUBLIC_KEY=027057cdf58628635ef7b75e6b6c90dd996a16929cd68130e16b9328d429e5e03a
ENV WITHDRAWALS_SECOND_PUBLIC_KEY=022084d827fea4823a69aa7c8d3e02fe780eaa0ef1e5e9841af395ba7e40465ab6

ENV TENDERDASH_P2P_PORT=26656

ENV QUORUM_SIZE=5
ENV QUORUM_TYPE=llmq_25_67
ENV CHAIN_ID=devnet
ENV BLOCK_SPACING_MS=3000

#
# END OF CONFIGURATION
#

# Create a volume
VOLUME /var/lib/dash

ENV DB_PATH=/var/lib/dash/rs-drive-abci/db

#
# Create new non-root user
#
ARG USERNAME=dash
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN addgroup -g $USER_GID $USERNAME && \
    adduser -D -u $USER_UID -G $USERNAME -h /var/lib/dash/rs-drive-abci $USERNAME && \
    chown -R $USER_UID:$USER_GID /var/lib/dash/rs-drive-abci 

USER $USERNAME

ENV RUST_BACKTRACE=1
WORKDIR /var/lib/dash/rs-drive-abci
ENTRYPOINT ["/usr/bin/drive-abci"]
CMD ["-vvvv", "start"]

EXPOSE 26658

#
# STAGE: DASHMATE BUILD
#
FROM build-wasm-dpp AS build-dashmate

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production dashmate && \
    cp -R /platform/.yarn/unplugged /tmp/

# Remove Rust sources
RUN find ./packages -name Cargo.toml | xargs -n1 dirname | xargs -t rm -r

# TODO: Clean all other files not needed by dashmate

#
#  STAGE: FINAL DASHMATE IMAGE
#
FROM node:16-alpine${ALPINE_VERSION} AS dashmate

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache docker-cli docker-cli-compose curl

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dashmate Helper Node.JS"

WORKDIR /platform

COPY --from=build-dashmate /platform /platform

ENTRYPOINT ["/platform/packages/dashmate/docker/entrypoint.sh"]


#
# STAGE: TEST SUITE BUILD
#
FROM build-wasm-dpp AS build-testsuite

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production @dashevo/platform-test-suite && \
    cp -R /platform/.yarn/unplugged /tmp/

# Remove Rust sources
RUN find ./packages -name Cargo.toml | xargs -n1 dirname | xargs -t rm -r

# TODO: Clean all other files not needed by test suite

#
#  STAGE: FINAL TEST SUITE IMAGE
#
FROM node:16-alpine${ALPINE_VERSION} AS testsuite

RUN apk add --no-cache bash

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dash Platform test suite"

WORKDIR /platform

COPY --from=build-testsuite /platform /platform

RUN cp /platform/packages/platform-test-suite/.env.example /platform/packages/platform-test-suite/.env

EXPOSE 2500 2501 2510

ENTRYPOINT ["/platform/packages/platform-test-suite/bin/test.sh"]

#
# STAGE: DAPI BUILD
#
FROM build-wasm-dpp AS build-dapi

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production @dashevo/dapi && \
    cp -R /platform/.yarn/unplugged /tmp/

# Remove Rust sources
RUN find ./packages -name Cargo.toml | xargs -n1 dirname | xargs -t rm -r

# TODO: Clean all other files not needed by dapi

#
# STAGE: FINAL DAPI IMAGE
#
FROM node:16-alpine3.16 AS dapi

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

WORKDIR /platform

COPY --from=build-dapi /platform /platform

RUN cp /platform/packages/dapi/.env.example /platform/packages/dapi/.env

EXPOSE 2500 2501 2510
