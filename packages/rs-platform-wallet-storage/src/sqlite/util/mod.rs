//! Shared internal helpers (safe casts, file permissions, etc.).

// `permissions` backs the persister's and backup writer's on-disk hardening and
// has no production caller outside this crate, so it stays crate-private rather
// than becoming semver surface. `__test-helpers` widens it the same way `schema`
// and `migrations` are widened in `sqlite/mod.rs`, so this crate's own
// integration tests can assert the applied modes directly.
#[cfg(any(test, feature = "__test-helpers"))]
pub mod permissions;
#[cfg(not(any(test, feature = "__test-helpers")))]
pub(crate) mod permissions;
pub mod safe_cast;
