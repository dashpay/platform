//! An affected-state result that arrives after a spend was proved failed
//! (dashpay/platform#6160).
//!
//! An identity top-up (type 22) and a private document create (type 29) get
//! their result as a snapshot: the spend's nullifiers read spent, which does
//! not say which transition spent them. While such a spend waits, the note
//! scan can read another transition's action for one of its inputs and fail
//! its row on that proof, or its anchor can be proven pruned. A snapshot that
//! arrives afterwards must leave the row failed and report the spend as not
//! executed. The notes it shows spent are marked spent either way.

use super::*;
use crate::wallet::shielded::activity::ShieldedActivityEntry;
use crate::wallet::shielded::store::InMemoryShieldedStore;
use crate::wallet::shielded::sync::sync_notes_from_stream;
use dash_sdk::platform::shielded::notes_sync::types::ShieldedChunkBatch;
use dpp::shielded::SerializedAction;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use tokio_util::sync::CancellationToken;

type Store = Arc<RwLock<InMemoryShieldedStore>>;

const WALLET: WalletId = [0x68; 32];
/// The spend's activity row.
const ROW: [u8; 32] = [0x77; 32];
const ANCHOR: [u8; 32] = [0x52; 32];
const TOP_UP: &str = "identity top up from shielded pool";

fn subwallet() -> (SubwalletId, AccountViewingKeys) {
    let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
        .expect("derive viewing keys")
        .viewing_keys();
    (SubwalletId::new(WALLET, 0), views)
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

/// A private document create whose bundle spends each of `inputs` in an
/// action of its own and holds no padding action: the case in which another
/// transition can spend every one of its nullifiers.
fn payment(inputs: &[[u8; 32]]) -> StateTransition {
    let actions = (1u8..)
        .zip(inputs)
        .map(|(n, nullifier)| SerializedAction {
            nullifier: *nullifier,
            rk: [2; 32],
            cmx: cmx(n),
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        })
        .collect();
    PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
        Identifier::from([0x44; 32]),
        "note".to_string(),
        vec![BinaryData::new(vec![9; 64])],
        actions,
        0,
        ANCHOR,
        vec![8; 64],
        [9; 64],
        PlatformVersion::latest(),
    )
    .expect("a type 29 transition")
}

fn pending_row(inputs: &[[u8; 32]]) -> ShieldedActivityEntry {
    ShieldedActivityEntry {
        id: ROW,
        kind: ShieldedActivityKind::ShieldedSpend,
        direction: ShieldedDirection::Out,
        amount: 0,
        fee: Some(10),
        counterparty: None,
        memo: None,
        block_height: None,
        status: ShieldedActivityStatus::Pending,
        created_at_ms: 1,
        min_note_position: None,
        note_cmxs: vec![[0x7C; 32]],
        spent_nullifiers: inputs.to_vec(),
    }
}

/// The store as a spend of `inputs` leaves it once it selected its notes and
/// recorded its row: the notes reserved, the row pending.
async fn reserved_spend(
    store: &Store,
    id: SubwalletId,
    inputs: &[[u8; 32]],
) -> (Vec<ShieldedNote>, NoteReservation<InMemoryShieldedStore>) {
    let notes: Vec<ShieldedNote> = (100u64..)
        .zip(inputs)
        .map(|(position, nullifier)| ShieldedNote {
            position,
            cmx: [0x2A; 32],
            nullifier: *nullifier,
            block_height: 10,
            is_spent: false,
            value: 1_000,
            note_data: vec![0u8; 115],
        })
        .collect();
    let mut locked = store.write().await;
    for note in &notes {
        locked.save_note(id, note).expect("save the note");
    }
    locked
        .save_activity(id, &pending_row(inputs))
        .expect("save the row");
    let reservation = mark_notes_pending(&mut *locked, store, id, &notes).expect("reserved");
    (notes, reservation)
}

/// A scan pass that reads the whole pool, one action: another transition's
/// spend of the note with `nullifier`, creating a note of that transition's.
async fn scan_a_competing_spend(
    store: &Store,
    subwallets: &[(SubwalletId, AccountViewingKeys)],
    nullifier: [u8; 32],
) {
    let pool = ShieldedChunkBatch {
        start_index: 0,
        notes: vec![ShieldedEncryptedNote {
            cmx: cmx(9).to_vec(),
            nullifier: nullifier.to_vec(),
            cv_net: vec![],
            encrypted_note: vec![],
        }],
        decrypted: vec![],
        block_height: 1_000,
        is_partial: true,
        total_count: 1,
    };
    sync_notes_from_stream(
        store,
        subwallets,
        None,
        &CancellationToken::new(),
        move |_, _| futures::stream::iter([Ok(pool)]),
    )
    .await
    .expect("the pass succeeds");
}

async fn row_status(store: &Store, id: SubwalletId) -> ShieldedActivityStatus {
    store
        .read()
        .await
        .get_activity_by_entry_id(id, &ROW)
        .expect("read the row")
        .expect("the row")
        .status
}

async fn unspent(store: &Store, id: SubwalletId) -> Vec<[u8; 32]> {
    let mut nullifiers: Vec<[u8; 32]> = store
        .read()
        .await
        .get_unspent_notes(id)
        .expect("unspent notes")
        .into_iter()
        .map(|note| note.nullifier)
        .collect();
    nullifiers.sort_unstable();
    nullifiers
}

/// Whether every note of `inputs` is marked spent, which a note that is only
/// reserved is not.
async fn all_spent(store: &Store, id: SubwalletId, inputs: &[[u8; 32]]) -> bool {
    let notes = store.read().await.get_all_notes(id).expect("notes");
    inputs.iter().all(|nullifier| {
        notes
            .iter()
            .any(|note| note.nullifier == *nullifier && note.is_spent)
    })
}

/// The snapshot a node answers the result wait with: every nullifier of the
/// spend reads spent.
fn snapshot(inputs: &[[u8; 32]]) -> (StateTransitionProofResult, ResponseMetadata) {
    (
        StateTransitionProofResult::VerifiedShieldedNullifiers(
            inputs
                .iter()
                .map(|nullifier| (nullifier.to_vec(), true))
                .collect(),
        ),
        ResponseMetadata::default(),
    )
}

/// The race of #6160 through the private document flow: the payment is saved
/// and broadcast, a scan pass reads the other device's spend of its first
/// note while it waits and fails its row, and then its result arrives as a
/// snapshot that shows both nullifiers spent. The row stays failed and the
/// payment is reported as not executed.
#[tokio::test]
async fn should_keep_a_payment_failed_when_its_result_arrives_after_the_scan_proved_it_failed() {
    let store: Store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
    let subwallets = [subwallet()];
    let id = subwallets[0].0;
    let inputs = [nullifier(0xA1), nullifier(0xA2)];
    let (selected, reservation) = reserved_spend(&store, id, &inputs).await;
    let started = PaymentStarted::default();

    let submitted = submit_private_document_payment(
        PrivateDocumentSubmission {
            store: &store,
            persister: None,
            wallet_id: WALLET,
            id,
            pending_entry: Some(pending_row(&inputs)),
            selected_notes: &selected,
            reservation,
            state_transition: payment(&inputs),
            anchor: ANCHOR,
            anchor_height: 5,
            private_contact: None,
            journaled: None,
            payment_started: &started,
        },
        |_| async {
            scan_a_competing_spend(&store, &subwallets, inputs[0]).await;
            assert_eq!(
                row_status(&store, id).await,
                ShieldedActivityStatus::Failed,
                "the scan proved the payment failed while it waited"
            );
            assert_eq!(
                unspent(&store, id).await,
                vec![inputs[1]],
                "the scan freed the note the other spend did not take"
            );
            Ok(snapshot(&inputs))
        },
    )
    .await;

    assert!(
        matches!(
            submitted,
            Err(PlatformWalletError::ShieldedSpendNotExecuted { .. })
        ),
        "{submitted:?}"
    );
    assert_eq!(
        row_status(&store, id).await,
        ShieldedActivityStatus::Failed,
        "the late result does not confirm a payment proved failed"
    );
    assert!(
        all_spent(&store, id, &inputs).await,
        "the result proves every nullifier spent, whichever transition spent it"
    );
    assert!(store
        .read()
        .await
        .pending_redrives(id)
        .expect("records")
        .is_empty());
}

/// The settlement both spends share, for a top-up: a row that already reads
/// failed, its notes released as the release of an expired spend leaves them,
/// stays failed, and the snapshot still marks the notes spent. The release
/// itself is driven in
/// `coordinator::tests::should_fail_an_expired_spend_s_row_before_a_waiting_result_can_confirm_it`.
#[tokio::test]
async fn should_report_a_top_up_as_not_executed_when_its_row_already_reads_failed() {
    let store: Store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
    let (id, _) = subwallet();
    let inputs = [nullifier(0xA1), nullifier(0xA2)];
    let (selected, reservation) = reserved_spend(&store, id, &inputs).await;
    reservation.cancel().await;
    record_activity_status_by_id(
        &store,
        None,
        WALLET,
        id,
        &ROW,
        ShieldedActivityStatus::Failed,
    )
    .await;
    assert_eq!(unspent(&store, id).await, inputs.to_vec());

    let settled = settle_affected_state_spend(
        &store,
        None,
        WALLET,
        id,
        &Some(pending_row(&inputs)),
        &selected,
        TOP_UP,
    )
    .await;

    assert!(
        matches!(
            settled,
            Err(PlatformWalletError::ShieldedSpendNotExecuted { operation }) if operation == TOP_UP
        ),
        "{settled:?}"
    );
    assert_eq!(row_status(&store, id).await, ShieldedActivityStatus::Failed);
    assert!(all_spent(&store, id, &inputs).await);
}

/// Nothing contradicts the snapshot: the spend is confirmed at once, without
/// waiting for a scan, and its notes are spent.
#[tokio::test]
async fn should_confirm_a_top_up_at_once_when_nothing_proved_it_failed() {
    let store: Store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
    let (id, _) = subwallet();
    let inputs = [nullifier(0xA1), nullifier(0xA2)];
    let (selected, mut reservation) = reserved_spend(&store, id, &inputs).await;
    reservation.hand_over();

    settle_affected_state_spend(
        &store,
        None,
        WALLET,
        id,
        &Some(pending_row(&inputs)),
        &selected,
        TOP_UP,
    )
    .await
    .expect("the top-up is confirmed");

    assert_eq!(
        row_status(&store, id).await,
        ShieldedActivityStatus::Confirmed
    );
    assert!(all_spent(&store, id, &inputs).await);
}
