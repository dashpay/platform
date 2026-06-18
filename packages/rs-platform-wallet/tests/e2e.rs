//! End-to-end integration tests for `rs-platform-wallet`.
//!
//! Single test binary with a process-shared `E2eContext` (bank
//! wallet, SDK, panic-safe registry). `framework/` provides the
//! harness; `cases/` hosts `#[tokio_shared_rt::test(shared)]` entries.
//
// TODO(bilby, 2026-06-18, #3549 base-merge into v3.1-dev): post-merge
// validation found a REGRESSION — the base-merge is NOT "no worse than the
// recorded baseline". The compile gate passes, but at runtime v3.1-dev's new
// `PlatformWalletManager::register_wallet` -> `wallet.downgrade_to_external_signable()`
// (manager/wallet_lifecycle.rs:244, absent on the pre-merge branch) strips the
// private key from every registered wallet. The bank's hardened DIP-17 / BIP-44
// address derivation (framework/bank.rs `derive_platform_address_at_index` /
// `derive_core_receive_address_at_index`, both via the managed wallet's
// `derive_public_key`) then fails with "External signable wallet has no private
// key", killing bank setup and therefore EVERY bank-funded case (cr_001/cr_004/
// id_001/id_002 were green in the baseline; al_001/dpns_001/id_002b/cr_003 also
// affected). FIX: derive the bank's primary/core addresses from the retained
// seed (a fresh signable Wallet::from_seed_bytes) instead of the downgraded
// managed wallet — the framework already signs everything else via seed-based
// signers, so the blast radius is just those two bank helpers + print_bank_address_offline.
// SECONDARY: `--test-threads=14` OOM-killed the run at 24/170 on this 19 GB-RAM
// box; use fewer threads (~4) for a complete pass once the regression is fixed.

#![allow(dead_code, unused_imports)]
#![allow(clippy::result_large_err)]

// `tests/e2e.rs` is the integration-test crate root; explicit
// `#[path]` keeps the on-disk layout grouped under `tests/e2e/`.
#[path = "e2e/cases/mod.rs"]
mod cases;
#[path = "e2e/framework/mod.rs"]
mod framework;
