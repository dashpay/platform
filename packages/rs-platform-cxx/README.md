# Dash Platform CXX

`dash-platform-cxx` is the transport-free C++ embedding surface for Dash
Platform. It exposes proof verification, DPP decoding, and state-transition
construction while leaving transport, endpoint selection, quorum-state
synchronization, and private-key custody with the embedding application.

The Rust implementation and CXX schema live in this package. Consumers build
the `standalone` manifest as one static library and include
`dash/platform/ffi.h` and `dash/platform/signer.h` from the generated CXX
include tree. The standalone manifest has its own lockfile so embedders can
vendor this package's dependency closure without vendoring unrelated Platform
workspace packages.

Build and install the standalone archive with:

```sh
CARGO_TARGET_DIR=/path/to/target \
  cargo build --manifest-path standalone/Cargo.toml --locked --release
CARGO_TARGET_DIR=/path/to/target ./install.sh /path/to/prefix
```

The installed interface consists of `include/dash/platform/ffi.h`,
`include/dash/platform/signer.h`, the generated CXX headers they require, and
`lib/libdash_platform_cxx.a`. The embedding application remains responsible
for the platform-specific system libraries required by a Rust static library.

The normal dependency graph intentionally excludes `rs-dapi-client` and the
Hyper/Rustls/Tower transport stack. Generated protobuf client types, Tokio
utilities, and the context-provider abstraction remain available without a
native DAPI transport.
