//! Round-trip: a Type 15 Shield bundle built by the wallet's own
//! builder must be trial-decryptable by the same wallet's IVK.
//!
//! This is the exact client-side pair the app exercises end-to-end:
//!
//!   * build side — `operations::shield` derives the recipient from
//!     `OrchardKeySet::default_address` and calls dpp's
//!     `build_shield_transition`, whose serialized actions are stored
//!     verbatim on-chain (`ShieldedActionNote::from(&SerializedAction)`
//!     → `insert_note_op`, all field-name-mapped, no re-encoding);
//!   * scan side — the sync stream parses the stored item back into
//!     `ShieldedEncryptedNote {cmx, nullifier, cv_net, encrypted_note}`
//!     and runs `try_decrypt_note` under the account's prepared IVK.
//!
//! So building the transition here and feeding its actions straight
//! into `try_decrypt_note` reproduces the full client round trip with
//! the chain's (verbatim) storage cut out of the middle. Exactly one
//! of the two padded actions must decrypt: the real recipient output.
//! The other is Orchard's dummy padding output, decryptable by no one.

use std::collections::BTreeMap;

use dash_sdk::platform::shielded::try_decrypt_note;
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

use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::prover::CachedOrchardProver;

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
async fn shield_built_note_is_trial_decryptable_by_own_ivk() {
    let seed = [0x42u8; 32];
    let keys = OrchardKeySet::from_seed(&seed, Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed");

    // Recipient = the wallet's own default address — the same
    // conversion `operations::shield` performs via
    // `default_orchard_address`.
    let recipient = OrchardAddress::from_raw_bytes(&keys.default_address.to_raw_address_bytes())
        .expect("default address must convert to OrchardAddress");

    let amount: u64 = 200_000_000_000; // 2 DASH in credits
    let mut inputs = BTreeMap::new();
    inputs.insert(
        PlatformAddress::P2pkh([0xAB; 20]),
        (0u32, 500_000_000_000u64),
    );

    // `OrchardProver` is implemented for `&CachedOrchardProver`
    // (the cached key lives in a static), so P = `&CachedOrchardProver`
    // and the builder's `&P` is a double reference — the same shape
    // `shielded_shield_from_account` passes through `operations::shield`.
    let prover = CachedOrchardProver::new();
    let st = build_shield_transition(
        &recipient,
        amount,
        inputs,
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
        &DummySigner,
        0,
        &&prover,
        [0u8; 36],
        // Production config (`operations::shield`): the output's
        // out_ciphertext is keyed to the wallet's own OVK. Irrelevant to
        // the IVK trial-decryption under test, but kept in lockstep.
        Some(keys.outgoing_viewing_key.clone()),
        PlatformVersion::latest(),
    )
    .await
    .expect("shield transition build should succeed");

    let StateTransition::Shield(ShieldTransition::V0(v0)) = st else {
        panic!("expected a Shield state transition");
    };

    // Output-only bundles pad to exactly 2 actions.
    assert_eq!(
        v0.actions.len(),
        2,
        "output-only shield bundle must carry exactly 2 padded actions"
    );

    // Reassemble each action exactly as the chain stores and the sync
    // stream re-parses it, then trial-decrypt with the same keyset's
    // prepared IVK — the scanner's exact call.
    let ivk = keys.prepared_ivk();
    let decrypted: Vec<u64> = v0
        .actions
        .iter()
        .filter_map(|a| {
            let wire = ShieldedEncryptedNote {
                cmx: a.cmx.to_vec(),
                nullifier: a.nullifier.to_vec(),
                cv_net: a.cv_net.to_vec(),
                encrypted_note: a.encrypted_note.clone(),
            };
            try_decrypt_note(&ivk, &wire).map(|(note, _addr)| note.value().inner())
        })
        .collect();

    assert_eq!(
        decrypted.len(),
        1,
        "exactly one action (the real recipient output) must trial-decrypt; \
         0 means the scanner can never see shielded funds, 2 means padding leaked"
    );
    assert_eq!(
        decrypted[0], amount,
        "decrypted note value must equal the shielded amount"
    );
}
