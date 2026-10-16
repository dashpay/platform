# dash-sdk-contract

Contract-author SDK for DashVM: the declaration model behind `#[persistent]`,
`#[index]`, `#[rule]` and `#[entry]`, the attribute grammar those macros
implement, the diagnostics they report, and the canonical manifest a contract
package publishes.

```rust
use dash_sdk_contract::prelude::*;
```

This crate specifies the author-facing model. It carries no proc macros, no
host context and no runtime. It compiles without `std` on `wasm32v1-none`:

```bash
cargo check -p dash-sdk-contract --no-default-features --target wasm32v1-none
cargo test -p dash-sdk-contract
cargo test -p dash-sdk-contract --no-default-features
```

The chapter [Contract Declarations and the Author API](../../book/src/dashvm/contract-declarations.md)
in the Dash Platform Book describes the grammar, the identity rules, the
persistence semantics and what the validator checks versus what native
validation enforces.
