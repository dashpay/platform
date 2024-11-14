# syntax = docker/dockerfile:1

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
# - drive-abci, dashmate-helper, test-suite, dapi - final images
#
# The following build arguments can be provided using --build-arg:
# - CARGO_BUILD_PROFILE - set to `release` to build final binary, without debugging information
# - NODE_ENV - node.js environment name to use to build the library
# - RUSTC_WRAPPER - set to `sccache` to enable sccache support and make the following variables available:
#   - SCCACHE_GHA_ENABLED, ACTIONS_CACHE_URL, ACTIONS_RUNTIME_TOKEN - store sccache caches inside github actions
#   - SCCACHE_BUCKET, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION, SCCACHE_S3_KEY_PREFIX - store caches in S3
#   - SCCACHE_MEMCACHED - set to memcache server URI (eg. tcp://172.17.0.1:11211) to enable sccache memcached backend
# - ALPINE_VERSION - use different version of Alpine base image; requires also rust:apline...
#   image to be available
# - USERNAME, USER_UID, USER_GID - specification of user used to run the binary
#
#
#
#
# BUILD PROCESS
#
# 1. All these --mount... are to cache reusable info between runs.
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
# 2. Github Actions have shared networking configured, so we need to set a random SCCACHE_SERVER_PORT port to avoid
# conflicts in case of parallel compilation.
# 3. Configuration variables are shared between runs using /root/env file.

ARG ALPINE_VERSION=3.18
ARG RUSTC_WRAPPER

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


# TODO: It doesn't sharing PATH between stages, so we need "source $HOME/.cargo/env" everywhere
COPY rust-toolchain.toml .
RUN TOOLCHAIN_VERSION="$(grep channel rust-toolchain.toml | awk '{print $3}' | tr -d '"')" && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
        --profile minimal \
        -y \
        --default-toolchain "${TOOLCHAIN_VERSION}" \
        --target wasm32-unknown-unknown

ONBUILD ENV HOME=/root
ONBUILD ENV CARGO_HOME=$HOME/.cargo

# Install protoc - protobuf compiler
# The one shipped with Alpine does not work
ARG PROTOC_VERSION=27.3
RUN if [[ "$TARGETARCH" == "arm64" ]] ; then export PROTOC_ARCH=aarch_64; else export PROTOC_ARCH=x86_64; fi; \
    curl -Ls https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip \
        -o /tmp/protoc.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

# Switch to clang
RUN rm /usr/bin/cc && ln -s /usr/bin/clang /usr/bin/cc

# Select whether we want dev or release
ONBUILD ARG CARGO_BUILD_PROFILE=dev

ARG NODE_ENV=production
ENV NODE_ENV=${NODE_ENV}

#
# DEPS-SCCACHE stage
#
# This stage is used to install sccache and configure it.
# Later on, one should source /root/env before building to use sccache.

# Note that, due to security concerns, each stage needs to declare variables containing authentication secrets, like
# ACTIONS_RUNTIME_TOKEN, AWS_SECRET_ACCESS_KEY. It is done using ONBUILD directive, so the secrets are not stored in the
# final image.

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
ARG RUSTC_WRAPPER

# Disable incremental builds, not supported by sccache
RUN echo 'export CARGO_INCREMENTAL=false' >> /root/env

# Set args below to use Github Actions cache; see https://github.com/mozilla/sccache/blob/main/docs/GHA.md
ARG SCCACHE_GHA_ENABLED
ARG ACTIONS_CACHE_URL

# Alternative solution is to use memcache
ARG SCCACHE_MEMCACHED

# S3 storage
ARG SCCACHE_BUCKET
ARG AWS_ACCESS_KEY_ID
ARG AWS_REGION
ARG SCCACHE_REGION
ARG SCCACHE_S3_KEY_PREFIX

# Generate sccache configuration variables and save them to /root/env
#
# We only enable one cache at a time. Setting env variables belonging to multiple cache backends may fail the build.
RUN <<EOS
    set -ex -o pipefail

    if [ -n "${SCCACHE_GHA_ENABLED}" ]; then
        # Github Actions cache
        echo "export SCCACHE_GHA_ENABLED=${SCCACHE_GHA_ENABLED}" >> /root/env
        echo "export ACTIONS_CACHE_URL=${ACTIONS_CACHE_URL}" >> /root/env
        # ACTIONS_RUNTIME_TOKEN is a secret so we load it using ONBUILD ARG later on
    elif [ -n "${SCCACHE_BUCKET}" ]; then
        # AWS S3
        if [ -z "${SCCACHE_REGION}" ] ; then
            # Default to AWS_REGION if not set
            export SCCACHE_REGION=${AWS_REGION}
        fi

        echo "export AWS_REGION='${AWS_REGION}'" >> /root/env
        echo "export SCCACHE_REGION='${SCCACHE_REGION}'" >> /root/env
        echo "export AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}" >> /root/env
        # AWS_SECRET_ACCESS_KEY is a secret so we load it using ONBUILD ARG later on
        echo "export SCCACHE_BUCKET='${SCCACHE_BUCKET}'" >> /root/env
        echo "export SCCACHE_S3_USE_SSL=true" >> /root/env
        echo "export SCCACHE_S3_KEY_PREFIX='${SCCACHE_S3_KEY_PREFIX}/${TARGETARCH}/linux-musl'" >> /root/env
    elif [ -n "${SCCACHE_MEMCACHED}" ]; then
        # memcached
        echo "export SCCACHE_MEMCACHED='${SCCACHE_MEMCACHED}'" >> /root/env
    fi

    if [ -n "${RUSTC_WRAPPER}" ]; then
        echo "export CXX='${RUSTC_WRAPPER} clang++'" >> /root/env
        echo "export CC='${RUSTC_WRAPPER} clang'" >> /root/env
        echo "export RUSTC_WRAPPER='${RUSTC_WRAPPER}'" >> /root/env
        echo "export SCCACHE_SERVER_PORT=$((RANDOM+1025))" >> /root/env
    fi
    # for debugging, we display what we generated
    cat /root/env
EOS

# We provide secrets using ONBUILD ARG mechanism, to avoid putting them into a file and potentialy leaking them
# to the final image or to layer cache
ONBUILD ARG ACTIONS_RUNTIME_TOKEN
ONBUILD ARG AWS_SECRET_ACCESS_KEY

# Image containing compolation dependencies; used to overcome lack of interpolation in COPY --from
FROM deps-${RUSTC_WRAPPER:-base} AS deps-compilation
# Stage intentionally left empty

#
# BUILD ROCKSDB STATIC LIBRARY
#

FROM deps-compilation AS deps-rocksdb

RUN mkdir -p /tmp/rocksdb
WORKDIR /tmp/rocksdb

RUN <<EOS
set -ex -o pipefail
git clone https://github.com/facebook/rocksdb.git -b v8.10.2 --depth 1 .
source /root/env

if [[ "$TARGETARCH" == "amd64" ]] ; then
    export PORTABLE=haswell
else
    export PORTABLE=1
fi

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
RUN <<EOS
set -ex;
if [ "$TARGETARCH" = "amd64" ]; then
    CARGO_BINSTALL_ARCH="x86_64-unknown-linux-musl";
elif [ "$TARGETARCH" = "arm64" ]; then
    CARGO_BINSTALL_ARCH="aarch64-unknown-linux-musl";
else
    echo "Unsupported architecture: $TARGETARCH"; exit 1;
fi;
# Construct download URL
DOWNLOAD_URL="https://github.com/cargo-bins/cargo-binstall/releases/download/v${BINSTALL_VERSION}/cargo-binstall-${CARGO_BINSTALL_ARCH}.tgz";
# Download and extract the cargo-binstall binary
curl -A "Mozilla/5.0 (X11; Linux x86_64; rv:60.0) Gecko/20100101 Firefox/81.0" -L --proto '=https' --tlsv1.2 -sSf "$DOWNLOAD_URL" | tar -xvzf -;
./cargo-binstall -y --force cargo-binstall;
rm ./cargo-binstall;
source $HOME/.cargo/env;
cargo binstall -V

source $HOME/.cargo/env;

cargo binstall wasm-bindgen-cli@0.2.86 cargo-chef@0.1.67 \
    --locked \
    --no-discover-github-token \
    --disable-telemetry \
    --no-track \
    --no-confirm

EOS

#
# Rust build planner to speed up builds
#
FROM lklimek/dash-platform-build-base AS build-planner

WORKDIR /platform
COPY . .

RUN source $HOME/.cargo/env && \
    source /root/env && \
    cargo chef prepare --recipe-path recipe.json

# Workaround: as we cache dapi-grpc, its build.rs is not rerun, so we need to touch it
RUN touch /platform/packages/dapi-grpc/build.rs

#
# STAGE: BUILD RS-DRIVE-ABCI
#
# This will prebuild majority of dependencies
FROM deps AS build-drive-abci

SHELL ["/bin/bash", "-o", "pipefail","-e", "-x", "-c"]

WORKDIR /platform

COPY --from=build-planner /platform/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_${TARGETARCH},target=/platform/target \
    source $HOME/.cargo/env && \
    source /root/env && \
    cargo chef cook \
        --recipe-path recipe.json \
        --profile "$CARGO_BUILD_PROFILE" \
        --package drive-abci \
        --locked && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi

COPY . .

RUN mkdir /artifacts

# Build Drive ABCI
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_${TARGETARCH},target=/platform/target \
    source $HOME/.cargo/env && \
    source /root/env && \
    if  [[ "${CARGO_BUILD_PROFILE}" == "release" ]] ; then \
        mv .cargo/config-release.toml .cargo/config.toml && \
        export OUT_DIRECTORY=release ; \
    else \
        export FEATURES_FLAG="--features=console,grovedbg" ; \
        export OUT_DIRECTORY=debug ; \
    fi && \
    cargo build \
        --profile "${CARGO_BUILD_PROFILE}" \
        --package drive-abci \
        ${FEATURES_FLAG} \
        --locked && \
    cp /platform/target/${OUT_DIRECTORY}/drive-abci /artifacts/ && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi

#
# STAGE: BUILD JAVASCRIPT INTERMEDIATE IMAGE
#
FROM deps AS build-js

WORKDIR /platform

COPY --from=build-planner /platform/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_${TARGETARCH},target=/platform/target \
    source $HOME/.cargo/env && \
    source /root/env && \
    cargo chef cook \
        --recipe-path recipe.json \
        --profile "$CARGO_BUILD_PROFILE" \
        --package wasm-dpp \
        --target wasm32-unknown-unknown \
        --locked && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi

COPY . .

RUN --mount=type=cache,sharing=shared,id=cargo_registry_index,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,id=cargo_registry_cache,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,id=cargo_git,target=${CARGO_HOME}/git/db \
    --mount=type=cache,sharing=shared,id=target_wasm,target=/platform/target \
    --mount=type=cache,sharing=shared,id=unplugged_${TARGETARCH},target=/tmp/unplugged \
    source $HOME/.cargo/env && \
    source /root/env && \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn install --inline-builds && \
    cp -R /platform/.yarn/unplugged /tmp/ && \
    export SKIP_GRPC_PROTO_BUILD=1 && \
    yarn build && \
    if [[ -x /usr/bin/sccache ]]; then sccache --show-stats; fi

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
COPY --from=build-drive-abci /platform/packages/rs-drive-abci/.env.mainnet /var/lib/dash/rs-drive-abci/.env

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

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN yarn workspaces focus --production @dashevo/platform-test-suite

#  STAGE: FINAL TEST SUITE IMAGE
#
FROM node:20-alpine${ALPINE_VERSION} AS test-suite

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
COPY --from=build-test-suite /platform/packages/rs-platform-serialization-derive packages/rs-platform-serialization-derive
COPY --from=build-test-suite /platform/packages/rs-platform-version packages/rs-platform-version
COPY --from=build-test-suite /platform/packages/rs-platform-versioning packages/rs-platform-versioning
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
FROM node:20-alpine${ALPINE_VERSION} AS dapi

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk add --no-cache zeromq-dev

WORKDIR /platform/packages/dapi

COPY --from=build-dapi /platform/.yarn /platform/.yarn
COPY --from=build-dapi /platform/package.json /platform/yarn.lock /platform/.yarnrc.yml /platform/.pnp* /platform/
# List of required dependencies. Based on:
# yarn run ultra --info --filter '@dashevo/dapi' |  sed -E 's/.*@dashevo\/(.*)/COPY --from=build-dapi \/platform\/packages\/\1 \/platform\/packages\/\1/'
COPY --from=build-dapi /platform/packages/dapi /platform/packages/dapi
COPY --from=build-dapi /platform/packages/dapi-grpc /platform/packages/dapi-grpc
COPY --from=build-dapi /platform/packages/js-grpc-common /platform/packages/js-grpc-common
COPY --from=build-dapi /platform/packages/wasm-dpp /platform/packages/wasm-dpp
COPY --from=build-dapi /platform/packages/js-dapi-client /platform/packages/js-dapi-client

RUN cp /platform/packages/dapi/.env.example /platform/packages/dapi/.env

EXPOSE 2500 2501 2510
USER node
