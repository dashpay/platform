//! Round-trip: a Type 15 Shield bundle built by the wallet's own
//! builder must OVK-recover under the same wallet's outgoing viewing
//! key — the sender-side mirror of `shield_decrypt_tests`.
//!
//! This is the exact client-side pair the app exercises end-to-end:
//!
//!   * build side — `operations::shield` passes the account's
//!     `OrchardKeySet::outgoing_viewing_key` into dpp's
//!     `build_shield_transition`, which keys the recipient output's
//!     `out_ciphertext` to it (the Zcash outgoing-transaction-history
//!     convention); the serialized actions are stored verbatim on-chain;
//!   * scan side — `sync_notes_across` re-parses each stored item into
//!     `ShieldedEncryptedNote` and runs `try_recover_outgoing_note`
//!     under every subwallet's OVK, persisting hits via
//!     `record_outgoing_note`.
//!
//! Before the OVK was threaded into the builders, every output was
//! encrypted under a per-output RANDOM outgoing cipher key
//! (`add_output(None, ...)`), so this recovery returned `None` forever
//! and the outgoing-notes store could never populate. This test pins
//! the fixed behavior: exactly one action (the real recipient output)
//! recovers — recipient, value, AND memo — and persists as a
//! `ShieldedOutgoingNote` row.

use std::collections::BTreeMap;

use dash_sdk::platform::shielded::try_recover_outgoing_note;
use dashcore::Network;
use dpp::address_funds::{
    AddressFundsFeeStrategyStep, AddressWitness, OrchardAddress, PlatformAddress,
};
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::shielded::builder::build_shield_transition;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use grovedb_commitment_tree::ExtractedNoteCommitment;

use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::prover::CachedOrchardProver;
use crate::wallet::shielded::store::{InMemoryShieldedStore, ShieldedStore, SubwalletId};

/// Fake transparent-side signer: the input witnesses are irrelevant
/// to note encryption (they sign the transparent inputs, not the
/// Orchard bundle), so a dummy 65-byte signature suffices.
#[derive(Debug)]
struct DummySigner;

#[async_trait::async_trait]
impl Signer<PlatformAddress> for DummySigner {
    async fn sign(
        &self,
        _key: &PlatformAddress,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::new(vec![0u8; 65]))
    }

    async fn sign_create_witness(
        &self,
        _key: &PlatformAddress,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Ok(AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0u8; 65]),
        })
    }

    fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
        true
    }
}

#[tokio::test]
async fn shield_built_note_ovk_recovers_and_persists_as_outgoing() {
    let seed = [0x42u8; 32];
    let keys = OrchardKeySet::from_seed(&seed, Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed");

    let recipient = OrchardAddress::from_raw_bytes(&keys.default_address.to_raw_address_bytes())
        .expect("default address must convert to OrchardAddress");

    let amount: u64 = 200_000_000_000; // 2 DASH in credits
    let mut memo = [0u8; 36];
    memo[..12].copy_from_slice(b"sent-history");
    let mut inputs = BTreeMap::new();
    inputs.insert(
        PlatformAddress::P2pkh([0xAB; 20]),
        (0u32, 500_000_000_000u64),
    );

    let prover = CachedOrchardProver::new();
    let st = build_shield_transition(
        &recipient,
        amount,
        inputs,
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
        &DummySigner,
        0,
        &&prover,
        memo,
        Some(keys.outgoing_viewing_key.clone()),
        PlatformVersion::latest(),
    )
    .await
    .expect("shield transition build should succeed");

    let StateTransition::Shield(ShieldTransition::V0(v0)) = st else {
        panic!("expected a Shield state transition");
    };

    // Reassemble each action exactly as the chain stores and the sync
    // stream re-parses it, then run the scanner's exact OVK-recovery call.
    let recovered: Vec<_> = v0
        .actions
        .iter()
        .filter_map(|a| {
            let wire = ShieldedEncryptedNote {
                cmx: a.cmx.to_vec(),
                nullifier: a.nullifier.to_vec(),
                cv_net: a.cv_net.to_vec(),
                encrypted_note: a.encrypted_note.clone(),
            };
            try_recover_outgoing_note(&keys.outgoing_viewing_key, &wire)
        })
        .collect();

    assert_eq!(
        recovered.len(),
        1,
        "exactly one action (the real recipient output) must OVK-recover; \
         0 means the wallet can never reconstruct its send history, \
         2 means Orchard's dummy padding leaked"
    );
    let (note, recovered_recipient, recovered_memo) = &recovered[0];
    assert_eq!(
        note.value().inner(),
        amount,
        "recovered note value must equal the shielded amount"
    );
    assert_eq!(
        recovered_recipient.to_raw_address_bytes(),
        keys.default_address.to_raw_address_bytes(),
        "recovered recipient must be the address the builder paid"
    );
    assert_eq!(
        &recovered_memo[..],
        &memo[..],
        "recovered memo must round-trip"
    );

    // Persist through the same store path `sync_notes_across` drives —
    // the row the host surfaces as an outgoing payment.
    let mut store = InMemoryShieldedStore::new();
    let id = SubwalletId::new([0x01; 32], 0);
    let outgoing = crate::wallet::shielded::store::ShieldedOutgoingNote {
        cmx: ExtractedNoteCommitment::from(note.commitment()).to_bytes(),
        recipient: recovered_recipient.to_raw_address_bytes().to_vec(),
        value: note.value().inner(),
        memo: recovered_memo.to_vec(),
        block_height: 42,
    };
    assert!(
        store.record_outgoing_note(id, &outgoing).unwrap(),
        "first record of the recovered send must be newly stored"
    );
    let stored = store.get_outgoing_notes(id).unwrap();
    assert_eq!(stored.len(), 1, "one outgoing row must persist");
    assert_eq!(stored[0].value, amount);
    assert_eq!(stored[0].memo.as_slice(), &memo[..]);

    // A foreign wallet's OVK opens nothing — the send stays out of
    // everyone else's history.
    let other = OrchardKeySet::from_seed(&[0x99u8; 32], Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed");
    let foreign_hits = v0
        .actions
        .iter()
        .filter_map(|a| {
            let wire = ShieldedEncryptedNote {
                cmx: a.cmx.to_vec(),
                nullifier: a.nullifier.to_vec(),
                cv_net: a.cv_net.to_vec(),
                encrypted_note: a.encrypted_note.clone(),
            };
            try_recover_outgoing_note(&other.outgoing_viewing_key, &wire)
        })
        .count();
    assert_eq!(
        foreign_hits, 0,
        "a foreign OVK must not recover the wallet's send"
    );
}

/// The live activity recorder must recover at least one visible output
/// cmx from a real built bundle — the same recovery the scan runs. A
/// `None` here means every live entry silently vanishes (the exact
/// failure mode debugged on devnet 2026-06-12).
#[tokio::test]
async fn live_recorder_builds_entry_from_real_shield_bundle() {
    use crate::wallet::shielded::activity::{ShieldedActivityKind, ShieldedDirection};
    use crate::wallet::shielded::activity_recorder::{
        build_pending_entry, visible_output_cmxs, LiveEntryParams,
    };

    let seed = [0x42u8; 32];
    let keys = OrchardKeySet::from_seed(&seed, Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed");
    let recipient = OrchardAddress::from_raw_bytes(&keys.default_address.to_raw_address_bytes())
        .expect("default address must convert to OrchardAddress");

    let mut inputs = BTreeMap::new();
    inputs.insert(
        PlatformAddress::P2pkh([0xAB; 20]),
        (0u32, 500_000_000_000u64),
    );
    let prover = CachedOrchardProver::new();
    let st = build_shield_transition(
        &recipient,
        200_000_000_000,
        inputs,
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
        &DummySigner,
        0,
        &&prover,
        [0u8; 36],
        Some(keys.outgoing_viewing_key.clone()),
        PlatformVersion::latest(),
    )
    .await
    .expect("shield transition build should succeed");

    let StateTransition::Shield(ShieldTransition::V0(v0)) = st else {
        panic!("expected a Shield state transition");
    };

    let cmxs = visible_output_cmxs(&v0.actions, &keys);
    assert!(
        !cmxs.is_empty(),
        "recorder must recover the wallet-visible output cmx from a real bundle"
    );

    let entry = build_pending_entry(
        &keys,
        LiveEntryParams {
            kind: ShieldedActivityKind::Shield,
            direction: ShieldedDirection::In,
            amount: 200_000_000_000,
            fee: None,
            counterparty: None,
            memo: None,
            actions: &v0.actions,
            spent_notes: &[],
        },
    );
    assert!(entry.is_some(), "live entry must build from a real bundle");
}
