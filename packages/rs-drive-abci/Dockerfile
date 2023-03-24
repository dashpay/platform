# syntax = docker/dockerfile:1.5

# Docker image for rs-drive-abci
#
# This image is divided into 3 stages, for better layer caching:
# - deps - includes all dependencies and some libraries
# - build - actual build process
# - release - final image
#
# We use Debian as base, because Alpine causes segmentation fault in git2 rust crate.

#
# DEPS: INSTALL AND CACHE DEPENDENCIES
#
FROM rust:bullseye as deps

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive ABCI Rust"

RUN --mount=type=cache,sharing=locked,target=/var/lib/apt/lists \
    --mount=type=cache,sharing=locked,target=/var/cache/apt \
    rm -f /etc/apt/apt.conf.d/docker-clean \
    && apt-get update \
    && apt-get install --yes \
        build-essential \
        libclang-dev \
        libssl-dev \
        protobuf-compiler \
        git \
        wget \
        unzip \
        bash

SHELL ["/bin/bash", "-c"]

#
# Install sccache - build cache for Rust
#
ARG SCCACHE_URL="https://github.com/mozilla/sccache/releases/download/v0.4.0/sccache-v0.4.0-x86_64-unknown-linux-musl.tar.gz"
RUN wget -q -O /tmp/sccache.tar.gz ${SCCACHE_URL} \
    && mkdir -p /tmp/sccache \
    && tar -z -C /tmp/sccache -xf /tmp/sccache.tar.gz \
    && cp /tmp/sccache/sccache*/sccache /usr/bin/sccache \
    && rm -r /tmp/sccache.tar.gz /tmp/sccache

#
# Update Rust runtime
#
ARG CARGO_BUILD_PROFILE=debug
RUN rustup install stable

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
# See https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci
#    --mount=type=cache,sharing=private,target=/usr/src/platform/target \
RUN --mount=type=cache,sharing=shared,target=/root/.cache/sccache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/.crates.toml \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/.crates2.json \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/index \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/registry/cache \
    --mount=type=cache,sharing=shared,target=${CARGO_HOME}/git/db \
    cargo build -p drive-abci \
    && cp /usr/src/platform/target/${CARGO_BUILD_PROFILE}/drive-abci /usr/src/platform/drive-abci

#
# FINAL IMAGE
#
FROM debian:bullseye-slim AS release

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
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID --home /var/lib/platform/rs-drive-abci --create-home $USERNAME \
    && chown -R $USER_UID:$USER_GID /var/lib/platform/rs-drive-abci 

USER $USERNAME

WORKDIR /var/lib/platform/rs-drive-abci
ENTRYPOINT ["/usr/bin/drive-abci"]
CMD ["start"]

EXPOSE 26658
