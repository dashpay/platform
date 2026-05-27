//! TC-CODE-020-1 — feature-flag boundary check.
//!
//! Asserts that without the `sqlite` feature the crate compiles with a
//! minimal cross-cutting surface only (`WalletStorageError` is NOT
//! reachable; `SqlitePersister` is NOT reachable). The dev-deps for
//! this crate enable `sqlite`, so this file's purpose is to surface a
//! regression at *review time* by reading the manifest and the lib.rs
//! gating layout — it does NOT re-build the crate.
//!
//! The bare-build invariant itself is enforced by `cargo build
//! -p platform-wallet-storage --no-default-features` in CI; this test
//! pins the source-level expectations so the gate stays meaningful.

#[test]
fn tc_code_020_1_sqlite_items_are_feature_gated() {
    let lib_src = include_str!("../src/lib.rs");
    assert!(
        lib_src.contains(r#"#[cfg(feature = "sqlite")]"#),
        "lib.rs MUST gate sqlite re-exports behind the `sqlite` feature"
    );
    assert!(
        lib_src.contains(
            r#"#[cfg(feature = "sqlite")]
pub mod sqlite;"#
        ),
        "the `sqlite` module declaration MUST be cfg-gated"
    );
}

#[test]
fn tc_code_020_1_wallet_and_serde_deps_are_optional() {
    let manifest = include_str!("../Cargo.toml");
    // Both must be tagged optional so a bare build does NOT pull
    // platform-wallet (which transitively brings dpp / drive / dashcore)
    // or serde.
    assert!(
        manifest.contains("platform-wallet = { path = \"../rs-platform-wallet\", features = [\n    \"serde\",\n], optional = true }"),
        "platform-wallet MUST be optional (gated by the `sqlite` feature)"
    );
    assert!(
        manifest.contains("serde = { version = \"1\", features = [\"derive\"], optional = true }"),
        "serde MUST be optional (gated by the `sqlite` feature)"
    );
    // The sqlite feature MUST pull both back in.
    assert!(
        manifest.contains("\"dep:platform-wallet\""),
        "the sqlite feature MUST activate dep:platform-wallet"
    );
    assert!(
        manifest.contains("\"dep:serde\""),
        "the sqlite feature MUST activate dep:serde"
    );
}
