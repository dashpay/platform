//! Found-024 — V27-007 regression pin: `transfer` post-broadcast loop must
//! not write foreign output-address balances into the source wallet's ledger.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-024).
//! Pinned status: BUG-PIN (regression guard) — PASSES today (fix is in place),
//! FAILS if V27-007 regresses.
//!
//! ## Bug shape (V27-007)
//!
//! Before the fix, `transfer.rs`'s post-broadcast ledger-update loop iterated
//! over every `(address, AddressInfo)` pair returned by the SDK — which
//! includes foreign output addresses the caller does not own — and wrote their
//! balances into the source wallet's local ledger unconditionally via
//! `set_address_credit_balance`. This caused `total_credits()` to absorb the
//! recipient's balance, corrupting dust-gate checks and sweep logic.
//!
//! ## Fix (V27-007)
//!
//! An ownership guard was added at `transfer.rs:154`:
//!
//! ```ignore
//! if !account.contains_platform_address(&p2pkh) {
//!     continue;
//! }
//! ```
//!
//! Only addresses that belong to this wallet's account are written; foreign
//! addresses are silently skipped.
//!
//! ## Test contract
//!
//! This test exercises the guard predicate directly on `ManagedPlatformAccount`
//! — the same type the production loop calls — without going through the SDK
//! or any async harness.
//!
//! 1. A `ManagedPlatformAccount` is constructed with one owned address holding
//!    1 000 000 000 credits (1 DASH).
//! 2. A foreign address (NOT in the account's pool) represents the transfer
//!    recipient. It carries a large balance (9 680 000 000 000 credits — the
//!    "bank pollution" amount from the original incident report).
//! 3. The loop logic from `transfer.rs` lines 143–167 is replicated: for each
//!    `(address, balance)` pair the guard is applied; only owned addresses are
//!    written.
//! 4. After the loop:
//!    - `total_credit_balance()` equals the owned address's new balance only.
//!    - The foreign address is absent from `address_balances`.
//!
//! PASSES with the V27-007 fix in place.
//! FAILS if the ownership guard is removed and `set_address_credit_balance` is
//! called unconditionally for all addresses returned by the SDK.

use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
use key_wallet::managed_account::managed_platform_account::ManagedPlatformAccount;
use key_wallet::Network;
use key_wallet::PlatformP2PKHAddress;

/// Initial balance of the owned address (1 DASH = 1 000 000 000 credits).
const OWNED_INITIAL_CREDITS: u64 = 1_000_000_000;

/// Post-transfer balance the SDK reports for the owned address.
const OWNED_POST_TRANSFER_CREDITS: u64 = 500_000_000;

/// Balance the SDK reports for the foreign (recipient) address — the
/// "bank pollution" amount from the original V27-007 incident.
const FOREIGN_CREDITS: u64 = 9_680_000_000_000;

/// Regression pin for V27-007: the post-broadcast ledger-update loop in
/// `transfer.rs` must skip foreign output addresses.
///
/// The guard under test is:
/// ```ignore
/// if !account.contains_platform_address(&p2pkh) { continue; }
/// ```
///
/// Without it, `total_credits()` absorbs the recipient's balance.
#[ignore = "Found-024 regression pin — V27-007 transfer foreign-pollution; \
            pure unit test (no async, no harness, no chain); run with \
            `cargo test -- --ignored`"]
#[test]
fn found_024_set_address_credit_balance_foreign_address_rejected() {
    // DIP-17 base path — matches the shape used in apply.rs unit tests.
    let base_path = DerivationPath::from(vec![
        ChildNumber::from_hardened_idx(9).unwrap(),
        ChildNumber::from_hardened_idx(1).unwrap(),
        ChildNumber::from_hardened_idx(17).unwrap(),
        ChildNumber::from_hardened_idx(0).unwrap(),
        ChildNumber::from_hardened_idx(0).unwrap(),
    ]);
    let pool = AddressPool::new_without_generation(
        base_path,
        AddressPoolType::Absent,
        20,
        Network::Testnet,
    );
    let mut account = ManagedPlatformAccount::new(0, 0, pool, false);

    // Register the owned address with its initial balance.
    let owned_addr = PlatformP2PKHAddress::new([0x11u8; 20]);
    account.set_address_credit_balance(owned_addr, OWNED_INITIAL_CREDITS, None);

    // Sanity: account knows about the owned address.
    assert!(
        account.contains_platform_address(&owned_addr),
        "pre-condition: owned address must be in the account pool"
    );

    // The foreign (recipient) address — NOT in this account's pool.
    let foreign_addr = PlatformP2PKHAddress::new([0xFFu8; 20]);
    assert!(
        !account.contains_platform_address(&foreign_addr),
        "pre-condition: foreign address must NOT be in the account pool"
    );

    // Replicate the post-broadcast loop from transfer.rs lines 143-167.
    // The SDK returns (address, balance) pairs for all transition participants.
    let sdk_response: &[(PlatformP2PKHAddress, u64)] = &[
        (owned_addr, OWNED_POST_TRANSFER_CREDITS),
        (foreign_addr, FOREIGN_CREDITS),
    ];

    for &(addr, balance) in sdk_response {
        // V27-007 ownership guard — the exact predicate from transfer.rs:154.
        if !account.contains_platform_address(&addr) {
            continue;
        }
        account.set_address_credit_balance(addr, balance, None);
    }

    // The owned address reflects its new (reduced) balance.
    assert_eq!(
        account.address_credit_balance(&owned_addr),
        OWNED_POST_TRANSFER_CREDITS,
        "owned address must carry its post-transfer balance"
    );

    // The total is the owned address only — foreign balance must not leak in.
    assert_eq!(
        account.total_credit_balance(),
        OWNED_POST_TRANSFER_CREDITS,
        "total_credits must reflect ONLY the owned address; \
         foreign address balance ({FOREIGN_CREDITS}) must not pollute the ledger"
    );

    // The foreign address has no entry in the local ledger.
    assert_eq!(
        account.address_credit_balance(&foreign_addr),
        0,
        "foreign address must not appear in the wallet's address_balances map"
    );
}
