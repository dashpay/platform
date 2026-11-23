//! A pending spend whose input another transition spent (dashpay/platform#6108).
//!
//! A spend the wallet could not confirm stays pending, with its notes
//! reserved and its signed transition saved. When a second device on the same
//! seed spends one of its inputs first, the saved transition can never
//! execute. The scan tells that from the action it reads for the input: the
//! note item holds the action's nullifier and its output commitment together,
//! so an item that carries the input's nullifier with a commitment the saved
//! transition does not pair it with is another transition's action.
//!
//! Feeds `sync_notes_from_stream` synthetic batches, so no network is used.

use std::sync::Arc;

use dash_sdk::platform::shielded::notes_sync::types::ShieldedChunkBatch;
use dpp::platform_value::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::shielded::SerializedAction;
use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::state_transition::StateTransition;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::{Stream, StreamExt};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::{sync_notes_from_stream, MultiSyncNotesResult};
use crate::wallet::shielded::activity::{
    ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
};
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::{AccountViewingKeys, OrchardKeySet};
use crate::wallet::shielded::store::{PendingRedrive, ShieldedNote, ShieldedStore, SubwalletId};

type BatchItem = Result<ShieldedChunkBatch, dash_sdk::Error>;
type Store = Arc<RwLock<FileBackedShieldedStore>>;

const ANCHOR: [u8; 32] = [0x52; 32];
const NOTE_VALUE: u64 = 1_000;
/// The spend under test.
const SPEND: [u8; 32] = [0xE1; 32];
/// Another attempt of the same wallet.
const OTHER_SPEND: [u8; 32] = [0xE2; 32];

fn temp_tree_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("shielded_spent_elsewhere_{tag}_{nanos}.sqlite"))
}

fn open(path: &std::path::Path) -> Store {
    Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(path, 100).expect("open store"),
    ))
}

fn subwallet() -> (SubwalletId, AccountViewingKeys) {
    let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
        .expect("derive viewing keys")
        .viewing_keys();
    (SubwalletId::new([0x11; 32], 0), views)
}

/// The nullifier of the wallet's note `n`.
fn nullifier(n: u8) -> [u8; 32] {
    [n; 32]
}

/// A commitment the tree accepts: a small value is a valid Pallas element.
fn cmx(n: u8) -> [u8; 32] {
    let mut cmx = [0u8; 32];
    cmx[0] = n;
    cmx
}

/// An action of a saved transition: it spends the note with `nullifier` and
/// creates the note with commitment `cmx`.
fn action(nullifier: [u8; 32], cmx: [u8; 32]) -> SerializedAction {
    SerializedAction {
        nullifier,
        rk: [2; 32],
        cmx,
        encrypted_note: vec![4; 216],
        cv_net: [5; 32],
        spend_auth_sig: [6; 64],
    }
}

/// A shielded transfer of `actions`. Only its bytes are saved, so it carries
/// no proof.
fn transfer(actions: Vec<SerializedAction>) -> StateTransition {
    ShieldedTransferTransitionV0 {
        actions,
        value_balance: 0,
        anchor: ANCHOR,
        proof: vec![],
        binding_signature: [0; 64],
    }
    .into()
}

/// A token purchase from the shielded pool whose fee bundle, the one written
/// to the credit pool, holds `fee_actions`. Its token bundle goes to the
/// token's pool, which this scan does not read.
fn purchase(fee_actions: Vec<SerializedAction>) -> StateTransition {
    StateTransition::TokenPurchaseFromShieldedPool(TokenPurchaseFromShieldedPoolTransition::V0(
        TokenPurchaseFromShieldedPoolTransitionV0 {
            data_contract_id: Identifier::from([0x44; 32]),
            token_contract_position: 0,
            token_id: Identifier::from([0x45; 32]),
            token_count: 10,
            total_agreed_price: 1_000,
            token_actions: vec![
                action(nullifier(0xB1), cmx(20)),
                action(nullifier(0xB2), cmx(21)),
            ],
            token_anchor: [7; 32],
            token_proof: vec![8; 64],
            token_binding_signature: [9; 64],
            fee_actions,
            fee_anchor: ANCHOR,
            fee_proof: vec![8; 64],
            fee_binding_signature: [9; 64],
            credit_amount: 2_000,
        },
    ))
}

/// The note item an executed action leaves in the commitment tree. Its empty
/// ciphertext opens under no key, and the one subwallet drives the stream, so
/// the scanner reads only the nullifier and the commitment.
fn scanned(nullifier: [u8; 32], cmx: [u8; 32]) -> ShieldedEncryptedNote {
    ShieldedEncryptedNote {
        cmx: cmx.to_vec(),
        nullifier: nullifier.to_vec(),
        cv_net: vec![],
        encrypted_note: vec![],
    }
}

/// The whole pool as one batch from position 0.
fn pool(notes: Vec<ShieldedEncryptedNote>) -> ShieldedChunkBatch {
    let total_count = notes.len() as u64;
    ShieldedChunkBatch {
        start_index: 0,
        notes,
        decrypted: vec![],
        block_height: 1_000,
        is_partial: true,
        total_count,
    }
}

fn unspent_note(nullifier: [u8; 32], position: u64) -> ShieldedNote {
    ShieldedNote {
        position,
        cmx: [0x2A; 32],
        nullifier,
        block_height: 10,
        is_spent: false,
        value: NOTE_VALUE,
        note_data: vec![0u8; 115],
    }
}

fn pending_row(activity_id: [u8; 32], inputs: &[[u8; 32]]) -> ShieldedActivityEntry {
    ShieldedActivityEntry {
        id: activity_id,
        kind: ShieldedActivityKind::Sent,
        direction: ShieldedDirection::Out,
        amount: 500,
        fee: Some(10),
        counterparty: Some(vec![1u8; 43]),
        memo: None,
        block_height: None,
        status: ShieldedActivityStatus::Pending,
        created_at_ms: 1,
        min_note_position: None,
        note_cmxs: vec![[0x7C; 32]],
        spent_nullifiers: inputs.to_vec(),
    }
}

/// What the host restores at a bind: the wallet's notes, unspent, and the
/// spend's pending row. The file store keeps neither across a reopen.
fn restore_notes_and_row(
    store: &mut FileBackedShieldedStore,
    id: SubwalletId,
    activity_id: [u8; 32],
    inputs: &[[u8; 32]],
) {
    for (position, input) in (100u64..).zip(inputs) {
        store
            .save_note(id, &unspent_note(*input, position))
            .expect("save the note");
    }
    store
        .save_activity(id, &pending_row(activity_id, inputs))
        .expect("save the row");
}

/// Leave the store as a spend of `inputs` that ended unconfirmed leaves it:
/// its notes reserved under `activity_id`, its signed `transition` saved, its
/// row pending.
fn save_unconfirmed_spend(
    store: &mut FileBackedShieldedStore,
    id: SubwalletId,
    activity_id: [u8; 32],
    transition: &StateTransition,
    inputs: &[[u8; 32]],
) {
    restore_notes_and_row(store, id, activity_id, inputs);
    for input in inputs {
        store.mark_pending(id, input).expect("reserve the note");
        store
            .set_pending_spend(id, input, ANCHOR, activity_id, Some(5))
            .expect("link the reservation");
    }
    store
        .arm_redrive(
            id,
            PendingRedrive {
                activity_id,
                anchor: ANCHOR,
                nullifiers: inputs.to_vec(),
                st_bytes: transition.serialize_to_bytes().expect("serialize"),
                attempts: 0,
                identity_nonce_finalized: false,
                identity_user_abandoned: false,
                anchor_height: Some(5),
            },
        )
        .expect("save the transition");
}

async fn scan(store: &Store, stream: impl Stream<Item = BatchItem>) -> MultiSyncNotesResult {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sync_notes_from_stream(
            store,
            &[subwallet()],
            None,
            &CancellationToken::new(),
            move |_, _| stream,
        ),
    )
    .await
    .expect("the pass returns")
    .expect("the pass succeeds")
}

async fn scan_pool(store: &Store, notes: Vec<ShieldedEncryptedNote>) -> MultiSyncNotesResult {
    scan(store, futures::stream::iter(vec![Ok(pool(notes))])).await
}

fn status(
    store: &FileBackedShieldedStore,
    id: SubwalletId,
    activity_id: [u8; 32],
) -> ShieldedActivityStatus {
    store
        .get_activity_by_entry_id(id, &activity_id)
        .expect("read the row")
        .expect("the row")
        .status
}

fn unspent(store: &FileBackedShieldedStore, id: SubwalletId) -> Vec<[u8; 32]> {
    let mut nullifiers: Vec<[u8; 32]> = store
        .get_unspent_notes(id)
        .expect("unspent notes")
        .into_iter()
        .map(|note| note.nullifier)
        .collect();
    nullifiers.sort_unstable();
    nullifiers
}

/// The statuses of `activity_id`'s row the pass handed the host.
fn reported(
    result: &MultiSyncNotesResult,
    id: SubwalletId,
    activity_id: [u8; 32],
) -> Vec<ShieldedActivityStatus> {
    result
        .changeset
        .activity_entries
        .get(&id)
        .into_iter()
        .flatten()
        .filter(|entry| entry.id == activity_id)
        .map(|entry| entry.status)
        .collect()
}

/// A spend of one note: its bundle is padded to two actions, the second
/// spending no note of the wallet's.
fn single_input_spend() -> StateTransition {
    transfer(vec![
        action(nullifier(0xA1), cmx(1)),
        action(nullifier(0xD0), cmx(2)),
    ])
}

/// One input, spent by another transition: with no other note reserved, the
/// row has nothing else to fail it.
#[tokio::test]
async fn should_fail_a_pending_spend_whose_single_input_another_transition_spent() {
    let path = temp_tree_path("single");
    let store = open(&path);
    let (id, _) = subwallet();
    let input = nullifier(0xA1);
    save_unconfirmed_spend(
        &mut *store.write().await,
        id,
        SPEND,
        &single_input_spend(),
        &[input],
    );

    // The second device's spend of the same note: the same nullifier, with
    // the commitment of a note that device created.
    let result = scan_pool(&store, vec![scanned(input, cmx(9))]).await;

    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
    assert_eq!(
        reported(&result, id, SPEND),
        vec![ShieldedActivityStatus::Failed],
        "the host is told"
    );
    assert_eq!(
        result.changeset.nullifiers_spent.get(&id),
        Some(&vec![input]),
        "the note is spent all the same"
    );
    assert!(unspent(&store, id).is_empty());
    assert!(
        store.pending_redrives(id).expect("records").is_empty(),
        "its transition is never broadcast again"
    );
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// The same after a restart: the reopened store restores the saved
/// transition, the host restores the note and the pending row, and the scan
/// that then reads the competing action fails the row. A later restart finds
/// nothing saved.
#[tokio::test]
async fn should_fail_a_restored_pending_spend_whose_single_input_another_transition_spent() {
    let path = temp_tree_path("restart");
    let (id, _) = subwallet();
    let input = nullifier(0xA1);
    {
        let store = open(&path);
        save_unconfirmed_spend(
            &mut *store.write().await,
            id,
            SPEND,
            &single_input_spend(),
            &[input],
        );
    }

    let store = open(&path);
    {
        let mut store = store.write().await;
        assert_eq!(
            store.pending_redrives(id).expect("records").len(),
            1,
            "the saved transition survives the restart"
        );
        restore_notes_and_row(&mut store, id, SPEND, &[input]);
        assert!(
            unspent(&store, id).is_empty(),
            "the restored record reserves its note"
        );
    }

    let result = scan_pool(&store, vec![scanned(input, cmx(9))]).await;

    {
        let store = store.read().await;
        assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
        assert_eq!(
            reported(&result, id, SPEND),
            vec![ShieldedActivityStatus::Failed]
        );
        assert!(store.pending_redrives(id).expect("records").is_empty());
    }
    drop(store);
    let reopened = open(&path);
    assert!(
        reopened
            .read()
            .await
            .pending_redrives(id)
            .expect("records")
            .is_empty(),
        "nothing is left to restore"
    );
    drop(reopened);
    let _ = std::fs::remove_file(&path);
}

/// A restored spend whose input note the store does not hold: the reopened
/// store restores the saved transition, but the host has not restored the
/// note, so the spent mark finds nothing to mark and drops no record. The
/// pass is also short of coverage, so no reservation is released either.
/// The scan still fails the row and drops the transition itself, so nothing
/// broadcasts it again after another restart.
#[tokio::test]
async fn should_drop_the_transition_of_a_failed_spend_whose_note_the_store_does_not_hold() {
    let path = temp_tree_path("absent_note");
    let (id, _) = subwallet();
    let input = nullifier(0xA1);
    {
        let store = open(&path);
        save_unconfirmed_spend(
            &mut *store.write().await,
            id,
            SPEND,
            &single_input_spend(),
            &[input],
        );
    }

    let store = open(&path);
    {
        let mut store = store.write().await;
        store
            .save_activity(id, &pending_row(SPEND, &[input]))
            .expect("restore the row alone");
        assert!(store.get_all_notes(id).expect("notes").is_empty());
        assert_eq!(store.pending_redrives(id).expect("records").len(), 1);
    }

    // The competing action is in a range proved at 1000. The empty tail is
    // proved at 999, and carries the highest count seen, as the SDK's stream
    // emits it: no batch ends at its count, so the pass covers no height.
    let result = scan(
        &store,
        futures::stream::iter(vec![
            Ok(ShieldedChunkBatch {
                start_index: 0,
                notes: vec![scanned(input, cmx(9))],
                decrypted: vec![],
                block_height: 1_000,
                is_partial: false,
                total_count: 2,
            }),
            Ok(ShieldedChunkBatch {
                start_index: 1,
                notes: vec![],
                decrypted: vec![],
                block_height: 999,
                is_partial: true,
                total_count: 2,
            }),
        ]),
    )
    .await;

    assert_eq!(result.covered_height, None);
    assert_eq!(
        reported(&result, id, SPEND),
        vec![ShieldedActivityStatus::Failed]
    );
    assert!(store
        .read()
        .await
        .pending_redrives(id)
        .expect("records")
        .is_empty());
    drop(store);
    let reopened = open(&path);
    assert!(
        reopened
            .read()
            .await
            .pending_redrives(id)
            .expect("records")
            .is_empty(),
        "no saved transition is restored"
    );
    drop(reopened);
    let _ = std::fs::remove_file(&path);
}

/// The competing transition spent both inputs, in actions on either side of
/// a range boundary, and the second range is proved a block before it
/// executed, so it holds nothing. The pass reads only the first action. The
/// row fails on it, but the second note is not released: no batch reaches
/// the tree's end, so the scan covers no height, least of all the one that
/// proved the conflict, and cannot show the note unspent. The next pass reads the second action and marks the note spent.
#[tokio::test]
async fn should_hold_the_other_notes_until_the_scan_covers_the_competing_spend() {
    let path = temp_tree_path("stale_tail");
    let store = open(&path);
    let (id, _) = subwallet();
    let (first, second) = (nullifier(0xA1), nullifier(0xA2));
    save_unconfirmed_spend(
        &mut *store.write().await,
        id,
        SPEND,
        &transfer(vec![action(first, cmx(1)), action(second, cmx(2))]),
        &[first, second],
    );

    // The first range, proved at 1000, where the tree holds both actions.
    let fresh_head = ShieldedChunkBatch {
        start_index: 0,
        notes: vec![scanned(first, cmx(8))],
        decrypted: vec![],
        block_height: 1_000,
        is_partial: false,
        total_count: 2,
    };
    // The second range, proved at 999, where the tree ends before it. It
    // carries the highest count seen, as the SDK's stream emits it.
    let stale_tail = ShieldedChunkBatch {
        start_index: 1,
        notes: vec![],
        decrypted: vec![],
        block_height: 999,
        is_partial: true,
        total_count: 2,
    };
    let result = scan(
        &store,
        futures::stream::iter(vec![Ok(fresh_head), Ok(stale_tail)]),
    )
    .await;
    assert_eq!(result.covered_height, None);
    {
        let store = store.read().await;
        assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
        assert!(
            unspent(&store, id).is_empty(),
            "the second note stays reserved"
        );
        assert!(
            store.pending_redrives(id).expect("records").is_empty(),
            "its transition is gone all the same"
        );
        assert_eq!(store.spendable_balance(id).expect("balance"), 0);
    }

    let result = scan_pool(
        &store,
        vec![scanned(first, cmx(8)), scanned(second, cmx(9))],
    )
    .await;

    assert_eq!(
        result.changeset.nullifiers_spent.get(&id),
        Some(&vec![second]),
        "the second note was spent all along"
    );
    assert!(unspent(&*store.read().await, id).is_empty());
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// Several inputs, one spent by another transition: the row fails and the
/// untouched note is spendable again in the same pass, while a note another
/// attempt holds stays held.
#[tokio::test]
async fn should_free_the_untouched_notes_of_a_spend_another_transition_took_an_input_of() {
    let path = temp_tree_path("several");
    let store = open(&path);
    let (id, _) = subwallet();
    let (taken, untouched, others) = (nullifier(0xA1), nullifier(0xA2), nullifier(0xA3));
    {
        let mut store = store.write().await;
        save_unconfirmed_spend(
            &mut store,
            id,
            SPEND,
            &transfer(vec![action(taken, cmx(1)), action(untouched, cmx(2))]),
            &[taken, untouched],
        );
        save_unconfirmed_spend(
            &mut store,
            id,
            OTHER_SPEND,
            &transfer(vec![
                action(others, cmx(3)),
                action(nullifier(0xD0), cmx(4)),
            ]),
            &[others],
        );
        assert!(unspent(&store, id).is_empty(), "every note is reserved");
    }

    scan_pool(&store, vec![scanned(taken, cmx(9))]).await;

    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
    assert_eq!(
        unspent(&store, id),
        vec![untouched],
        "the untouched note is selectable, the taken one spent, the other attempt's held"
    );
    assert_eq!(store.spendable_balance(id).expect("balance"), NOTE_VALUE);
    assert_eq!(
        status(&store, id, OTHER_SPEND),
        ShieldedActivityStatus::Pending
    );
    let records = store.pending_redrives(id).expect("records");
    assert_eq!(records.len(), 1, "the other attempt keeps its transition");
    assert_eq!(records[0].activity_id, OTHER_SPEND);
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// The pass does cover a height, but one below the height that proved the
/// conflict: a first range, proved at 999, ends at the tree's end as it was
/// then. The range after it is proved at 1000, with the first competing
/// action and a count beyond its end, and the empty tail is proved at 999
/// again. Coverage through 999 says nothing of a transition that executed at
/// 1000, so the second note stays reserved.
#[tokio::test]
async fn should_hold_the_other_notes_when_the_scan_covers_only_a_lower_height() {
    let path = temp_tree_path("lower_coverage");
    let store = open(&path);
    let (id, _) = subwallet();
    let (first, second) = (nullifier(0xA1), nullifier(0xA2));
    save_unconfirmed_spend(
        &mut *store.write().await,
        id,
        SPEND,
        &transfer(vec![action(first, cmx(1)), action(second, cmx(2))]),
        &[first, second],
    );

    let batch = |start_index: u64,
                 notes: Vec<ShieldedEncryptedNote>,
                 block_height: u64,
                 total_count: u64,
                 is_partial: bool| ShieldedChunkBatch {
        start_index,
        notes,
        decrypted: vec![],
        block_height,
        is_partial,
        total_count,
    };
    let result = scan(
        &store,
        futures::stream::iter(vec![
            // Another wallet's action: the whole tree at 999.
            Ok(batch(
                0,
                vec![scanned(nullifier(0x0F), cmx(5))],
                999,
                1,
                false,
            )),
            // The first competing action, at 1000, where the tree holds two
            // more actions.
            Ok(batch(1, vec![scanned(first, cmx(8))], 1_000, 3, false)),
            // The tail, proved at 999, with the highest count seen.
            Ok(batch(2, vec![], 999, 3, true)),
        ]),
    )
    .await;

    assert_eq!(result.covered_height, Some(999));
    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
    assert!(
        unspent(&store, id).is_empty(),
        "the second note stays reserved"
    );
    assert_eq!(store.spendable_balance(id).expect("balance"), 0);
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// Every input spent by one other transition, as when both devices select
/// the same notes: no input is left unspent to tell the two spends apart, and
/// the actions still do.
#[tokio::test]
async fn should_fail_a_pending_spend_whose_every_input_another_transition_spent() {
    let path = temp_tree_path("every");
    let store = open(&path);
    let (id, _) = subwallet();
    let (first, second) = (nullifier(0xA1), nullifier(0xA2));
    save_unconfirmed_spend(
        &mut *store.write().await,
        id,
        SPEND,
        &transfer(vec![action(first, cmx(1)), action(second, cmx(2))]),
        &[first, second],
    );

    scan_pool(
        &store,
        vec![scanned(first, cmx(8)), scanned(second, cmx(9))],
    )
    .await;

    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
    assert!(unspent(&store, id).is_empty(), "both notes are spent");
    assert!(store.pending_redrives(id).expect("records").is_empty());
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// The spend's own action, read before the action that holds its change:
/// the input is spent by the saved transition itself, so the row is not
/// failed. It stays pending for the scan to confirm from its change.
#[tokio::test]
async fn should_keep_a_pending_spend_pending_when_the_scan_reads_its_own_action() {
    let path = temp_tree_path("own");
    let store = open(&path);
    let (id, _) = subwallet();
    let (first, second) = (nullifier(0xA1), nullifier(0xA2));
    save_unconfirmed_spend(
        &mut *store.write().await,
        id,
        SPEND,
        &transfer(vec![action(first, cmx(1)), action(second, cmx(2))]),
        &[first, second],
    );

    let result = scan_pool(&store, vec![scanned(first, cmx(1))]).await;

    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Pending);
    assert!(reported(&result, id, SPEND).is_empty());
    assert!(
        unspent(&store, id).is_empty(),
        "the first note is spent and the second stays reserved for the landed spend"
    );
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// A note of the spend that another attempt holds by now is not released
/// with it.
#[tokio::test]
async fn should_keep_a_note_another_attempt_reserved_since() {
    let path = temp_tree_path("rearmed");
    let store = open(&path);
    let (id, _) = subwallet();
    let (taken, rearmed) = (nullifier(0xA1), nullifier(0xA2));
    {
        let mut store = store.write().await;
        save_unconfirmed_spend(
            &mut store,
            id,
            SPEND,
            &transfer(vec![action(taken, cmx(1)), action(rearmed, cmx(2))]),
            &[taken, rearmed],
        );
        store
            .set_pending_spend(id, &rearmed, ANCHOR, OTHER_SPEND, Some(6))
            .expect("the other attempt takes the reservation");
    }

    scan_pool(&store, vec![scanned(taken, cmx(9))]).await;

    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
    assert!(
        unspent(&store, id).is_empty(),
        "the other attempt's note stays reserved"
    );
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// A pass cut short publishes nothing, the competing action included: the
/// row stays pending with its transition saved, and the next pass, which
/// reads the action again, fails it.
#[tokio::test]
async fn should_fail_the_spend_on_the_pass_after_a_cancelled_one() {
    let path = temp_tree_path("cancelled");
    let store = open(&path);
    let (id, _) = subwallet();
    let input = nullifier(0xA1);
    save_unconfirmed_spend(
        &mut *store.write().await,
        id,
        SPEND,
        &single_input_spend(),
        &[input],
    );

    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let cut_short = futures::stream::iter(vec![Ok(pool(vec![scanned(input, cmx(9))]))]).chain(
        futures::stream::once(async move {
            stop.cancel();
            futures::future::pending::<BatchItem>().await
        }),
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sync_notes_from_stream(&store, &[subwallet()], None, &cancel, move |_, _| cut_short),
    )
    .await
    .expect("the pass returns")
    .expect("the pass succeeds");
    {
        let store = store.read().await;
        assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Pending);
        assert_eq!(store.pending_redrives(id).expect("records").len(), 1);
    }

    scan_pool(&store, vec![scanned(input, cmx(9))]).await;

    let store = store.read().await;
    assert_eq!(status(&store, id, SPEND), ShieldedActivityStatus::Failed);
    assert!(store.pending_redrives(id).expect("records").is_empty());
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// A token purchase is told apart by its fee bundle, the bundle the credit
/// pool records: its own fee action leaves it pending, another transition's
/// spend of its fee note fails it.
#[tokio::test]
async fn should_tell_a_token_purchase_s_own_fee_action_from_another_spend_of_its_fee_note() {
    let (id, _) = subwallet();
    let fee_note = nullifier(0xA1);
    let saved = purchase(vec![
        action(fee_note, cmx(1)),
        action(nullifier(0xD0), cmx(2)),
    ]);
    for (tag, scanned_cmx, expected) in [
        ("purchase_own", cmx(1), ShieldedActivityStatus::Pending),
        ("purchase_other", cmx(9), ShieldedActivityStatus::Failed),
    ] {
        let path = temp_tree_path(tag);
        let store = open(&path);
        save_unconfirmed_spend(&mut *store.write().await, id, SPEND, &saved, &[fee_note]);

        scan_pool(&store, vec![scanned(fee_note, scanned_cmx)]).await;

        assert_eq!(status(&*store.read().await, id, SPEND), expected, "{tag}");
        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
