# dash-platform-cxx

Transport-free C++ embedding surface for Dash Platform: proof verification,
DPP decoding and state-transition construction for an application that owns
its own DAPI transport, quorum-key sync and private keys (Dash Core's
platform GUI is the first consumer).

## What it is

A thin [`cxx`](https://cxx.rs) bridge over the workspace's own crates:

- verification is `drive-proof-verifier`'s `FromProof` impls, including the
  request-driven `FromProof<GetDocumentsRequest>` that rebuilds a document
  query from the wire bytes the transport actually sent;
- document assembly is `dash-platform-queries`' pure DPNS / DashPay builders,
  the same functions `dash-sdk` uses;
- signing is dpp's own `Signer` trait implemented over a C++ digest callback
  (`WalletSigner`, `include/dash/platform/signer.h`), so private keys never
  cross the FFI;
- quorum keys and data contracts come from a `ContextProvider` the embedder
  feeds from its locally synced LLMQ state; nothing here talks to the network.

`cxx` rather than the workspace's usual cbindgen C ABI because the surface is
dominated by nested byte vectors (`Vec<Vec<u8>>` of documents, key lists) and
fallible calls: cxx gives typed `rust::Vec` fields and `Result → rust::Error`
without hand-written length/ownership plumbing on either side.

## Trust boundary

Every request and response byte comes from an untrusted node. The GroveDB
replay necessarily runs before the quorum signature check (the root hash only
exists after replay), so:

- inputs are capped at `verify::MAX_MESSAGE_BYTES` before decoding;
- a proof naming any quorum type other than the network's Platform type is
  refused before its key is looked up (`set_context` records the type);
- every bridge entry point runs under `catch_unwind`: a panic anywhere in the
  decoders is reported as a `rust::Error`, never a process abort. The crate
  must therefore be built with `panic = "unwind"` (the default);
- the returned metadata (height, time, core height) is authenticated by the
  signature; deciding whether it is *fresh enough* is the embedder's job.

## Building and installing

The crate is an ordinary workspace member. Build systems vendor from the
workspace root and build only this package:

```sh
cargo vendor --locked vendored            # offline crate sources
cargo build -p dash-platform-cxx --locked --offline --release
```

`build.rs` stages the headers an embedder includes under
`target/<profile>/include/`:

```
include/dash/platform/ffi.h      # generated bridge (namespace platform_ffi)
include/dash/platform/signer.h   # WalletSigner callback type
include/rust/cxx.h               # cxx runtime header ffi.h includes
```

Install that `include/` tree and `target/<profile>/libdash_platform_cxx.a`.
`tests/cxx_smoke.cc` is a link-and-run check of exactly that interface; CI
runs it through `scripts/cxx-smoke.sh`.
