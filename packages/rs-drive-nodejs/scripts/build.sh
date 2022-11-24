#!/usr/bin/env bash

PROFILE_ARG=""

if [ -n "$CARGO_BUILD_PROFILE" ]; then
    if [ "$CARGO_BUILD_PROFILE" == "release" ]; then
      PROFILE_ARG="--release"
    elif [ "$CARGO_BUILD_PROFILE" != "debug" ]; then
      PROFILE_ARG="--profile $CARGO_BUILD_PROFILE"
    fi
fi

cargo-cp-artifact -ac drive-nodejs native/index.node -- \
  cargo build --message-format=json-render-diagnostics $PROFILE_ARG \
  && neon-tag-prebuild \
  && rm -rf native
