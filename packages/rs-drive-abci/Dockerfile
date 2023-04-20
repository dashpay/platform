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
ARG ALPINE_VERSION=3.17

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
        clang-static clang-dev \
        llvm-static llvm-dev  \
        git \
        linux-headers \
        openssl-dev \
        perl \
        unzip \
        cmake \
        python3 \
        wget \
        xz

SHELL ["/bin/bash", "-c"]

ARG TARGETARCH

RUN rustup install stable

# Install protoc - protobuf compiler
# The one shipped with Alpine does not work
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls https://github.com/protocolbuffers/protobuf/releases/download/v22.2/protoc-22.2-linux-${PROTOC_ARCH}.zip \
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

RUN curl -Ls \
        https://github.com/emscripten-core/emsdk/archive/refs/tags/3.1.36.tar.gz | \
        tar -C /opt -xz && \
        ln -s /opt/emsdk-* /opt/emsdk && \
        /opt/emsdk/emsdk install latest && \
        /opt/emsdk/emsdk activate latest

#
# EXECUTE BUILD
#
FROM deps as build

WORKDIR /usr/src/dash-platform

COPY . .

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
ENV CARGO_INCREMENTAL=false

# Select whether we want dev or release
ARG CARGO_BUILD_PROFILE=dev

# Run the build, with extensive caching.
# 
# Note:
# 1. All these --mount... are to cache reusable info between runs.
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
# 2. We add `--config net.git-fetch-with-cli=true` to address ARM build issue,
# see https://github.com/rust-lang/cargo/issues/10781#issuecomment-1441071052
# 3. To preserve space on github cache, we call `cargo clean`.
# 4. Github Actions have shared networking configured, so we need to set a random
# SCCACHE_SERVER_PORT port to avoid conflicts in case of parallel compilation
# 5. We also set RUSTC to include exact toolchain name in compilation command, and
# include this in cache key
# 6. We build `dashcore-rpc` first, as arm64 builder on github hangs if we don't do this.
RUN rm /usr/bin/cc && ln -s /usr/bin/clang /usr/bin/cc

RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    export SCCACHE_SERVER_PORT=$((RANDOM+1025)) && \
    cargo build \
        --package dashcore-rpc \
       --config net.git-fetch-with-cli=true && \
    cargo build \
        --package drive-abci \
        --config net.git-fetch-with-cli=true && \
    cp /usr/src/dash-platform/target/*/drive-abci /usr/src/dash-platform/drive-abci && \
    cargo clean && \
    sccache --show-stats

#
# FINAL IMAGE
#
FROM alpine:${ALPINE_VERSION} AS release

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive ABCI Rust"


#
# Install binaries and data
#
WORKDIR /var/lib/dash

RUN apk add --no-cache libgcc libstdc++

COPY --from=build /usr/src/dash-platform/drive-abci /usr/bin/drive-abci
COPY --from=build /usr/src/dash-platform/packages/rs-drive-abci/.env.example /var/lib/dash/rs-drive-abci/.env

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
ENV VALIDATOR_SET_QUORUM_ROTATION_BLOCK_COUNT=1024

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

WORKDIR /var/lib/dash/rs-drive-abci
ENTRYPOINT ["/usr/bin/drive-abci"]
CMD ["-vv", "start"]

EXPOSE 26658
