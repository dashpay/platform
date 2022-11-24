#!/usr/bin/env bash

if [ -z "$CARGO_BUILD_PROFILE" ]; then
    CARGO_BUILD_PROFILE="debug"
fi

cargo-cp-artifact -ac drive-nodejs native/index.node -- \
  cargo build --message-format=json-render-diagnostics --$CARGO_BUILD_PROFILE \
  && neon-tag-prebuild \
  && rm -rf native
