# Swift SDK - Architectural Rules

This document exists to keep the Swift SDK and its FFI layer from accreting
business logic that belongs in Rust. Read this before writing or reviewing
code in `Sources/SwiftDashSDK/`, `packages/rs-platform-wallet-ffi/`,
`packages/rs-sdk-ffi/`, or `packages/key-wallet-ffi/`.

## The rule

**The Swift SDK does exactly three things:**

1. **Persist data.** Write SwiftData rows and Keychain items in response to
   state emitted from Rust (the platform-wallet persister callback, the
   SDK's response types, etc.).
2. **Load data.** Expose `PersistentX` `@Query`-able rows so SwiftUI views
   can observe state reactively.
3. **Bridge.** Thin FFI wrappers around functions already defined in
   `key-wallet`, `rs-sdk`, or `platform-wallet`. Call → marshal in →
   marshal out → return.

**The FFI crates do exactly one thing:** expose existing Rust library
functions over a C ABI. Resolve the handle, call the function, marshal
the result. Nothing else.

## What that forbids

- No iteration / gap-limit walks / policy loops in Swift.
- No building derivation paths in Swift.
- No orchestrating multi-step derivation pipelines in Swift (mnemonic →
  seed → path → key → store).
- No pulling mnemonics or seeds across the FFI boundary so Swift can
  "finish" an operation Rust already knows how to complete.
- No re-implementing protocol constants (gap limit, key indices, path
  shapes) as Swift mirrors.
- No FFI function that stitches together calls the Rust library doesn't
  already expose as a single entry point. If it needs stitching, add the
  helper in the Rust library (e.g. `platform-wallet`) first, then expose
  it.

## High-level operations go through `platform-wallet`

Anything that spans identities / platform balances / core sync / tokens /
DashPay / identity key derivation / identity registration belongs in the
`platform-wallet` crate. The Swift SDK calls into `platform-wallet` via
`rs-platform-wallet-ffi` and persists/loads the results. It does not
re-implement the orchestration.

Examples that must route through `platform-wallet`:
- Identity registration and top-up.
- Identity discovery (gap-limit scan).
- Identity key derivation + any action that uses a derived key.
- Platform balance sync.
- Core SPV sync and UTXO tracking.
- Token balance sync.
- DashPay (contact requests, contacts, payments, profile).

## The one allowed exception: Keychain

iOS Keychain writes are the only operation Rust cannot perform from its
side, so Swift necessarily owns the final persist-the-private-key-bytes
step after Rust has derived them. Keep that interaction as narrow as
possible:

- **Do** accept `(path_string, 32_private_key_bytes)` from a Rust FFI
  call and write to Keychain.
- **Don't** fetch the mnemonic from Keychain, hand it back to Rust, wait
  for derived bytes, and write those to Keychain — that's the same
  pipeline orchestrated on the wrong side. If you need this pipeline,
  add a single FFI entry point that does the whole thing and returns the
  bytes ready to persist.

## Concrete precedent

The correct shape is `platform_wallet_discover_identities` (and its
sibling `platform_wallet_preview_identity_registration_keys`, added
under this rule): one FFI call that takes a wallet handle, does all the
derivation + Platform lookups + policy enforcement on the Rust side, and
hands back a flat array for Swift to render / persist.

Anti-precedent: any code that calls `WalletStorage().retrieveMnemonic(for:)`
in a Swift view or handler, then `Mnemonic.toSeed`, then
`KeyDerivation.getIdentityAuthenticationPath`, then a `key_wallet_derive_*`
FFI, then writes the result somewhere. That pipeline is
Rust-library-owned. Push it down.

## How to review

When reading Swift SDK or FFI code, ask one question per line:

> *"Is this marshalling values, or is it deciding something?"*

If it's deciding anything — how many, which index, which path, which
key, which order — move the decision to Rust. If you find a decision
that Rust doesn't currently let you ask for by a single call, add the
helper in the Rust library first.
