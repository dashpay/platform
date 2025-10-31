# syntax = docker/dockerfile:1.7-labs

# Docker image for rs-drive-abci
#
# This image is divided multiple parts:
# - deps-base - includes all base dependencies and some libraries
# - deps-sccache - deps image with sccache included
# - deps-compilation - deps image with all compilation dependencies - it's either deps-base or deps-sccache
# - deps-rocksdb - build static rocksdb library
# - deps - all deps, including wasm-bindgen-cli; built on top of either deps-base or deps-sccache
# - build-planner - image used to prepare build plan for rs-drive-abci
# - build-* - actual build process of given image
# - drive-abci, dashmate-helper, test-suite, rs-dapi - final images
#
# The following build arguments can be provided using --build-arg:
# - CARGO_BUILD_PROFILE - set to `release` to build final binary, without debugging information
# - NODE_ENV - node.js environment name to use to build the library
# - ALPINE_VERSION - use different version of Alpine base image; requires also rust:apline...
#   image to be available
# - USERNAME, USER_UID, USER_GID - specification of user used to run the binary
# - SDK_TEST_DATA - set to `true` to create SDK test data on chain genesis. It should be used only for testing
#   purpose in local development environment
#
# # sccache cache backends
#
# To enable sccache support and make the following variables available:
#    1. For S3 buckets:
#       - SCCACHE_BUCKET - S3 bucket name
#       - AWS_PROFILE
#       - SCCACHE_REGION
#       - SCCACHE_S3_KEY_PREFIX
#       - SCCACHE_ENDPOINT
#       - also, AWS credentials file ($HOME/.aws/credentials) should be provided as a secret file with id=AWS
#    2. For Github Actions:
#       - SCCACHE_GHA_ENABLED, ACTIONS_CACHE_URL
#       - also, Github Actions token should be provided as a secret file with id=GHA
#    3. For memcached:
#       - SCCACHE_MEMCACHED - set to memcache server URI (eg. tcp://172.17.0.1:11211) to enable sccache memcached backend

#
# BUILD PROCESS
#
# 1. All these --mount... are to cache reusable info between runs.
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
# 2. Github Actions have shared networking configured, so we need to set a random SCCACHE_SERVER_PORT port to avoid
# conflicts in case of parallel compilation.
# 3. Configuration variables are shared between runs using /root/env file.

ARG ALPINE_VERSION=3.21

# deps-${RUSTC_WRAPPER:-base}
# If one of SCCACHE_GHA_ENABLED, SCCACHE_BUCKET, SCCACHE_MEMCACHED is set, then deps-sccache is used, otherwise deps-base
ARG SCCACHE_GHA_ENABLED
ARG SCCACHE_BUCKET
ARG SCCACHE_MEMCACHED

# Determine if we have sccache enabled; if yes, use deps-sccache, otherwise use deps-base as a dependency image
ARG DEPS_IMAGE=${SCCACHE_GHA_ENABLED}${SCCACHE_BUCKET}${SCCACHE_MEMCACHED}
ARG DEPS_IMAGE=${DEPS_IMAGE:+sccache}
ARG DEPS_IMAGE=deps-${DEPS_IMAGE:-base}

#
# DEPS: INSTALL AND CACHE DEPENDENCIES
#
FROM node:20-alpine${ALPINE_VERSION} AS deps-base

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
    curl \
    git \
    libc-dev \
    linux-headers \
    llvm-static llvm-dev  \
    openssl-dev \
    snappy-static snappy-dev \
    perl \
    python3 \
    unzip \
    wget \
    xz \
    zeromq-dev

# Configure snappy, dependency of librocksdb-sys
RUN <<EOS
echo "export SNAPPY_STATIC=/usr/lib/libsnappy.a" >> /root/env
echo "export SNAPPY_LIB_DIR=/usr/lib" >> /root/env
echo "export SNAPPY_INCLUDE_DIR=/usr/include" >> /root/env
EOS

# Configure Node.js

RUN npm config set --global audit false

# Install latest Rust toolbox

ARG TARGETARCH

WORKDIR /platform


COPY rust-toolchain.toml .
RUN TOOLCHAIN_VERSION="$(grep channel rust-toolchain.toml | awk '{print $3}' | tr -d '"')" && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    --profile minimal \
    -y \
    --default-toolchain "${TOOLCHAIN_VERSION}" \
    --target wasm32-unknown-unknown

ONBUILD ENV HOME=/root
ONBUILD ENV CARGO_HOME=$HOME/.cargo

ONBUILD ARG CARGO_BUILD_PROFILE=dev

# Configure Rust toolchain and C / C++ compiler
RUN <<EOS
# It doesn't sharing PATH between stages, so we need "source $HOME/.cargo/env" everywhere
echo 'source $HOME/.cargo/env' >> /root/env

# Enable gcc / g++ optimizations
if [[ "$TARGETARCH" == "amd64" ]] ; then
    if [[ "${CARGO_BUILD_PROFILE}" == "release" ]] ; then
        echo "export CFLAGS=-march=x86-64-v3" >> /root/env
        echo "export CXXFLAGS=-march=x86-64-v3" >> /root/env
        echo "export PORTABLE=x86-64-v3" >> /root/env
    else
        echo "export CFLAGS=-march=x86-64" >> /root/env
        echo "export CXXFLAGS=-march=x86-64" >> /root/env
        echo "export PORTABLE=x86-64" >> /root/env
    fi
else
    echo "export PORTABLE=1" >> /root/env
fi
EOS

# Install protoc - protobuf compiler (pin to 32.0)
# The one shipped with Alpine does not work
ARG PROTOC_VERSION=32.0
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip \
    -o /tmp/protoc.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

# Switch to clang
# Note that CC / CXX can be updated later on (eg. when configuring sccache)
RUN rm /usr/bin/cc && \
    ln -s /usr/bin/clang /usr/bin/cc
RUN <<EOS
echo "export CXX='clang++'" >> /root/env
echo "export CC='clang'" >> /root/env
EOS

ARG NODE_ENV=production
ENV NODE_ENV=${NODE_ENV}

#
# DEPS-SCCACHE stage
#
# This stage is used to install sccache and configure it.
# Later on, one should source /root/env before building to use sccache.
#
# Note that, due to security concerns, each stage needs to declare variables containing authentication secrets, like
# ACTIONS_RUNTIME_TOKEN, AWS_SECRET_ACCESS_KEY. This is to prevent leaking secrets to the final image. The secrets are
# loaded using docker buildx `--secret` flag and need to be explicitly mounted with `--mount=type=secret,id=SECRET_ID`.

FROM deps-base AS deps-sccache

# SCCACHE_VERSION must be the same as in github actions, to avoid cache incompatibility
ARG SCCHACHE_VERSION=0.8.2

# Install sccache for caching
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export SCC_ARCH=aarch64; else export SCC_ARCH=x86_64; fi; \
    curl -Ls \
    https://github.com/mozilla/sccache/releases/download/v$SCCHACHE_VERSION/sccache-v$SCCHACHE_VERSION-${SCC_ARCH}-unknown-linux-musl.tar.gz | \
    tar -C /tmp -xz && \
    mv /tmp/sccache-*/sccache /usr/bin/

#
# Configure sccache
#

# Set args below to use Github Actions cache; see https://github.com/mozilla/sccache/blob/main/docs/GHA.md
ARG SCCACHE_GHA_ENABLED
ARG ACTIONS_CACHE_URL

# Alternative solution is to use memcache
ARG SCCACHE_MEMCACHED

# S3 storage
ARG SCCACHE_BUCKET
ARG AWS_PROFILE
ARG SCCACHE_REGION
ARG SCCACHE_S3_KEY_PREFIX
ARG SCCACHE_ENDPOINT

# Generate sccache configuration variables and save them to /root/env
#
# We only enable one cache at a time. Setting env variables belonging to multiple cache backends may fail the build.
RUN --mount=type=secret,id=AWS <<EOS
    set -ex -o pipefail

    ### Github Actions ###
    if [ -n "${SCCACHE_GHA_ENABLED}" ]; then
        echo "export SCCACHE_GHA_ENABLED=${SCCACHE_GHA_ENABLED}" >> /root/env
        echo "export ACTIONS_CACHE_URL=${ACTIONS_CACHE_URL}" >> /root/env
        # ACTIONS_RUNTIME_TOKEN is a secret so we quote it here, and it will be loaded when `source /root/env` is run
        echo 'export ACTIONS_RUNTIME_TOKEN="$(cat /run/secrets/GHA)"' >> /root/env

    ### AWS S3 ###
    elif [ -n "${SCCACHE_BUCKET}" ]; then
        echo "export SCCACHE_BUCKET='${SCCACHE_BUCKET}'" >> /root/env
        echo "export SCCACHE_REGION='${SCCACHE_REGION}'" >> /root/env
        [ -n "${AWS_PROFILE}" ] && echo "export AWS_PROFILE='${AWS_PROFILE}'" >> /root/env
        echo "export SCCACHE_ENDPOINT='${SCCACHE_ENDPOINT}'" >> /root/env
        echo "export SCCACHE_S3_KEY_PREFIX='${SCCACHE_S3_KEY_PREFIX}'" >> /root/env

        # Configure AWS credentials
        mkdir --mode=0700 -p "$HOME/.aws"
        ln -s /run/secrets/AWS "$HOME/.aws/credentials"
        echo "export AWS_SHARED_CREDENTIALS_FILE=$HOME/.aws/credentials" >> /root/env

        # Check if AWS credentials file is mounted correctly, eg. --mount=type=secret,id=AWS
        echo '[ -e "${AWS_SHARED_CREDENTIALS_FILE}" ] || {
            echo "$(id -u): Cannot read ${AWS_SHARED_CREDENTIALS_FILE}; did you use RUN --mount=type=secret,id=AWS ?";
            exit 1;
        }' >> /root/env

    ### memcached ###
    elif [ -n "${SCCACHE_MEMCACHED}" ]; then
        # memcached
        echo "export SCCACHE_MEMCACHED='${SCCACHE_MEMCACHED}'" >> /root/env
    else
        echo "Error: cannot determine sccache cache backend" >&2
        exit 1
    fi

    echo "export SCCACHE_SERVER_PORT=$((RANDOM+1025))" >> /root/env

    # Configure compilers to use sccache
    echo "export CXX='sccache clang++'" >> /root/env
    echo "export CC='sccache clang'" >> /root/env
    echo "export RUSTC_WRAPPER=sccache" >> /root/env
    # Disable Rust incremental builds, not supported by sccache
    echo 'export CARGO_INCREMENTAL=0' >> /root/env

    # for debugging, we display what we generated
    cat /root/env
EOS

# Image containing compolation dependencies; used to overcome lack of interpolation in COPY --from
FROM ${DEPS_IMAGE} AS deps-compilation
# Stage intentionally left empty

#
# BUILD ROCKSDB STATIC LIBRARY
#

FROM deps-compilation AS deps-rocksdb

RUN mkdir -p /tmp/rocksdb
WORKDIR /tmp/rocksdb


# RUN --mount=type=secret,id=AWS <<EOS
# echo Testing sccache configuration

# source /root/env
# sccache --start-server

# # Build some test project to check if sccache is working
# mkdir /tmp/sccache-test
# cd /tmp/sccache-test
# echo 'int main() { return 0; }' > a.c
# sccache clang -o a.o -c a.c
# cd -

# sccache -s
# EOS

# Select whether we want dev or release
# This variable will be also visibe in next stages
ONBUILD ARG CARGO_BUILD_PROFILE=dev

RUN --mount=type=secret,id=AWS <<EOS
set -ex -o pipefail
git clone https://github.com/facebook/rocksdb.git -b v10.4.2 --depth 1 .
source /root/env

make -j$(nproc) static_lib

mkdir -p /opt/rocksdb/usr/local/lib
cp librocksdb.a /opt/rocksdb/usr/local/lib/
cp -r include /opt/rocksdb/usr/local/
cd /
rm -rf /tmp/rocksdb
if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi
EOS

# Configure RocksDB env variables
RUN <<EOS
echo "export ROCKSDB_STATIC=/opt/rocksdb/usr/local/lib/librocksdb.a" >> /root/env
echo "export ROCKSDB_LIB_DIR=/opt/rocksdb/usr/local/lib" >> /root/env
echo "export ROCKSDB_INCLUDE_DIR=/opt/rocksdb/usr/local/include" >> /root/env
EOS


#
# DEPS: FULL DEPENDENCIES LIST
#
FROM deps-rocksdb AS deps


WORKDIR /platform

# Download and install cargo-binstall
ENV BINSTALL_VERSION=1.10.11
RUN --mount=type=secret,id=AWS \
    set -ex; \
    source /root/env; \
    if [ "$TARGETARCH" = "amd64" ]; then \
    CARGO_BINSTALL_ARCH="x86_64-unknown-linux-musl"; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
    CARGO_BINSTALL_ARCH="aarch64-unknown-linux-musl"; \
    else \
    echo "Unsupported architecture: $TARGETARCH"; exit 1; \
    fi; \
    # Construct download URL
    DOWNLOAD_URL="https://github.com/cargo-bins/cargo-binstall/releases/download/v${BINSTALL_VERSION}/cargo-binstall-${CARGO_BINSTALL_ARCH}.tgz"; \
    # Download and extract the cargo-binstall binary
    curl -A "Mozilla/5.0 (X11; Linux x86_64; rv:60.0) Gecko/20100101 Firefox/81.0" -L --proto '=https' --tlsv1.2 -sSf "$DOWNLOAD_URL" | tar -xvzf -;  \
    ./cargo-binstall -y --force cargo-binstall@${BINSTALL_VERSION}; \
    rm ./cargo-binstall; \
    cargo binstall -V

RUN --mount=type=secret,id=AWS \
    source /root/env; \
    cargo binstall wasm-bindgen-cli@0.2.103 cargo-chef@0.1.72 \
    --locked \
    --no-discover-github-token \
    --disable-telemetry \
    --no-track \
    --no-confirm

#
# Rust build planner to speed up builds
#
FROM deps AS build-planner

WORKDIR /platform

COPY --parents \
    Cargo.lock \
    Cargo.toml \
    rust-toolchain.toml \
    .cargo \
    packages/dapi-grpc \
    packages/rs-dapi-grpc-macros \
    packages/rs-dpp \
    packages/rs-drive \
    packages/rs-platform-value \
    packages/rs-platform-serialization \
    packages/rs-platform-serialization-derive \
    packages/rs-platform-version \
    packages/rs-platform-versioning \
    packages/rs-platform-value-convertible \
    packages/rs-drive-abci \
    packages/rs-dapi \
    packages/rs-dash-event-bus \
    packages/dashpay-contract \
    packages/withdrawals-contract \
    packages/masternode-reward-shares-contract \
    packages/feature-flags-contract \
    packages/dpns-contract \
    packages/wallet-utils-contract \
    packages/token-history-contract \
    packages/keyword-search-contract \
    packages/data-contracts \
    packages/strategy-tests \
    packages/simple-signer \
    packages/rs-json-schema-compatibility-validator \
    # TODO: We don't need those. Maybe dynamically remove them from workspace or move outside of monorepo?
    packages/rs-drive-proof-verifier \
    packages/rs-context-provider \
    packages/rs-sdk-trusted-context-provider \
    packages/rs-platform-wallet \
    packages/wasm-dpp \
    packages/wasm-drive-verify \
    packages/rs-dapi-client \
    packages/rs-sdk \
    packages/rs-sdk-ffi \
    packages/check-features \
    packages/dash-platform-balance-checker \
    packages/wasm-sdk \
    /platform/

RUN --mount=type=secret,id=AWS \
    source /root/env && \
    cargo chef prepare $RELEASE --recipe-path recipe.json

#
# STAGE: BUILD RS-DRIVE-ABCI
#
# This will prebuild majority of dependencies
FROM deps AS build-drive-abci

# Pass SDK_TEST_DATA=true to create SDK test data on chain genesis
# This is only for testing purpose and should be used only for
# local development environment
ARG SDK_TEST_DATA

SHELL ["/bin/bash", "-o", "pipefail","-e", "-x", "-c"]

WORKDIR /platform

COPY --from=build-planner --parents /platform/recipe.json /platform/.cargo /

# Build dependencies - this is the caching Docker layer!
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=secret,id=AWS \
    set -ex; \
    source /root/env && \
    if  [[ "${CARGO_BUILD_PROFILE}" == "release" ]] ; then \
    mv .cargo/config-release.toml .cargo/config.toml; \
    else \
    export FEATURES_FLAG="--features=console,grovedbg"; \
    fi && \
    if [ "${SDK_TEST_DATA}" == "true" ]; then \
    mv .cargo/config-test-sdk-data.toml .cargo/config.toml; \
    fi && \
    cargo chef cook \
    --recipe-path recipe.json \
    --profile "$CARGO_BUILD_PROFILE" \
    --package drive-abci \
    ${FEATURES_FLAG} \
    --locked && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi

COPY --parents \
    Cargo.lock \
    Cargo.toml \
    rust-toolchain.toml \
    .cargo \
    packages/dapi-grpc \
    packages/rs-dapi-grpc-macros \
    packages/rs-dapi \
    packages/rs-dash-event-bus \
    packages/rs-dpp \
    packages/rs-drive \
    packages/rs-platform-value \
    packages/rs-platform-serialization \
    packages/rs-platform-serialization-derive \
    packages/rs-platform-version \
    packages/rs-platform-versioning \
    packages/rs-platform-value-convertible \
    packages/rs-drive-abci \
    packages/dashpay-contract \
    packages/wallet-utils-contract \
    packages/token-history-contract \
    packages/keyword-search-contract \
    packages/withdrawals-contract \
    packages/masternode-reward-shares-contract \
    packages/feature-flags-contract \
    packages/dpns-contract \
    packages/data-contracts \
    packages/strategy-tests \
    # These packages are part of workspace and must be here otherwise it builds from scratch
    # See todo below
    packages/simple-signer \
    packages/rs-json-schema-compatibility-validator \
    # TODO: We don't need those. Maybe dynamically remove them from workspace or move outside of monorepo?
    packages/rs-drive-proof-verifier \
    packages/rs-context-provider \
    packages/rs-sdk-trusted-context-provider \
    packages/rs-platform-wallet \
    packages/wasm-dpp \
    packages/wasm-drive-verify \
    packages/rs-dapi-client \
    packages/rs-sdk \
    packages/rs-sdk-ffi \
    packages/check-features \
    packages/dash-platform-balance-checker \
    packages/wasm-sdk \
    /platform/

RUN mkdir /artifacts

# Build Drive ABCI
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=secret,id=AWS \
    set -ex; \
    source /root/env && \
    if  [[ "${CARGO_BUILD_PROFILE}" == "release" ]] ; then \
    mv .cargo/config-release.toml .cargo/config.toml; \
    export OUT_DIRECTORY=release; \
    else \
    export FEATURES_FLAG="--features=console,grovedbg"; \
    export OUT_DIRECTORY=debug; \
    fi && \
    if [ "${SDK_TEST_DATA}" == "true" ]; then \
    mv .cargo/config-test-sdk-data.toml .cargo/config.toml; \
    fi && \
    # Workaround: as we cache dapi-grpc, its build.rs is not rerun, so we need to touch it
    echo "// $(date) " >> /platform/packages/dapi-grpc/build.rs && \
    cargo build \
    --profile "${CARGO_BUILD_PROFILE}" \
    --package drive-abci \
    ${FEATURES_FLAG} \
    --locked && \
    cp target/${OUT_DIRECTORY}/drive-abci /artifacts/ && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi && \
    # Remove /platform to reduce layer size
    rm -rf /platform


#
# STAGE: BUILD JAVASCRIPT INTERMEDIATE IMAGE
#
FROM deps AS build-js

WORKDIR /platform

COPY --from=build-planner /platform/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
# Note we unset CFLAGS and CXXFLAGS as they have `-march` included, which breaks wasm32 build
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=secret,id=AWS \
    source /root/env && \
    unset CFLAGS CXXFLAGS && \
    cargo chef cook \
    --recipe-path recipe.json \
    --profile "$CARGO_BUILD_PROFILE" \
    --package wasm-dpp \
    --target wasm32-unknown-unknown \
    --locked && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi


# Rust deps
COPY --parents \
    Cargo.lock \
    Cargo.toml \
    rust-toolchain.toml \
    .cargo \
    packages/rs-dapi \
    packages/rs-dash-event-bus \
    packages/rs-dpp \
    packages/rs-platform-value \
    packages/rs-platform-serialization \
    packages/rs-platform-serialization-derive \
    packages/rs-platform-version \
    packages/rs-platform-versioning \
    packages/rs-platform-value-convertible \
    packages/rs-json-schema-compatibility-validator \
    # Common
    packages/wasm-dpp \
    packages/dashpay-contract \
    packages/withdrawals-contract \
    packages/wallet-utils-contract \
    packages/token-history-contract \
    packages/keyword-search-contract \
    packages/masternode-reward-shares-contract \
    packages/feature-flags-contract \
    packages/dpns-contract \
    packages/data-contracts \
    packages/dapi-grpc \
    # JS deps
    .yarn \
    .pnp* \
    .yarnrc.yml \
    yarn.lock \
    package.json \
    packages/js-grpc-common \
    packages/js-dapi-client \
    packages/wallet-lib \
    packages/js-dash-sdk \
    packages/dash-spv \
    /platform/

# We unset CFLAGS CXXFLAGS because they hold `march` flags which break wasm32 build
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=unplugged_${TARGETARCH},target=/tmp/unplugged \
    --mount=type=secret,id=AWS \
    source /root/env && \
    unset CFLAGS CXXFLAGS && \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn install --inline-builds && \
    cp -R /platform/.yarn/unplugged /tmp/ && \
    export SKIP_GRPC_PROTO_BUILD=1 && \
    yarn build && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi && \
    # Remove target directory and rust packages to save space
    rm -rf target packages/rs-*

#
# STAGE: FINAL DRIVE-ABCI IMAGE
#
FROM alpine:${ALPINE_VERSION} AS drive-abci

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive ABCI Rust"

RUN apk add --no-cache libgcc libstdc++

ENV DB_PATH=/var/lib/dash/rs-drive-abci/db
ENV REJECTIONS_PATH=/var/log/dash/rejected

RUN mkdir -p /var/log/dash \
    /var/lib/dash/rs-drive-abci/db \
    ${REJECTIONS_PATH}

COPY --from=build-drive-abci /artifacts/drive-abci /usr/bin/drive-abci
COPY packages/rs-drive-abci/.env.mainnet /var/lib/dash/rs-drive-abci/.env

# Create a volume
VOLUME /var/lib/dash/rs-drive-abci/db
VOLUME /var/log/dash

# Double-check that we don't have missing deps
RUN ldd /usr/bin/drive-abci

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
EXPOSE 26659
# Prometheus port
EXPOSE 29090

#
# STAGE: DASHMATE HELPER BUILD
#
FROM build-js AS build-dashmate-helper

# Copy dashmate package
COPY packages/dashmate packages/dashmate

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN yarn workspaces focus --production dashmate

#
#  STAGE: FINAL DASHMATE HELPER IMAGE
#
FROM node:20-alpine${ALPINE_VERSION} AS dashmate-helper

RUN apk add --no-cache docker-cli docker-cli-compose curl

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dashmate Helper Node.JS"

WORKDIR /platform

# TODO: Do one COPY with --parents
COPY --from=build-dashmate-helper /platform/.yarn /platform/.yarn
COPY --from=build-dashmate-helper /platform/package.json /platform/yarn.lock /platform/.yarnrc.yml /platform/.pnp* /platform/

# Copy only necessary packages from monorepo
COPY --from=build-dashmate-helper /platform/packages/dashmate packages/dashmate
COPY --from=build-dashmate-helper /platform/packages/dashpay-contract packages/dashpay-contract
COPY --from=build-dashmate-helper /platform/packages/wallet-lib packages/wallet-lib
COPY --from=build-dashmate-helper /platform/packages/js-dapi-client packages/js-dapi-client
COPY --from=build-dashmate-helper /platform/packages/js-grpc-common packages/js-grpc-common
COPY --from=build-dashmate-helper /platform/packages/dapi-grpc packages/dapi-grpc
COPY --from=build-dashmate-helper /platform/packages/dash-spv packages/dash-spv
COPY --from=build-dashmate-helper /platform/packages/wallet-utils-contract packages/wallet-utils-contract
COPY --from=build-dashmate-helper /platform/packages/token-history-contract packages/token-history-contract
COPY --from=build-dashmate-helper /platform/packages/keyword-search-contract packages/keyword-search-contract
COPY --from=build-dashmate-helper /platform/packages/withdrawals-contract packages/withdrawals-contract
COPY --from=build-dashmate-helper /platform/packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY --from=build-dashmate-helper /platform/packages/feature-flags-contract packages/feature-flags-contract
COPY --from=build-dashmate-helper /platform/packages/dpns-contract packages/dpns-contract
COPY --from=build-dashmate-helper /platform/packages/data-contracts packages/data-contracts
COPY --from=build-dashmate-helper /platform/packages/wasm-dpp packages/wasm-dpp

ENV DASHMATE_HOME_DIR=/home/dashmate/.dashmate
ENV DASHMATE_HELPER=1

ENTRYPOINT ["/platform/packages/dashmate/docker/entrypoint.sh"]

#
# STAGE: TEST SUITE BUILD
#
FROM build-js AS build-test-suite

COPY packages/platform-test-suite packages/platform-test-suite

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN yarn workspaces focus --production @dashevo/platform-test-suite

#
#  STAGE: FINAL TEST SUITE IMAGE
#
FROM node:20-alpine${ALPINE_VERSION} AS test-suite

RUN apk add --no-cache bash

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dash Platform test suite"

WORKDIR /platform

COPY --from=build-test-suite /platform /platform
COPY --from=build-test-suite /platform/packages/platform-test-suite/.env.example /platform/packages/platform-test-suite/.env

EXPOSE 2500 2501 2510
USER node
ENTRYPOINT ["/platform/packages/platform-test-suite/bin/test.sh"]

EXPOSE 2500 2501 2510
USER node

#
# STAGE: BUILD RS-DAPI
#
FROM deps AS build-rs-dapi

SHELL ["/bin/bash", "-o", "pipefail","-e", "-x", "-c"]

WORKDIR /platform

COPY --from=build-planner --parents /platform/recipe.json /platform/.cargo /

# Build dependencies - this is the caching Docker layer!
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=secret,id=AWS \
    set -ex; \
    source /root/env && \
    if  [[ "${CARGO_BUILD_PROFILE}" == "release" ]] ; then \
    mv .cargo/config-release.toml .cargo/config.toml; \
    fi && \
    cargo chef cook \
    --recipe-path recipe.json \
    --profile "$CARGO_BUILD_PROFILE" \
    --package rs-dapi \
    --locked && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi

COPY --parents \
    Cargo.lock \
    Cargo.toml \
    rust-toolchain.toml \
    .cargo \
    packages/dapi-grpc \
    packages/rs-dapi-grpc-macros \
    packages/rs-dpp \
    packages/rs-drive \
    packages/rs-platform-value \
    packages/rs-platform-serialization \
    packages/rs-platform-serialization-derive \
    packages/rs-platform-version \
    packages/rs-platform-versioning \
    packages/rs-platform-value-convertible \
    packages/rs-drive-abci \
    packages/rs-dapi \
    packages/rs-dash-event-bus \
    packages/dashpay-contract \
    packages/wallet-utils-contract \
    packages/token-history-contract \
    packages/keyword-search-contract \
    packages/withdrawals-contract \
    packages/masternode-reward-shares-contract \
    packages/feature-flags-contract \
    packages/dpns-contract \
    packages/data-contracts \
    packages/strategy-tests \
    # These packages are part of workspace and must be here otherwise it builds from scratch
    packages/simple-signer \
    packages/rs-json-schema-compatibility-validator \
    packages/rs-drive-proof-verifier \
    packages/rs-context-provider \
    packages/rs-sdk-trusted-context-provider \
    packages/wasm-dpp \
    packages/wasm-drive-verify \
    packages/rs-dapi-client \
    packages/rs-sdk \
    packages/rs-sdk-ffi \
    packages/rs-platform-wallet \
    packages/check-features \
    packages/dash-platform-balance-checker \
    packages/wasm-sdk \
    /platform/

RUN mkdir /artifacts

# Build rs-dapi
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=secret,id=AWS \
    set -ex; \
    source /root/env && \
    if  [[ "${CARGO_BUILD_PROFILE}" == "release" ]] ; then \
    mv .cargo/config-release.toml .cargo/config.toml; \
    export OUT_DIRECTORY=release; \
    else \
    export OUT_DIRECTORY=debug; \
    fi && \
    # Workaround: as we cache dapi-grpc, its build.rs is not rerun, so we need to touch it
    echo "// $(date) " >> /platform/packages/dapi-grpc/build.rs && \
    cargo build \
    --profile "${CARGO_BUILD_PROFILE}" \
    --package rs-dapi \
    --locked && \
    cp target/${OUT_DIRECTORY}/rs-dapi /artifacts/ && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi && \
    # Remove /platform to reduce layer size
    rm -rf /platform

#
# STAGE: RS-DAPI RUNTIME
#
FROM alpine:${ALPINE_VERSION} AS rs-dapi

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dash Platform API (DAPI) - Rust Implementation"

RUN apk add --no-cache libgcc libstdc++

ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

COPY --from=build-rs-dapi /artifacts/rs-dapi /usr/bin/rs-dapi

# Create example .env file
RUN mkdir -p /app
COPY packages/rs-dapi/.env.example /app/.env

# Double-check that we don't have missing deps
RUN ldd /usr/bin/rs-dapi

#
# Create new non-root user
#
ARG USERNAME=dapi
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN addgroup -g $USER_GID $USERNAME && \
    adduser -D -u $USER_UID -G $USERNAME -h /app $USERNAME && \
    mkdir -p /var/log/rs-dapi && \
    chown -R $USER_UID:$USER_GID /app /var/log/rs-dapi

USER $USERNAME

WORKDIR /app
ENTRYPOINT ["/usr/bin/rs-dapi", "start"]

# Default gRPC port
EXPOSE 3010
# Optional HTTP/REST port (if implemented)
EXPOSE 3000
