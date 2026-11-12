//! Notes sent to fresh shielded addresses are found by the scan, and again
//! by a wallet restored from the same seed (spec AC-04, AC-05).
//!
//! Fresh addresses are drawn at random diversifier indices and never
//! stored; the scan finds their notes through the account's IVK alone.
//! Each test drives the real coordinator against a mock Platform pool that
//! holds one note to each of N fresh addresses plus one to a foreign
//! address.

use tokio_util::sync::CancellationToken;

use super::cancelled_scan_tests::{
    chunked_pool, coordinator_over, host_note_values, note_to, WALLET,
};
use super::{OrchardKeySet, SubwalletId};

const SEED: [u8; 64] = [42; 64];
const FOREIGN_SEED: [u8; 64] = [24; 64];
const FRESH_VALUES: [u64; 5] = [101, 102, 103, 104, 105];
const FOREIGN_VALUE: u64 = 999;

fn views(seed: &[u8; 64]) -> super::AccountViewingKeys {
    OrchardKeySet::from_seed(seed, dashcore::Network::Testnet, 0)
        .expect("test viewing keys")
        .viewing_keys()
}

/// One note to each of `FRESH_VALUES.len()` fresh addresses of `SEED`'s
/// account, then one to a foreign address. Returns the notes and the raw
/// fresh addresses.
fn pool_to_fresh_addresses() -> (
    Vec<drive_proof_verifier::types::ShieldedEncryptedNote>,
    Vec<[u8; 43]>,
) {
    let drawer = views(&SEED);
    let mut notes = Vec::new();
    let mut addresses = Vec::new();
    for value in FRESH_VALUES {
        let address = drawer.fresh_address().expect("fresh address");
        addresses.push(address.to_raw_address_bytes());
        notes.push(note_to(address, value));
    }
    notes.push(note_to(views(&FOREIGN_SEED).default_address, FOREIGN_VALUE));
    (notes, addresses)
}

/// Scan `notes` with keys freshly derived from `SEED` — a new wallet
/// object that never saw the addresses being drawn — and return the
/// recorded note values and the spendable balance.
async fn scan_with_fresh_keys(
    notes: &[drive_proof_verifier::types::ShieldedEncryptedNote],
) -> (Vec<u64>, u64) {
    let keys = views(&SEED);
    let sub = SubwalletId::new(WALLET, 0);
    let (sdk, _) = chunked_pool(&[notes.to_vec()]).await;
    let (coordinator, host) = coordinator_over(sdk, &keys).await;
    let summary = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(
        summary.success_count(),
        1,
        "the pass completes: {summary:?}"
    );
    let balance = coordinator
        .store()
        .read()
        .await
        .local_account_balance(sub)
        .expect("balance")
        .spendable_credits;
    (host_note_values(&host, sub), balance)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notes_to_fresh_addresses_are_found_by_the_scan() {
    let (notes, _) = pool_to_fresh_addresses();
    let (values, balance) = scan_with_fresh_keys(&notes).await;
    assert_eq!(
        values,
        FRESH_VALUES.to_vec(),
        "every fresh note, no foreign one"
    );
    assert_eq!(balance, FRESH_VALUES.iter().sum::<u64>());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restore_from_the_same_seed_finds_notes_to_fresh_addresses() {
    let (notes, addresses) = pool_to_fresh_addresses();

    // The live wallet scans the pool …
    let (live_values, live_balance) = scan_with_fresh_keys(&notes).await;
    // … and a wallet restored on another device scans it from scratch.
    let (restored_values, restored_balance) = scan_with_fresh_keys(&notes).await;
    assert_eq!(restored_values, live_values);
    assert_eq!(restored_balance, live_balance);
    assert_eq!(restored_values, FRESH_VALUES.to_vec());

    let restored = views(&SEED);
    for address in &addresses {
        assert!(
            restored.is_own_recipient(address),
            "a restored wallet recognises every fresh address as its own"
        );
    }
}
