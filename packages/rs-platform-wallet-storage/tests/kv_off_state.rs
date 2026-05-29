//! Off-state guard for the `kv` feature.
//!
//! Mirrors the `feature_flag_build.rs` pattern: a source-level audit
//! that the `KvStore` trait, `KvError`, the SQLite-backed impl, and
//! the top-level re-exports are all cfg-gated behind `feature = "kv"`.
//! The bare-build invariant itself is enforced by
//! `cargo build -p platform-wallet-storage --no-default-features
//! --features sqlite,cli` in CI; this test pins the source-level
//! expectations so the gate stays meaningful.

#[test]
fn kv_module_is_feature_gated_in_lib_rs() {
    let lib_src = include_str!("../src/lib.rs");
    assert!(
        lib_src.contains(
            r#"#[cfg(feature = "kv")]
pub mod kv;"#
        ),
        "the top-level `kv` module declaration MUST be gated behind `feature = \"kv\"`"
    );
    assert!(
        lib_src.contains(
            r#"#[cfg(feature = "kv")]
pub use kv::{KvError, KvStore};"#
        ),
        "the `KvError`/`KvStore` re-exports MUST be gated behind `feature = \"kv\"`"
    );
}

#[test]
fn sqlite_kv_impl_is_feature_gated_in_sqlite_mod_rs() {
    let mod_src = include_str!("../src/sqlite/mod.rs");
    assert!(
        mod_src.contains(
            r#"#[cfg(feature = "kv")]
pub mod kv;"#
        ),
        "the SQLite-backed `kv` impl module MUST be gated behind `feature = \"kv\"`"
    );
}

#[test]
fn kv_feature_requires_sqlite_in_manifest() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(r#"kv = ["sqlite"]"#),
        "the `kv` feature MUST activate `sqlite` (the only backend it can run on)"
    );
}

/// On-state symbol check: under any build with `kv` enabled the public
/// trait, error type, and length constant MUST resolve through the
/// crate-root re-exports. Compile-time only — if a future edit
/// removes the re-exports, this file fails to compile.
#[cfg(feature = "kv")]
#[allow(dead_code)]
fn _kv_symbols_are_present() {
    use platform_wallet_storage::kv::MAX_KEY_LEN;
    use platform_wallet_storage::{KvError, KvStore};
    let _ = MAX_KEY_LEN;
    fn _accepts_kv_store(_: &dyn KvStore) {}
    fn _accepts_kv_error(_: &KvError) {}
}
