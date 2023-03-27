# syntax = docker/dockerfile:1.5

# Docker image for rs-drive-abci
#
# This image is divided into 3 stages, for better layer caching:
# - deps - includes all dependencies and some libraries
# - build - actual build process
# - release - final image
#

ARG ALPINE_VERSION=3.17

#
# DEPS: INSTALL AND CACHE DEPENDENCIES
#
FROM rust:alpine${ALPINE_VERSION} as deps

#
# Update Rust runtime
#
ARG CARGO_BUILD_PROFILE=debug
RUN rustup install stable

RUN apk add --no-cache \
        alpine-sdk \
        bash \
        clang-dev \
        git \
        linux-headers \
        openssl-dev \
        perl \
        sccache \
        unzip \
        wget

SHELL ["/bin/bash", "-c"]

# Install protoc - protobuf compiler
# The one shipped with Alpine does not work
ARG PROTOC_ARCH=x86_64
RUN wget -q -O /tmp/protoc.zip https://github.com/protocolbuffers/protobuf/releases/download/v22.2/protoc-22.2-linux-${PROTOC_ARCH}.zip && \
    unzip -qd /opt/protoc /tmp/protoc.zip && \
    rm /tmp/protoc.zip && \
    ln -s /opt/protoc/bin/protoc /usr/bin/

#
# EXECUTE BUILD
#
FROM deps as build

WORKDIR /usr/src/platform

COPY . .

#
# Configure sccache
#
# Set RUSTC_WRAPPER=/usr/bin/sccache to enable `sccache` caching.
ARG RUSTC_WRAPPER=/usr/bin/sccache
# Set args below to use Github Actions cache; see https://github.com/mozilla/sccache/blob/main/docs/GHA.md
ARG SCCACHE_GHA_ENABLED
ARG ACTIONS_CACHE_URL
ARG ACTIONS_RUNTIME_TOKEN
# Disable incremental buildings, not supported by sccache
ARG CARGO_INCREMENTAL=false

# Run the build, with extensive caching.
# 
# Note:
# 1. All these --mount... are to cache reusable info between runs.
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
# 2. We add `--config net.git-fetch-with-cli=true` to address ARM build issue,
# see https://github.com/rust-lang/cargo/issues/10781#issuecomment-1441071052
# 3. To preserve space on github cache, we call `cargo clean`.
RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/.crates.toml \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/.crates2.json \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    cargo build \
        --package drive-abci \
        --config net.git-fetch-with-cli=true && \
    cp /usr/src/platform/target/${CARGO_BUILD_PROFILE}/drive-abci /usr/src/platform/drive-abci && \
    cargo clean

#
# FINAL IMAGE
#
FROM alpine:${ALPINE_VERSION} AS release

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive ABCI Rust"

VOLUME /var/lib/platform

ENV DB_PATH=/var/lib/platform/rs-drive-abci/db

#
# Install binaries and data
#
WORKDIR /var/lib/platform

COPY --from=build /usr/src/platform/drive-abci /usr/bin/drive-abci
COPY --from=build /usr/src/platform/packages/rs-drive-abci/.env.example /var/lib/platform/rs-drive-abci/.env

#
# Create new non-root user
#
ARG USERNAME=platform
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN addgroup -g $USER_GID $USERNAME && \
    adduser -D -u $USER_UID -G $USERNAME -h /var/lib/platform/rs-drive-abci $USERNAME && \
    chown -R $USER_UID:$USER_GID /var/lib/platform/rs-drive-abci 

USER $USERNAME

WORKDIR /var/lib/platform/rs-drive-abci
ENTRYPOINT ["/usr/bin/drive-abci"]
CMD ["start"]

EXPOSE 26658
