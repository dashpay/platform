#!/usr/bin/env bash

PROFILE_ARG=""
FEATURES_ARG=""

if [ -n "$CARGO_BUILD_PROFILE" ]; then
    if [ "$CARGO_BUILD_PROFILE" == "release" ]; then
      PROFILE_ARG="--release"
    elif [ "$CARGO_BUILD_PROFILE" != "debug" ]; then
      PROFILE_ARG="--profile $CARGO_BUILD_PROFILE"
      FEATURES_ARG="--features enable-core-rpc-mocking"
    fi
fi

cargo-cp-artifact -ac drive-nodejs native/index.node -- \
  cargo build --message-format=json-render-diagnostics $PROFILE_ARG $FEATURES_ARG --features enable-core-rpc-mocking  \
  && neon-tag-prebuild \
  && rm -rf native
