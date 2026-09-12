//! End-to-end integration tests for `rs-platform-wallet`.
//!
//! Single test binary with a process-shared `E2eContext` (bank
//! wallet, SDK, panic-safe registry). `framework/` provides the
//! harness; `cases/` hosts `#[tokio_shared_rt::test(shared)]` entries.
//
// NOTE(bilby, 2026-06-18, #3549 base-merge into v3.1-dev): the base-merge
// introduced a runtime regression (NOT caught by the compile gate) that is now
// FIXED. v3.1-dev's `PlatformWalletManager::register_wallet` ->
// `wallet.downgrade_to_external_signable()` (manager/wallet_lifecycle.rs)
// strips the private key from every registered wallet, so the bank's hardened
// DIP-17 / BIP-44 address derivation could no longer derive from the managed
// wallet and failed with "External signable wallet has no private key",
// breaking every bank-funded case at setup. RESOLVED in `framework/bank.rs`:
// `derive_platform_address_at_index` / `derive_core_receive_address_at_index`
// now rebuild a fresh signable `Wallet::from_seed_bytes` from the bank's
// retained seed (same root key -> identical addresses) and derive from that.
// Verified: the "External signable" error is gone from every test path and the
// bank funds asset locks again. (Production also logs a benign WARN — identity
// discovery during registration is now deferred for external-signable wallets;
// expected, handled via `PlatformWallet::identity().discover()`.)
//
// ENV CAVEAT: bank-funded network cases (cr_001/cr_004/id_001/id_002/id_002b)
// still fail on the CURRENT degraded testnet — asset-lock "Timed out waiting
// for finality proof" (ChainLock/InstantSend latency) and bank Core-balance
// depletion — same environmental class as the baseline's 36 reds, not a code
// regression. Confirm green on a healthy testnet with a topped-up bank Core
// balance. Also: `--test-threads=14` OOM-killed a full run at 24/170 on this
// 19 GB-RAM box; the suite serializes on the SPV slot-lock, so ~4 threads is
// plenty and avoids the OOM.

#![allow(dead_code, unused_imports)]
#![allow(clippy::result_large_err)]

// `tests/e2e.rs` is the integration-test crate root; explicit
// `#[path]` keeps the on-disk layout grouped under `tests/e2e/`.
#[path = "e2e/cases/mod.rs"]
mod cases;
#[path = "e2e/framework/mod.rs"]
mod framework;
