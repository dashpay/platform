//! Found-024 — V27-007 regression pin: `transfer`'s post-broadcast persistence
//! builder must not write foreign output-address balances into the source
//! wallet's ledger.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-024).
//! Pinned status: BUG-PIN (regression guard) — PASSES today (fix is in place),
//! FAILS if V27-007 regresses.
//!
//! ## Bug shape (V27-007)
//!
//! Before the fix, `transfer`'s post-broadcast ledger-update loop persisted an
//! entry for every `(address, AddressInfo)` pair drive returned — which
//! includes foreign output addresses the caller does not own — mis-attributing
//! the recipient's balance to a fabricated derivation index in the source
//! wallet's ledger. This corrupted `total_credit_balance()` (dust-gate / sweep
//! logic) on the next restore.
//!
//! ## Fix (V27-007)
//!
//! `build_transfer_persistence_entries` (`transfer.rs`) filters every address
//! through the wallet's `owned` derivation-index map and keeps only entries
//! whose address is in the pool; foreign addresses are dropped.
//!
//! ## What this test drives
//!
//! It calls the REAL production function `build_transfer_persistence_entries`
//! (exposed via the `test-utils` seam — pulled in by `e2e`), NOT an inline copy
//! of the loop. Deleting the `owned`-membership guard from the production
//! function therefore turns this test RED — which is the whole point of a
//! regression pin.

use std::collections::BTreeMap;

use dash_sdk::query_types::AddressInfo;
use dpp::address_funds::PlatformAddress;
use key_wallet::PlatformP2PKHAddress;
use platform_wallet::wallet::platform_addresses::transfer::test_utils::build_transfer_persistence_entries;

/// DIP-17 account context for the synthetic changeset.
const WALLET_ID: [u8; 32] = [0x24u8; 32];
const ACCOUNT_INDEX: u32 = 0;

/// Derivation index drive returns for the owned input address.
const OWNED_ADDRESS_INDEX: u32 = 3;

/// Post-transfer balance drive reports for the owned address.
const OWNED_POST_TRANSFER_CREDITS: u64 = 500_000_000;

/// Balance drive reports for the foreign (recipient) address — the "bank
/// pollution" amount from the original V27-007 incident.
const FOREIGN_CREDITS: u64 = 9_680_000_000_000;

/// Regression pin for V27-007: the production persistence builder must drop
/// foreign output addresses so they never pollute the source wallet's ledger.
///
/// Drives the real `build_transfer_persistence_entries`; if the ownership guard
/// inside it is removed, the foreign address produces a persistence entry and
/// the assertions below fail.
#[test]
fn found_024_transfer_persistence_builder_drops_foreign_address() {
    let owned_addr = PlatformP2PKHAddress::new([0x11u8; 20]);
    let owned_platform = PlatformAddress::P2pkh([0x11u8; 20]);
    // Foreign recipient — NOT in the wallet's derived pool.
    let foreign_addr = PlatformP2PKHAddress::new([0xFFu8; 20]);
    let foreign_platform = PlatformAddress::P2pkh([0xFFu8; 20]);

    // The wallet's derived address pool: only the owned address, at its real
    // derivation index. The production guard filters against exactly this map.
    let mut owned: BTreeMap<PlatformP2PKHAddress, u32> = BTreeMap::new();
    owned.insert(owned_addr, OWNED_ADDRESS_INDEX);

    // The address-info set drive returns spans inputs ∪ outputs, so it carries
    // BOTH the owned input and the foreign recipient.
    let owned_info = AddressInfo {
        address: owned_platform,
        nonce: 1,
        balance: OWNED_POST_TRANSFER_CREDITS,
    };
    let foreign_info = AddressInfo {
        address: foreign_platform,
        nonce: 0,
        balance: FOREIGN_CREDITS,
    };
    let address_infos: BTreeMap<PlatformAddress, Option<AddressInfo>> = [
        (owned_platform, Some(owned_info)),
        (foreign_platform, Some(foreign_info)),
    ]
    .into_iter()
    .collect();

    let entries = build_transfer_persistence_entries(
        WALLET_ID,
        ACCOUNT_INDEX,
        &owned,
        address_infos.iter().map(|(a, i)| (a, i.as_ref())),
    );

    // The builder must emit exactly one entry — the owned address — and the
    // foreign recipient must be absent. If the guard regresses, a second entry
    // for the foreign address appears and these fail.
    assert_eq!(
        entries.len(),
        1,
        "exactly one (owned) persistence entry expected; foreign recipient must be filtered. \
         entries={entries:?}"
    );
    let entry = &entries[0];
    assert_eq!(entry.wallet_id, WALLET_ID);
    assert_eq!(entry.account_index, ACCOUNT_INDEX);
    assert_eq!(
        entry.address, owned_addr,
        "the single entry must be the wallet-owned address"
    );
    assert_eq!(
        entry.address_index, OWNED_ADDRESS_INDEX,
        "owned address must keep its real derivation index, not a fabricated one"
    );
    assert_eq!(entry.funds.balance, OWNED_POST_TRANSFER_CREDITS);

    assert!(
        !entries.iter().any(|e| e.address == foreign_addr),
        "foreign address ({FOREIGN_CREDITS} credits) must never appear in a persistence entry — \
         that is the V27-007 ledger pollution this pin guards against"
    );
}
