//! PA-005b — `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-005b.
//! Priority: P2.
//!
//! ## Status
//!
//! `BLOCKED — needs production API.` See spec status field.
//!
//! The wallet's only public derivation API today is
//! `PlatformAddressWallet::next_unused_receive_address`, which
//! delegates to `key_wallet::AddressPool::next_unused`. That helper
//! returns the LOWEST unused index — repeated calls yield the same
//! address until something marks it used (an inbound credit observed
//! via `sync_balances`). Driving the `DEFAULT_GAP_LIMIT = 20`
//! boundary therefore requires either:
//!
//! 1. **A production accessor** wrapping the upstream `AddressPool::next_unused_multiple(count)`
//!    helper. Suggested signature:
//!    ```rust,ignore
//!    pub async fn next_unused_receive_addresses(
//!        &self,
//!        account_key: PlatformPaymentAccountKey,
//!        count: usize,
//!    ) -> Result<Vec<PlatformAddress>, PlatformWalletError>;
//!    ```
//!    Calling with `count = 21` would return either 21 addresses
//!    (gap-limit grown) or a typed `GapLimitExceeded` error — exactly
//!    the contract PA-005b wants to pin.
//!
//! 2. **OR ~21 fund-and-derive rounds** that mark each address used
//!    in turn. Each round costs one bank fund call (~30s on testnet),
//!    so the test would run ~10 minutes per sub-case — operationally
//!    noisy and well past the P2 budget.
//!
//! The brief explicitly forbids production-side changes, so option 1
//! is unavailable. Option 2 is feasible but its 30+ minute runtime
//! across the triplet (3 sub-cases × 21 rounds × ~30s) is the reason
//! this case stays `#[ignore]`'d for now.

#[tokio_shared_rt::test(shared)]
#[ignore = "BLOCKED — needs production API: \
            PlatformAddressWallet::next_unused_receive_addresses(count) wrapping \
            key_wallet::AddressPool::next_unused_multiple. The 21-round funding \
            workaround works but is ~10 min runtime per sub-case. See spec status."]
async fn pa_005b_gap_limit_triplet() {
    panic!(
        "PA-005b is BLOCKED on a missing production API. \
         `PlatformAddressWallet::next_unused_receive_address` parks on the \
         lowest-unused index until observed-used; deriving 19/20/21 distinct \
         unused addresses requires either a `next_unused_multiple`-style \
         accessor (production change, ruled out) or ~30 min of testnet \
         funding rounds per sub-case. See TEST_SPEC.md → PA-005b → **Status**."
    );
}
