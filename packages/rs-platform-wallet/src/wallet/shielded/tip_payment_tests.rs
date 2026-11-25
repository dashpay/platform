//! Shielded tips: the amount parser, whom an unresolved transfer pays as
//! read back from its saved transition, the confirmed recipients a wallet
//! keeps, and the refusals of a tip send.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use dashcore::Network;
use dpp::address_funds::OrchardAddress;
use dpp::address_funds::PlatformAddress;
use dpp::identity::{Identity, IdentityV0};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::serialization::{PlatformDeserializableTrusted, PlatformSerializable};
use dpp::shielded::builder::{build_shielded_transfer_transition, SpendableNote};
use dpp::shielded::SerializedAction;
use dpp::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Hashable, MerkleHashOrchard, MerklePath, Note,
    NoteValue, Nullifier, OrchardDomain, OutgoingViewingKey, PaymentAddress, RandomSeed, Rho,
    ValueCommitTrapdoor, ValueCommitment, NOTE_COMMITMENT_TREE_DEPTH,
};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use super::activity::compute_activity_id;
use super::activity_recorder::visible_output_cmxs;
use super::keys::{AccountViewingKeys, OrchardKeySet};
use super::operations::shielded_actions;
use super::prover::CachedOrchardProver;
use super::store::{PendingRedrive, ShieldedNote, ShieldedStore, SubwalletId};
use super::tip_payment::{
    ensure_expected_identity, ensure_recipient_unchanged, parse_shielded_tip_amount,
    pays_unresolved, recipient_change, resolve_recipient, tip_recipient_label,
    unresolved_transfer_recipients, ShieldedIdentityBalance, ShieldedTipRecipient,
    ShieldedTipRecipientChange, ShieldedTipSource,
};
use super::viewing_key_bind_tests::{
    add_identity_at_slot, coordinator_at, platform_wallet_with, seed_master_key, temp_dir,
    tip_race_fixture, CapturingPersistence, BIND_SEED,
};
use super::{shielded_identity_account_index, FileBackedShieldedStore};
use crate::{PaymentDelivery, PlatformWalletError};

/// The Orchard note encryptor with Dash's 36-byte memo.
type OrchardNoteEncryption = grovedb_commitment_tree::OrchardNoteEncryption<DashMemo>;

const WALLET: [u8; 32] = [7; 32];

fn keys_of(seed_byte: u8, account: u32) -> AccountViewingKeys {
    OrchardKeySet::from_seed(&[seed_byte; 64], Network::Testnet, account)
        .expect("derive keys")
        .viewing_keys()
}

fn random_bytes(rng: &mut StdRng) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    bytes
}

/// The randomness of one output, seeded from what the output is (whom it
/// pays, and whether the paying account's key opens it), so a failing test
/// builds the same bytes when it is run again.
fn output_rng(recipient: &PaymentAddress, keyed: bool) -> StdRng {
    let raw = recipient.to_raw_address_bytes();
    let mut seed = [0u8; 8];
    seed.copy_from_slice(&raw[..8]);
    StdRng::seed_from_u64(u64::from_le_bytes(seed) ^ u64::from(keyed))
}

/// One action of a transfer as the builder emits it: an output note to
/// `recipient`, whose outgoing ciphertext is keyed to `ovk`. `None` is a
/// padding output, keyed to a random key that nothing opens.
fn output_to(recipient: PaymentAddress, ovk: Option<OutgoingViewingKey>) -> SerializedAction {
    let mut rng = output_rng(&recipient, ovk.is_some());
    // The output note's rho is the nullifier of the note the action spends.
    let (nullifier, rho) = loop {
        let bytes = random_bytes(&mut rng);
        if let (Some(nullifier), Some(rho)) = (
            Nullifier::from_bytes(&bytes).into_option(),
            Rho::from_bytes(&bytes).into_option(),
        ) {
            break (nullifier, rho);
        }
    };
    let rseed = loop {
        if let Some(rseed) = RandomSeed::from_bytes(random_bytes(&mut rng), &rho).into_option() {
            break rseed;
        }
    };
    let value = NoteValue::from_raw(1_000);
    let note = Note::from_parts(recipient, value, rho, rseed)
        .into_option()
        .expect("valid note parts");
    let cmx = ExtractedNoteCommitment::from(note.commitment());
    let rcv = loop {
        if let Some(rcv) = ValueCommitTrapdoor::from_bytes(random_bytes(&mut rng)).into_option() {
            break rcv;
        }
    };
    let cv = ValueCommitment::derive(value - NoteValue::from_raw(0), rcv);
    let encryption = OrchardNoteEncryption::new(ovk, note, [0u8; 36]);
    let epk = OrchardDomain::<DashMemo>::epk_bytes(encryption.epk());
    let enc = encryption.encrypt_note_plaintext();
    let out = encryption.encrypt_outgoing_plaintext(&cv, &cmx, &mut rng);
    let mut encrypted_note = Vec::with_capacity(216);
    encrypted_note.extend_from_slice(&epk.0);
    encrypted_note.extend_from_slice(enc.as_ref());
    encrypted_note.extend_from_slice(&out);
    SerializedAction {
        nullifier: nullifier.to_bytes(),
        rk: [0; 32],
        cmx: cmx.to_bytes(),
        encrypted_note,
        cv_net: cv.to_bytes(),
        spend_auth_sig: [0; 64],
    }
}

/// The saved record of a spend whose signed transition is `transition`,
/// consuming one note per action.
fn record_of(
    activity: u8,
    transition: StateTransition,
    actions: &[SerializedAction],
) -> PendingRedrive {
    PendingRedrive {
        activity_id: [activity; 32],
        anchor: [0; 32],
        nullifiers: actions.iter().map(|action| action.nullifier).collect(),
        st_bytes: transition.serialize_to_bytes().expect("serialize"),
        attempts: 0,
        anchor_height: None,
        identity_nonce_finalized: false,
        identity_user_abandoned: false,
    }
}

/// The saved record of a shielded transfer with `actions`.
fn transfer_record(activity: u8, actions: Vec<SerializedAction>) -> PendingRedrive {
    let transition = StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
        ShieldedTransferTransitionV0 {
            actions: actions.clone(),
            value_balance: 0,
            anchor: [0; 32],
            proof: Vec::new(),
            binding_signature: [0; 64],
        },
    ));
    record_of(activity, transition, &actions)
}

/// The id a spend with `actions` is saved under: the hash of the outputs its
/// account reads, as the live recorder computes it.
fn saved_id(actions: &[SerializedAction], payer: &AccountViewingKeys) -> [u8; 32] {
    compute_activity_id(&visible_output_cmxs(actions, payer))
}

/// The saved record of the transfer with `actions`, under the id `payer`'s
/// spend saves it with.
fn saved_transfer(actions: Vec<SerializedAction>, payer: &AccountViewingKeys) -> PendingRedrive {
    PendingRedrive {
        activity_id: saved_id(&actions, payer),
        ..transfer_record(0, actions)
    }
}

/// The saved record of a transfer from `payer` to `recipient`, with the
/// change it returns to `payer`'s own default address.
fn transfer_paying(payer: &AccountViewingKeys, recipient: PaymentAddress) -> PendingRedrive {
    let ovk = payer.outgoing_viewing_key.clone();
    saved_transfer(
        vec![
            output_to(recipient, Some(ovk.clone())),
            output_to(payer.default_address, Some(ovk)),
        ],
        payer,
    )
}

/// The note `nullifier` spends, as the wallet holds it before the spend.
fn input_note(nullifier: [u8; 32]) -> ShieldedNote {
    ShieldedNote {
        position: 0,
        cmx: nullifier,
        nullifier,
        block_height: 1,
        is_spent: false,
        value: 1_000,
        note_data: vec![0; 115],
    }
}

/// Save `record` as a spend does: its input notes are held and reserved
/// before the record is saved.
fn arm_spend(store: &mut FileBackedShieldedStore, id: SubwalletId, record: PendingRedrive) {
    for nullifier in &record.nullifiers {
        store.save_note(id, &input_note(*nullifier)).expect("save");
        store.mark_pending(id, nullifier).expect("reserve");
    }
    store.arm_redrive(id, record).expect("arm");
}

fn store_at(tag: &str) -> (FileBackedShieldedStore, PathBuf) {
    let dir = temp_dir(tag);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("tree.sqlite");
    (
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
        path,
    )
}

fn recipient(identity: u8, address: [u8; 43]) -> ShieldedTipRecipient {
    ShieldedTipRecipient {
        identity_id: Identifier::from([identity; 32]),
        address,
    }
}

#[test]
fn should_parse_exact_tip_amounts_and_refuse_ambiguous_input() {
    for (text, credits) in [
        ("1", 100_000_000_000),
        ("0.5", 50_000_000_000),
        ("0.00000000001", 1),
        ("2.10000000000000", 210_000_000_000),
        ("1,5", 150_000_000_000),
        (".5", 50_000_000_000),
        (" 1 ", 100_000_000_000),
        ("184467440", 18_446_744_000_000_000_000),
        // The last credit a u64 holds.
        ("184467440.73709551615", u64::MAX),
    ] {
        assert_eq!(
            parse_shielded_tip_amount(text).unwrap(),
            credits,
            "{text:?}"
        );
    }
    for refused in [
        "",
        " ",
        "0",
        "0.0",
        ".",
        "1.",
        "-1",
        "+1",
        "1e3",
        "1 000",
        "1,000.5",
        "1.000,5",
        // Finer than one credit.
        "0.000000000001",
        // Past u64 credits, by the whole part and by the fraction.
        "184467441",
        "184467440.73709551616",
        "1.2.3",
        // Arabic-Indic three: a digit, but not an ASCII one.
        "\u{663}",
    ] {
        assert!(
            matches!(
                parse_shielded_tip_amount(refused),
                Err(PlatformWalletError::InvalidParameter(_))
            ),
            "{refused:?}"
        );
    }
}

#[test]
fn should_key_confirmed_recipients_by_the_resolved_dpns_label() {
    assert_eq!(
        tip_recipient_label("Alice"),
        tip_recipient_label("a11ce.DASH")
    );
    assert_eq!(
        tip_recipient_label(" alice.dash "),
        tip_recipient_label("alice")
    );
    assert_ne!(tip_recipient_label("alice"), tip_recipient_label("bob"));
}

#[test]
fn should_tell_a_new_an_unchanged_and_a_changed_recipient() {
    let current = recipient(1, [4; 43]);
    assert_eq!(
        recipient_change(None, &current),
        ShieldedTipRecipientChange::New
    );
    assert_eq!(
        recipient_change(Some(current.clone()), &current),
        ShieldedTipRecipientChange::Unchanged
    );
    // The name moved to another identity, or its profile to another address.
    for previous in [recipient(2, [4; 43]), recipient(1, [5; 43])] {
        assert_eq!(
            recipient_change(Some(previous.clone()), &current),
            ShieldedTipRecipientChange::Changed { previous }
        );
    }
}

/// An earlier payment to the address a name used to publish still blocks a
/// tip to the address it publishes now: the recipient may have replaced its
/// address after that payment was sent.
#[test]
fn should_count_an_unresolved_payment_to_the_address_last_confirmed_for_the_name() {
    let unresolved = BTreeSet::from([[4u8; 43]]);
    let (old, new) = (recipient(1, [4; 43]), recipient(1, [5; 43]));
    assert!(pays_unresolved(&unresolved, &old, None));
    assert!(!pays_unresolved(&unresolved, &new, None));
    assert!(pays_unresolved(&unresolved, &new, Some(&old)));
    assert!(!pays_unresolved(&BTreeSet::new(), &new, Some(&old)));
}

/// The recipient of an unresolved transfer is read from its saved
/// transition, across a restart, until the transfer resolves. Its own change
/// is not a payment, and another wallet's transfers do not count.
#[test]
fn should_read_whom_an_unresolved_transfer_pays_from_its_saved_transition() {
    let (mut store, path) = store_at("tip_unresolved_recipients");
    let id = SubwalletId::new(WALLET, 0);
    let payer = keys_of(1, 0);
    let keys = BTreeMap::from([(0, payer.clone())]);
    let paid = keys_of(2, 0).default_address;
    let record = transfer_paying(&payer, paid);
    let spent = record.nullifiers[0];
    arm_spend(&mut store, id, record);

    let expected = BTreeSet::from([paid.to_raw_address_bytes()]);
    assert_eq!(
        unresolved_transfer_recipients(&store, WALLET, &keys).unwrap(),
        expected
    );
    assert!(unresolved_transfer_recipients(&store, [9; 32], &keys)
        .unwrap()
        .is_empty());

    // A restart restores the record, and the answer with it.
    drop(store);
    let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
    assert_eq!(
        unresolved_transfer_recipients(&store, WALLET, &keys).unwrap(),
        expected
    );

    // Seen executed: the scan marks an input spent, which drops the record.
    // The notes themselves are the host's to restore after a restart.
    store.save_note(id, &input_note(spent)).expect("restore");
    assert!(store.mark_spent(id, &spent).expect("mark spent"));
    assert!(unresolved_transfer_recipients(&store, WALLET, &keys)
        .unwrap()
        .is_empty());
}

/// A transfer whose anchor is proven pruned can never execute: the release
/// of its notes drops its record, and it no longer blocks a tip.
#[test]
fn should_stop_counting_a_transfer_released_as_unable_to_execute() {
    let (mut store, _) = store_at("tip_released_transfer");
    let id = SubwalletId::new(WALLET, 0);
    let payer = keys_of(1, 0);
    let keys = BTreeMap::from([(0, payer.clone())]);
    let record = transfer_paying(&payer, keys_of(2, 0).default_address);
    let released = record.nullifiers[0];
    arm_spend(&mut store, id, record);
    assert_eq!(
        unresolved_transfer_recipients(&store, WALLET, &keys)
            .unwrap()
            .len(),
        1
    );
    assert!(store.clear_pending(id, &released).expect("release"));
    assert!(unresolved_transfer_recipients(&store, WALLET, &keys)
        .unwrap()
        .is_empty());
}

/// Only a transfer pays an Orchard address of the credit pool: an unshield
/// pays a Platform address, and an identity-funded shield pays this wallet.
#[test]
fn should_not_count_spends_that_pay_no_shielded_address() {
    let (mut store, _) = store_at("tip_other_spend_kinds");
    let id = SubwalletId::new(WALLET, 0);
    let payer = keys_of(1, 0);
    let keys = BTreeMap::from([(0, payer.clone())]);

    let actions = vec![output_to(
        payer.default_address,
        Some(payer.outgoing_viewing_key.clone()),
    )];
    let unshield = StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
        output_address: PlatformAddress::P2pkh([1; 20]),
        actions: actions.clone(),
        unshielding_amount: 1,
        anchor: [0; 32],
        proof: Vec::new(),
        binding_signature: [0; 64],
    }));
    store
        .arm_redrive(id, record_of(1, unshield, &actions))
        .expect("arm unshield");
    // An identity-funded shield consumes no note, and pays this wallet.
    let identity_shield: StateTransition = ShieldFromIdentityTransitionV0 {
        identity_id: [0xB1; 32].into(),
        amount: 1_000,
        actions: Vec::new(),
        anchor: [0; 32],
        proof: Vec::new(),
        binding_signature: [0; 64],
        nonce: 7,
        user_fee_increase: 0,
        signature_public_key_id: 1,
        signature: BinaryData::new(Vec::new()),
    }
    .into();
    store
        .arm_redrive(id, record_of(2, identity_shield, &[]))
        .expect("arm identity shield");

    assert!(unresolved_transfer_recipients(&store, WALLET, &keys)
        .unwrap()
        .is_empty());
}

/// A saved transfer whose recipient cannot be read blocks the check instead
/// of reading as no payment: its transition may still execute.
#[test]
fn should_fail_closed_when_a_saved_transfer_does_not_say_whom_it_pays() {
    let payer = keys_of(1, 0);
    let keys = BTreeMap::from([(0, payer.clone())]);
    let paid = keys_of(2, 0).default_address;
    let id = SubwalletId::new(WALLET, 0);

    // A transition that does not decode.
    let (mut store, _) = store_at("tip_undecodable_transfer");
    store
        .arm_redrive(
            id,
            PendingRedrive {
                st_bytes: vec![0xAB; 96],
                ..transfer_paying(&payer, paid)
            },
        )
        .expect("arm");
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(0),
            ..
        })
    ));

    // A transfer none of whose outputs opens under its account's key.
    let (mut store, _) = store_at("tip_unopenable_transfer");
    store
        .arm_redrive(id, transfer_record(1, vec![output_to(paid, None)]))
        .expect("arm");
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(0),
            ..
        })
    ));
    // The payment's ciphertext no longer opens while the change still does:
    // the outputs the account reads are not the ones the record was saved
    // with, so the change alone does not make it read as "pays nobody".
    let (mut store, _) = store_at("tip_damaged_recipient_output");
    let intact = transfer_paying(&payer, paid);
    let transition =
        StateTransition::deserialize_from_bytes_trusted(&intact.st_bytes).expect("decode");
    let mut actions = shielded_actions(&transition).to_vec();
    // epk(32) || enc_ciphertext(104) || out_ciphertext(80)
    for byte in &mut actions[0].encrypted_note[136..] {
        *byte ^= 0xFF;
    }
    assert_eq!(
        visible_output_cmxs(&actions, &payer),
        vec![actions[1].cmx],
        "only the change still opens"
    );
    store
        .arm_redrive(
            id,
            PendingRedrive {
                st_bytes: transfer_record(0, actions).st_bytes,
                ..intact
            },
        )
        .expect("arm");
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(0),
            ..
        })
    ));

    // A transfer whose record lost its note list is not an identity-funded
    // shield's record: it may still execute.
    let (mut store, _) = store_at("tip_transfer_without_nullifiers");
    store
        .arm_redrive(
            id,
            PendingRedrive {
                nullifiers: Vec::new(),
                ..transfer_paying(&payer, paid)
            },
        )
        .expect("arm");
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(0),
            ..
        })
    ));

    // The keys of another seed open nothing of an intact transfer either.
    let (mut store, _) = store_at("tip_keys_of_another_seed");
    store
        .arm_redrive(id, transfer_paying(&payer, paid))
        .expect("arm");
    assert_eq!(
        unresolved_transfer_recipients(&store, WALLET, &keys)
            .unwrap()
            .len(),
        1,
        "the record reads with its own account's keys"
    );
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &BTreeMap::from([(0, keys_of(3, 0))])),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(0),
            ..
        })
    ));

    // A record that lists no notes and does not decode: it may be a
    // transfer as well as an identity-funded shield.
    let (mut store, _) = store_at("tip_undecodable_without_nullifiers");
    store
        .arm_redrive(
            id,
            PendingRedrive {
                st_bytes: vec![0xAB; 96],
                nullifiers: Vec::new(),
                ..transfer_paying(&payer, paid)
            },
        )
        .expect("arm");
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(0),
            ..
        })
    ));

    // A transfer paid from an account whose keys are not bound.
    let (mut store, _) = store_at("tip_unbound_payer");
    store
        .arm_redrive(
            SubwalletId::new(WALLET, 5),
            transfer_paying(&keys_of(1, 5), paid),
        )
        .expect("arm");
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryKeysRequired {
            account_index: 5,
            ..
        })
    ));
}

/// A saved transfer whose row lost its note list is not loaded as a record
/// with no notes, which is what an identity-funded debit's record is: the
/// row is kept, no debit is listed for it, and tips fail closed.
#[test]
fn should_keep_unloaded_a_transfer_whose_row_lost_its_note_list() {
    let (mut store, path) = store_at("tip_row_without_nullifiers");
    let id = SubwalletId::new(WALLET, 0);
    let payer = keys_of(1, 0);
    let keys = BTreeMap::from([(0, payer.clone())]);
    arm_spend(
        &mut store,
        id,
        transfer_paying(&payer, keys_of(2, 0).default_address),
    );
    drop(store);
    let connection = rusqlite::Connection::open(&path).expect("open the store file");
    connection
        .execute("UPDATE shielded_pending_spends SET nullifiers = x''", [])
        .expect("empty the note list");

    let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
    assert!(store.has_unloaded_note_spends(WALLET));
    assert!(
        store
            .pending_redrives_for_wallet(WALLET)
            .expect("records")
            .is_empty(),
        "not loaded, so not listed as an identity-funded debit"
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count"),
        1,
        "the row is kept"
    );
    assert!(matches!(
        unresolved_transfer_recipients(&store, WALLET, &keys),
        Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: None,
            ..
        })
    ));
}

/// What the user confirmed for a name is kept per wallet, across a restart
/// and a shielded Clear, and goes with the wallet.
#[test]
fn should_keep_confirmed_recipients_across_a_restart_and_a_clear_until_the_wallet_is_removed() {
    let (mut store, path) = store_at("tip_confirmed_recipients");
    let (first, second) = (([1u8; 32], [4u8; 43]), ([2u8; 32], [5u8; 43]));
    assert_eq!(
        store.confirmed_tip_recipient(WALLET, "a11ce").unwrap(),
        None
    );
    store
        .confirm_tip_recipient(WALLET, "a11ce", &first)
        .expect("confirm");
    store
        .confirm_tip_recipient([9; 32], "a11ce", &second)
        .expect("confirm for another wallet");
    assert_eq!(
        store.confirmed_tip_recipient(WALLET, "a11ce").unwrap(),
        Some(first)
    );
    assert_eq!(store.confirmed_tip_recipient(WALLET, "b0b").unwrap(), None);

    // A later confirmation replaces the earlier one.
    store
        .confirm_tip_recipient(WALLET, "a11ce", &second)
        .expect("confirm again");
    drop(store);
    let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
    assert_eq!(
        store.confirmed_tip_recipient(WALLET, "a11ce").unwrap(),
        Some(second)
    );

    // What a shielded Clear purges.
    store.purge_all_subwallets().expect("purge subwallets");
    store.reset_commitment_tree().expect("reset tree");
    assert_eq!(
        store.confirmed_tip_recipient(WALLET, "a11ce").unwrap(),
        Some(second)
    );

    store.purge_wallet(WALLET).expect("purge wallet");
    assert_eq!(
        store.confirmed_tip_recipient(WALLET, "a11ce").unwrap(),
        None
    );
    assert_eq!(
        store.confirmed_tip_recipient([9; 32], "a11ce").unwrap(),
        Some(second),
        "another wallet's record stays"
    );
}

/// A tip requested while another is admitted is the same tip sent twice: it
/// is refused at once, before a seed is resolved. The admission of another
/// wallet does not admit this wallet's tip.
#[tokio::test]
async fn should_admit_one_tip_at_a_time_and_only_on_its_own_wallet() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tip_admission").await;
    let first = wallet.admit_shielded_tip().expect("admit the first tip");
    assert!(matches!(
        wallet.admit_shielded_tip(),
        Err(PlatformWalletError::ShieldedTipPending)
    ));
    // A clone of the handle is the same wallet.
    assert!(matches!(
        wallet.clone().admit_shielded_tip(),
        Err(PlatformWalletError::ShieldedTipPending)
    ));
    drop(first);
    drop(
        wallet
            .admit_shielded_tip()
            .expect("admitted once the first ended"),
    );

    let (other, _, _, _) = tip_race_fixture("tip_admission_other_wallet").await;
    let result = wallet
        .send_shielded_tip(
            other
                .admit_shielded_tip()
                .expect("admit on the other wallet"),
            &coordinator,
            &seed[..],
            ShieldedTipSource::Identity(id),
            "alice",
            &recipient(1, [4; 43]),
            None,
            1_000,
            [0; 36],
            &CachedOrchardProver::new(),
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::InvalidParameter(_))),
        "{result:?}"
    );
}

/// A tip started from a saved contact names the identity the name is
/// expected to belong to. A name that resolves to another identity is
/// refused, at the review and, before any wait or lookup, at the send. A
/// typed name expects nothing.
#[tokio::test]
async fn should_refuse_a_name_that_belongs_to_another_identity_than_expected() {
    let (contact, moved_to) = (Identifier::from([1; 32]), Identifier::from([2; 32]));
    assert!(ensure_expected_identity(None, &moved_to).is_ok());
    assert!(ensure_expected_identity(Some(&contact), &contact).is_ok());
    assert!(matches!(
        ensure_expected_identity(Some(&contact), &moved_to),
        Err(PlatformWalletError::ShieldedTipRecipientMismatch { expected, resolved })
            if expected == contact && resolved == moved_to
    ));

    let (wallet, seed, id, coordinator) = tip_race_fixture("tip_expected_identity").await;
    let reviewed = recipient(2, [4; 43]);
    let prover = CachedOrchardProver::new();
    let result = wallet
        .send_shielded_tip(
            wallet.admit_shielded_tip().expect("admit"),
            &coordinator,
            &seed[..],
            ShieldedTipSource::Identity(id),
            "alice",
            &reviewed,
            Some(&contact),
            1_000,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedTipRecipientMismatch { .. })
        ),
        "{result:?}"
    );
    // The expected identity is the reviewed one: the send goes on to its
    // name lookup, which the test's SDK does not answer.
    let result = wallet
        .send_shielded_tip(
            wallet.admit_shielded_tip().expect("admitted again"),
            &coordinator,
            &seed[..],
            ShieldedTipSource::Identity(id),
            "alice",
            &reviewed,
            Some(&moved_to),
            1_000,
            [0; 36],
            &prover,
        )
        .await;
    // The lookup failed before anything was built: proven not sent, with
    // the lookup's own diagnostic.
    let error = result.expect_err("the lookup is not answered");
    assert!(
        matches!(error.dispatched_source(), PlatformWalletError::Sdk(_)),
        "{error:?}"
    );
    assert_eq!(error.payment_delivery(), PaymentDelivery::NotSent);
}

/// A recipient that changed since the review is refused, also when only its
/// shielded address changed; `send_shielded_tip` runs this check with its
/// other pre-send refusals under `before_dispatch`, so it is `NotSent` and
/// keeps its diagnostic (the lookup failure above goes the same way).
#[test]
fn should_refuse_a_recipient_whose_address_alone_changed_as_not_sent() {
    let reviewed = recipient(2, [4; 43]);
    assert!(ensure_recipient_unchanged(&reviewed, &reviewed).is_ok());
    let moved_address = ShieldedTipRecipient {
        identity_id: reviewed.identity_id,
        address: [5; 43],
    };
    let refused = ensure_recipient_unchanged(&moved_address, &reviewed)
        .expect_err("a changed address is a changed recipient");
    assert!(matches!(
        refused,
        PlatformWalletError::InvalidIdentityData(_)
    ));
    let refused = refused.before_dispatch();
    assert_eq!(refused.payment_delivery(), PaymentDelivery::NotSent);
    assert!(matches!(
        refused.dispatched_source(),
        PlatformWalletError::InvalidIdentityData(_)
    ));
}

/// The owner of a contact's name is checked as soon as the name is resolved,
/// before its profile is read: a name that moved is reported as that, also
/// when its new owner publishes no shielded address, and also when it moved
/// between a review and the send.
#[tokio::test]
async fn should_report_a_moved_name_before_reading_its_new_owners_profile() {
    let (contact, new_owner) = (Identifier::from([1; 32]), Identifier::from([2; 32]));
    let address = keys_of(2, 0).default_address.to_raw_address_bytes();

    // The new owner has no shielded address: the profile is never read.
    let result = resolve_recipient(
        "alice",
        Some(&contact),
        || async { Ok(Some(new_owner)) },
        |_| async { panic!("the profile of another identity than expected was read") },
    )
    .await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedTipRecipientMismatch { expected, resolved })
                if expected == contact && resolved == new_owner
        ),
        "{result:?}"
    );
    // The new owner has one: a mismatch all the same, not a changed recipient.
    let result = resolve_recipient(
        "alice",
        Some(&contact),
        || async { Ok(Some(new_owner)) },
        |_| async move { Ok(Some(address)) },
    )
    .await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedTipRecipientMismatch { .. })
        ),
        "{result:?}"
    );

    // The name still belongs to the contact, or was typed: resolved as before.
    for expected in [Some(&contact), None] {
        let resolved = resolve_recipient(
            "alice",
            expected,
            || async { Ok(Some(contact)) },
            |_| async move { Ok(Some(address)) },
        )
        .await
        .expect("resolve");
        assert_eq!(
            resolved,
            ShieldedTipRecipient {
                identity_id: contact,
                address
            }
        );
    }
    // A typed name whose owner publishes no address, and a name not found.
    for result in [
        resolve_recipient(
            "alice",
            None,
            || async { Ok(Some(new_owner)) },
            |_| async { Ok(None) },
        )
        .await,
        resolve_recipient(
            "alice",
            Some(&contact),
            || async { Ok(None) },
            |_| async { Ok(None) },
        )
        .await,
    ] {
        assert!(
            matches!(result, Err(PlatformWalletError::InvalidIdentityData(_))),
            "{result:?}"
        );
    }
}

/// A tip of zero credits is refused before its recipient is recorded as
/// confirmed, so a later review does not report a recipient no payment was
/// ever admitted to.
#[tokio::test]
async fn should_refuse_a_zero_tip_before_confirming_its_recipient() {
    let (wallet, seed, _id, coordinator) = tip_race_fixture("tip_zero_amount").await;
    let paid = recipient(2, keys_of(3, 0).default_address.to_raw_address_bytes());
    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Account(0),
            0,
            "b0b",
            &paid,
            0,
            [0; 36],
            &CachedOrchardProver::new(),
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::InvalidParameter(_))),
        "{result:?}"
    );
    // Nor does a tip from an account that is not bound: it could never be
    // paid, so it confirms nothing either.
    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Account(5),
            5,
            "b0b",
            &paid,
            1_000,
            [0; 36],
            &CachedOrchardProver::new(),
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::ShieldedKeyDerivation(_))),
        "{result:?}"
    );
    assert_eq!(
        coordinator
            .store()
            .read()
            .await
            .confirmed_tip_recipient(wallet.wallet_id(), "b0b")
            .unwrap(),
        None
    );
}

/// A balance read that waits for the shielded store does not hold the
/// coordinator's operation admission, so a Clear requested meanwhile is
/// admitted. While that Clear is admitted the read reports nothing: the
/// ledger it read may be the one being purged, and the Clear may also give
/// up with the preparation still standing. The bounded read reports a held
/// store as busy too, by the same error.
#[tokio::test]
async fn should_not_refuse_a_clear_while_a_balance_read_waits_for_the_store() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tip_balance_read_and_clear").await;
    wallet
        .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
        .await
        .expect("prepare");
    let account = shielded_identity_account_index(2).unwrap();
    assert!(!coordinator.clear_in_progress());

    // A scan holds the store.
    let scan = coordinator.store().write().await;
    let busy = wallet
        .shielded_identity_balance_now(&coordinator, &id)
        .await;
    assert!(
        matches!(busy, Err(PlatformWalletError::ShieldedBalanceBusy)),
        "{busy:?}"
    );
    let mut waiting = Box::pin(wallet.shielded_identity_balance(&coordinator, &id));
    assert!(
        futures::poll!(&mut waiting).is_pending(),
        "the read waits for the store"
    );
    let admission = coordinator
        .try_admit_clear()
        .expect("a Clear is admitted while the read waits");
    assert!(coordinator.clear_in_progress());
    // The scan ends with the Clear still admitted. Bounded here, so a read
    // that came to wait for the Clear fails instead of hanging the test.
    drop(scan);
    let during = tokio::time::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .expect("the balance read finishes while the Clear is admitted");
    assert!(
        matches!(during, Err(PlatformWalletError::ShieldedBalanceBusy)),
        "{during:?}"
    );

    // The Clear gave up without wiping anything: the preparation stands.
    drop(admission);
    assert!(!coordinator.clear_in_progress());
    assert_eq!(
        wallet
            .shielded_identity_balance(&coordinator, &id)
            .await
            .expect("read"),
        ShieldedIdentityBalance::Unavailable { account }
    );
    // Asking whether a Clear is admitted takes nothing a Clear needs.
    drop(
        coordinator
            .try_admit_clear()
            .expect("a Clear is admitted after the reads"),
    );

    // A Clear that ran ends the preparation.
    coordinator
        .try_admit_clear()
        .expect("admit")
        .clear()
        .await
        .expect("clear");
    assert!(!coordinator.clear_in_progress());
    assert_eq!(
        wallet
            .shielded_identity_balance(&coordinator, &id)
            .await
            .expect("read"),
        ShieldedIdentityBalance::Unverified
    );
    // An identity the wallet does not hold is not a busy store.
    assert!(matches!(
        wallet
            .shielded_identity_balance_now(&coordinator, &Identifier::from([0x01; 32]))
            .await,
        Err(PlatformWalletError::IdentityNotFound(_))
    ));
}

/// A tip to an address of the account it is paid from is refused: its
/// payment would be one more of the account's own outputs beside its
/// change, and an unresolved one could not hold back a repeat.
#[tokio::test]
async fn should_refuse_a_tip_to_the_account_it_is_paid_from() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tip_to_own_account").await;
    wallet
        .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
        .await
        .expect("prepare");
    let identity_account = shielded_identity_account_index(2).unwrap();
    let published = wallet
        .shielded_default_address(identity_account)
        .await
        .expect("the identity account is bound");
    let own = recipient(0x66, published);
    let prover = CachedOrchardProver::new();

    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Identity(id),
            identity_account,
            "me",
            &own,
            1,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::InvalidParameter(_))),
        "{result:?}"
    );
    assert_eq!(
        coordinator
            .store()
            .read()
            .await
            .confirmed_tip_recipient(wallet.wallet_id(), "me")
            .unwrap(),
        None,
        "a refused tip confirms nothing"
    );

    // Paid from another account of the wallet it is a payment like any
    // other: admitted, and the spend then finds no notes to pay with.
    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Account(0),
            0,
            "me",
            &own,
            1,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        !matches!(
            result,
            Ok(()) | Err(PlatformWalletError::InvalidParameter(_))
        ),
        "{result:?}"
    );
}

/// A review and an identity's balance read the store of the coordinator
/// the wallet is bound on. Another coordinator's store holds none of the
/// wallet's history, and would show a changed recipient as new and an
/// unresolved payment as none.
#[tokio::test]
async fn should_refuse_a_review_and_a_balance_on_a_coordinator_the_wallet_is_not_bound_on() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tip_other_coordinator").await;
    wallet
        .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
        .await
        .expect("prepare");
    let other = coordinator_at(&temp_dir("tip_other_coordinator_second"));
    assert!(matches!(
        wallet.review_shielded_tip(&other, "alice", None).await,
        Err(PlatformWalletError::ShieldedNotBound)
    ));
    assert!(matches!(
        wallet.shielded_identity_balance(&other, &id).await,
        Err(PlatformWalletError::ShieldedNotBound)
    ));
    assert!(wallet
        .shielded_identity_balance(&coordinator, &id)
        .await
        .is_ok());
}

/// A bind that would drop the account an unresolved spend was paid from is
/// refused: dropping the account drops the spend's saved record, which is
/// what holds back a second payment to the same recipient. Once the spend
/// resolves the account can go.
#[tokio::test]
async fn should_refuse_a_bind_that_drops_the_account_of_an_unresolved_spend() {
    let (wallet, seed, _id, coordinator) = tip_race_fixture("tip_bind_drops_payer").await;
    wallet
        .bind_shielded(&seed[..], &[0, 1], &coordinator)
        .await
        .expect("bind a second ordinary account");
    let payer = wallet
        .shielded_keys
        .read()
        .await
        .as_ref()
        .and_then(|keys| keys.get(&1).cloned())
        .expect("account 1 is bound");
    let paid = keys_of(2, 0).default_address;
    let payer_id = SubwalletId::new(wallet.wallet_id(), 1);
    let record = transfer_paying(&payer, paid);
    let spent = record.nullifiers[0];
    arm_spend(&mut *coordinator.store().write().await, payer_id, record);

    let result = wallet.bind_shielded(&seed[..], &[0], &coordinator).await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedRecoveryKeysRequired {
                account_index: 1,
                ..
            })
        ),
        "{result:?}"
    );
    let keys = wallet.shielded_keys.read().await.clone().expect("bound");
    assert_eq!(
        unresolved_transfer_recipients(
            &*coordinator.store().read().await,
            wallet.wallet_id(),
            &keys
        )
        .unwrap(),
        BTreeSet::from([paid.to_raw_address_bytes()]),
        "the record, and the refusal it gives, are kept"
    );

    // Resolved (its anchor proven pruned, here): the account can go.
    assert!(coordinator
        .store()
        .write()
        .await
        .clear_pending(payer_id, &spent)
        .expect("release"));
    wallet
        .bind_shielded(&seed[..], &[0], &coordinator)
        .await
        .expect("bind without the account once its spend resolved");
}

/// A tip is paid only if its source still stands once the tip's notes are
/// claimed: claiming them can wait for a scan, and meanwhile the identity
/// can be removed, or the slot of a numbered account can be given to an
/// identity. The notes are then released and nothing is built or sent.
#[tokio::test]
async fn should_release_the_claimed_notes_when_the_tips_source_no_longer_stands() {
    let (wallet, seed, _id, coordinator) = tip_race_fixture("tip_source_after_claim").await;
    let identity_account = shielded_identity_account_index(2).unwrap();
    let payer = SubwalletId::new(wallet.wallet_id(), identity_account);
    let paid = recipient(2, keys_of(3, 0).default_address.to_raw_address_bytes());
    let gone = ShieldedTipSource::Identity(Identifier::from([0x01; 32]));
    let prover = CachedOrchardProver::new();

    // With nothing to claim the transfer ends before the check.
    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            gone,
            identity_account,
            "b0b",
            &paid,
            1_000,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::ShieldedNoUnspentNotes)),
        "{result:?}"
    );

    coordinator
        .store()
        .write()
        .await
        .save_note(
            payer,
            &ShieldedNote {
                value: 1_000_000_000_000,
                ..input_note([5; 32])
            },
        )
        .expect("save a note");
    // An identity that is gone, then the account by its number while the
    // fixture's identity holds its slot.
    let held_by_number = ShieldedTipSource::Account(identity_account);
    for source in [gone, held_by_number] {
        let result = wallet
            .send_resolved_shielded_tip(
                &coordinator,
                &seed[..],
                source,
                identity_account,
                "b0b",
                &paid,
                1_000,
                [0; 36],
                &prover,
            )
            .await;
        match source {
            ShieldedTipSource::Identity(_) => assert!(
                matches!(result, Err(PlatformWalletError::IdentityNotFound(_))),
                "{result:?}"
            ),
            ShieldedTipSource::Account(_) => assert!(
                matches!(result, Err(PlatformWalletError::InvalidIdentityData(_))),
                "{result:?}"
            ),
        }
        let store = coordinator.store().read().await;
        assert_eq!(
            store.get_unspent_notes(payer).expect("unspent").len(),
            1,
            "the claimed note is released: {source:?}"
        );
        assert!(
            store.pending_redrives(payer).expect("records").is_empty(),
            "nothing was built or saved: {source:?}"
        );
    }
}

/// The store of an earlier process holds a spend paid from account 1 that
/// is unresolved. The first bind of a new process must carry that account:
/// only its scan resolves the spend, and what reads the record needs its
/// keys.
#[tokio::test]
async fn should_refuse_a_first_bind_without_the_account_of_an_unresolved_spend() {
    let wallet = platform_wallet_with(Arc::new(CapturingPersistence::default())).await;
    let dir = temp_dir("tip_first_bind_without_payer");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let payer = OrchardKeySet::from_seed(&BIND_SEED, Network::Testnet, 1)
        .expect("derive keys")
        .viewing_keys();
    {
        let mut store =
            FileBackedShieldedStore::open_path(dir.join("tree.sqlite"), 100).expect("open");
        arm_spend(
            &mut store,
            SubwalletId::new(wallet.wallet_id(), 1),
            transfer_paying(&payer, keys_of(2, 0).default_address),
        );
    }
    let coordinator = coordinator_at(&dir);

    let result = wallet.bind_shielded(&BIND_SEED, &[0], &coordinator).await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedRecoveryKeysRequired {
                account_index: 1,
                ..
            })
        ),
        "{result:?}"
    );
    wallet
        .bind_shielded(&BIND_SEED, &[0, 1], &coordinator)
        .await
        .expect("bind with the paying account");
}

/// A damaged record of an unresolved spend is kept in the store without
/// being loaded, so nothing says which account paid it. A bind that drops
/// an account is refused while one is kept: the account's purge would
/// delete it, and after another restart nothing would hold back a second
/// payment.
#[tokio::test]
async fn should_refuse_a_bind_that_drops_an_account_while_a_damaged_spend_record_is_kept() {
    let wallet = platform_wallet_with(Arc::new(CapturingPersistence::default())).await;
    let dir = temp_dir("tip_bind_with_damaged_record");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("tree.sqlite");
    let payer = OrchardKeySet::from_seed(&BIND_SEED, Network::Testnet, 1)
        .expect("derive keys")
        .viewing_keys();
    {
        let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
        arm_spend(
            &mut store,
            SubwalletId::new(wallet.wallet_id(), 1),
            transfer_paying(&payer, keys_of(2, 0).default_address),
        );
    }
    let saved_rows = || {
        rusqlite::Connection::open(&path)
            .expect("open the store file")
            .query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count")
    };
    // An anchor of the wrong width: the row no longer loads.
    rusqlite::Connection::open(&path)
        .expect("open the store file")
        .execute("UPDATE shielded_pending_spends SET anchor = x'00'", [])
        .expect("damage the record");
    let coordinator = coordinator_at(&dir);
    assert!(coordinator
        .store()
        .read()
        .await
        .has_unloaded_note_spends(wallet.wallet_id()));

    wallet
        .bind_shielded(&BIND_SEED, &[0, 1], &coordinator)
        .await
        .expect("a bind that drops nothing");
    let result = wallet.bind_shielded(&BIND_SEED, &[0], &coordinator).await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedRecoveryCorrupted {
                account_index: None,
                ..
            })
        ),
        "{result:?}"
    );
    assert_eq!(saved_rows(), 1, "the damaged record is kept");
    assert_eq!(wallet.shielded_account_indices().await, vec![0, 1]);
}

/// The transfer builder keys the recipient's output and the change to the
/// paying account's outgoing viewing key, and the record of the transfer is
/// saved under the hash of the outputs the account reads. So the recipient
/// of a transfer built and proved as a tip is, read back from its saved
/// transition after a restart, the address it pays.
#[tokio::test]
async fn should_read_the_recipient_of_a_transfer_the_builder_proved() {
    let keyset = OrchardKeySet::from_seed(&[1; 64], Network::Testnet, 0).expect("derive keys");
    let payer = keyset.viewing_keys();
    let keys = BTreeMap::from([(0, payer.clone())]);
    let paid = keys_of(2, 0).default_address;
    let orchard = |address: &PaymentAddress| {
        OrchardAddress::from_raw_bytes(&address.to_raw_address_bytes()).expect("orchard address")
    };

    // A note of the paying account, with a witness whose root is the anchor.
    let rho = Option::from(Rho::from_bytes(&[0u8; 32])).expect("zero is a valid rho");
    let rseed = Option::from(RandomSeed::from_bytes([1u8; 32], &rho)).expect("valid rseed");
    let note: Note = Option::from(Note::from_parts(
        payer.default_address,
        NoteValue::from_raw(12_000_000_000),
        rho,
        rseed,
    ))
    .expect("valid note");
    let merkle_path = MerklePath::from_parts(
        0,
        [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH],
    );
    let anchor = merkle_path.root(ExtractedNoteCommitment::from(note.commitment()));

    let prover = CachedOrchardProver::new();
    let (transition, _fee) = build_shielded_transfer_transition(
        vec![SpendableNote { note, merkle_path }],
        &orchard(&paid),
        1_000,
        &orchard(&payer.default_address),
        &keyset.full_viewing_key,
        &keyset.spend_auth_key,
        anchor,
        &&prover,
        [0u8; 36],
        PlatformVersion::latest(),
    )
    .expect("build the transfer");
    let actions = shielded_actions(&transition).to_vec();
    assert_eq!(
        visible_output_cmxs(&actions, &payer).len(),
        2,
        "the account reads the payment and its change"
    );

    let (mut store, path) = store_at("tip_builder_transfer");
    let id = SubwalletId::new(WALLET, 0);
    let record = PendingRedrive {
        activity_id: saved_id(&actions, &payer),
        ..record_of(0, transition, &actions)
    };
    store.arm_redrive(id, record).expect("arm");
    drop(store);
    let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
    assert_eq!(
        unresolved_transfer_recipients(&store, WALLET, &keys).unwrap(),
        BTreeSet::from([paid.to_raw_address_bytes()])
    );
}

/// An identity pays from the account of the slot the seed proves it holds.
/// An identity account named by its number is spent only when its slot
/// holds no identity.
#[tokio::test]
async fn should_pay_an_identitys_tip_from_the_account_the_seed_proves() {
    let (wallet, seed, id, _coordinator) = tip_race_fixture("tip_source_account").await;
    let identity_account = shielded_identity_account_index(2).unwrap();
    assert_eq!(
        wallet
            .tip_source_account(&seed[..], ShieldedTipSource::Identity(id))
            .await
            .unwrap(),
        identity_account
    );

    // Recorded at slot 3, but carrying the MASTER key the seed derives at
    // slot 4: the seed does not prove slot 3's account is its own.
    let misplaced = {
        let misplaced = Identifier::from([0x77; 32]);
        let mut wm = wallet.wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                Identity::V0(IdentityV0 {
                    id: misplaced,
                    public_keys: seed_master_key(&seed[..], 4),
                    balance: 0,
                    revision: 0,
                }),
                3,
                wallet.wallet_id(),
                wallet.persister(),
            )
            .unwrap();
        misplaced
    };
    assert!(matches!(
        wallet
            .tip_source_account(&seed[..], ShieldedTipSource::Identity(misplaced))
            .await,
        Err(PlatformWalletError::InvalidIdentityData(_))
    ));
    assert!(matches!(
        wallet
            .tip_source_account(
                &seed[..],
                ShieldedTipSource::Identity(Identifier::from([0x01; 32]))
            )
            .await,
        Err(PlatformWalletError::IdentityNotFound(_))
    ));

    // By number: refused while the slot holds an identity, whichever one.
    for held in [2, 3] {
        let account = shielded_identity_account_index(held).unwrap();
        assert!(
            matches!(
                wallet
                    .tip_source_account(&seed[..], ShieldedTipSource::Account(account))
                    .await,
                Err(PlatformWalletError::InvalidIdentityData(_))
            ),
            "slot {held}"
        );
    }
    let retired = shielded_identity_account_index(9).unwrap();
    for account in [0, retired] {
        assert_eq!(
            wallet
                .tip_source_account(&seed[..], ShieldedTipSource::Account(account))
                .await
                .unwrap(),
            account
        );
    }
}

/// A tip is refused, with nothing recorded, while an earlier transfer to its
/// recipient may still execute. A tip to another recipient is not held back:
/// it goes on to the spend, and what the user confirmed is on file.
#[tokio::test]
async fn should_refuse_a_tip_while_an_earlier_payment_to_its_recipient_is_unresolved() {
    let (wallet, seed, _id, coordinator) = tip_race_fixture("tip_unresolved_send").await;
    let payer = wallet
        .shielded_keys
        .read()
        .await
        .as_ref()
        .and_then(|keys| keys.get(&0).cloned())
        .expect("account 0 is bound");
    let paid = recipient(1, keys_of(2, 0).default_address.to_raw_address_bytes());
    let other = recipient(2, keys_of(3, 0).default_address.to_raw_address_bytes());
    coordinator
        .store()
        .write()
        .await
        .arm_redrive(
            SubwalletId::new(wallet.wallet_id(), 0),
            transfer_paying(
                &payer,
                PaymentAddress::from_raw_address_bytes(&paid.address).unwrap(),
            ),
        )
        .expect("arm");
    let prover = CachedOrchardProver::new();

    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Account(0),
            0,
            "a11ce",
            &paid,
            1,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::ShieldedTipPending)),
        "{result:?}"
    );
    // The name's recipient replaced its address since: still the same payee.
    coordinator
        .store()
        .write()
        .await
        .confirm_tip_recipient(
            wallet.wallet_id(),
            "a11ce",
            &(paid.identity_id.to_buffer(), paid.address),
        )
        .expect("confirm");
    let rotated = recipient(1, other.address);
    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Account(0),
            0,
            "a11ce",
            &rotated,
            1,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        matches!(result, Err(PlatformWalletError::ShieldedTipPending)),
        "{result:?}"
    );
    assert_eq!(
        coordinator
            .store()
            .read()
            .await
            .confirmed_tip_recipient(wallet.wallet_id(), "a11ce")
            .unwrap(),
        Some((paid.identity_id.to_buffer(), paid.address)),
        "a refused tip confirms nothing"
    );

    // Another name, another recipient: admitted, and the spend then finds no
    // notes to pay with.
    let result = wallet
        .send_resolved_shielded_tip(
            &coordinator,
            &seed[..],
            ShieldedTipSource::Account(0),
            0,
            "b0b",
            &other,
            1,
            [0; 36],
            &prover,
        )
        .await;
    assert!(
        !matches!(
            result,
            Ok(()) | Err(PlatformWalletError::ShieldedTipPending)
        ),
        "{result:?}"
    );
    assert_eq!(
        coordinator
            .store()
            .read()
            .await
            .confirmed_tip_recipient(wallet.wallet_id(), "b0b")
            .unwrap(),
        Some((other.identity_id.to_buffer(), other.address))
    );
}

/// An identity's shielded balance is a number only once the seed proved its
/// account in this session and that account's ledger is established.
#[tokio::test]
async fn should_report_an_identitys_shielded_balance_only_once_it_is_known() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tip_identity_balance").await;
    assert!(matches!(
        wallet
            .shielded_identity_balance(&coordinator, &Identifier::from([0x01; 32]))
            .await,
        Err(PlatformWalletError::IdentityNotFound(_))
    ));
    // A recorded index alone proves nothing about the account.
    assert_eq!(
        wallet
            .shielded_identity_balance(&coordinator, &id)
            .await
            .unwrap(),
        ShieldedIdentityBalance::Unverified
    );

    wallet
        .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
        .await
        .expect("prepare");
    let account = shielded_identity_account_index(2).unwrap();
    // Proved, but neither restored nor scanned: not a known zero.
    assert_eq!(
        wallet
            .shielded_identity_balance(&coordinator, &id)
            .await
            .unwrap(),
        ShieldedIdentityBalance::Unavailable { account }
    );

    // Another identity's account being scanned says nothing about this one.
    let other = add_identity_at_slot(&wallet, &seed[..], 0x67, 3).await;
    wallet
        .prepare_shielded_identity_address(&seed[..], &other, &coordinator)
        .await
        .expect("prepare the other identity");
    let identity_subwallet = SubwalletId::new(wallet.wallet_id(), account);
    {
        let mut store = coordinator.store().write().await;
        for scanned in wallet.shielded_account_indices().await {
            store
                .set_last_synced_note_index(SubwalletId::new(wallet.wallet_id(), scanned), 0)
                .expect("complete a scan");
        }
        store
            .save_note(
                identity_subwallet,
                &ShieldedNote {
                    position: 1,
                    cmx: [1; 32],
                    nullifier: [1; 32],
                    block_height: 1,
                    is_spent: false,
                    value: 4_200,
                    note_data: vec![1; 115],
                },
            )
            .expect("save a note");
    }
    assert_eq!(
        wallet
            .shielded_identity_balance(&coordinator, &id)
            .await
            .unwrap(),
        ShieldedIdentityBalance::Available {
            account,
            credits: 4_200
        }
    );
    assert_eq!(
        wallet
            .shielded_identity_balance(&coordinator, &other)
            .await
            .unwrap(),
        ShieldedIdentityBalance::Available {
            account: shielded_identity_account_index(3).unwrap(),
            credits: 0
        }
    );
}
