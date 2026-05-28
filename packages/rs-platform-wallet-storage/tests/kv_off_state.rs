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

#[test]
fn kv_is_in_the_default_feature_set() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        default_features(manifest).iter().any(|f| f == "kv"),
        "`kv` MUST be in the default feature set so the API is on by default"
    );
}

/// Parse the `default = [...]` entry of the `[features]` table and return
/// its members as bare strings (no quotes/whitespace/commas). Robust against
/// reordering and additions so the on-by-default invariant survives future
/// feature-set churn without retouching this test.
fn default_features(manifest: &str) -> Vec<String> {
    let features_idx = manifest
        .find("\n[features]")
        .expect("Cargo.toml must contain a [features] table");
    let after_header = &manifest[features_idx + 1..];
    // Stop at the next `[section]` header so we don't bleed into other tables.
    let section_end = after_header[1..]
        .find("\n[")
        .map(|i| i + 1)
        .unwrap_or(after_header.len());
    let features_section = &after_header[..section_end];

    let default_line_start = features_section
        .find("\ndefault")
        .or_else(|| features_section.starts_with("default").then_some(0))
        .expect("[features] must declare a `default` entry");
    let from_default = &features_section[default_line_start..];
    let open = from_default
        .find('[')
        .expect("`default` must be assigned an array");
    let close = from_default[open..]
        .find(']')
        .expect("`default` array must be closed");
    let inner = &from_default[open + 1..open + close];

    inner
        .split(',')
        .map(|tok| tok.trim().trim_matches('"').to_string())
        .filter(|tok| !tok.is_empty())
        .collect()
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
