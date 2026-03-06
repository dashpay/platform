//! Convenience builders for constructing shielded state transitions.
//!
//! These functions encapsulate the full Orchard bundle construction pipeline:
//! builder configuration, proof generation, signature application,
//! and serialization into platform state transitions.
//!
//! Requires the `shielded-client` feature, which pulls in
//! `grovedb-commitment-tree` (and transitively the `orchard` crate).
//!
//! # Example
//!
//! ```ignore
//! use dpp::shielded::builder::*;
//! use grovedb_commitment_tree::{SpendingKey, FullViewingKey, Scope, ProvingKey};
//!
//! // Derive recipient address
//! let sk = SpendingKey::from_bytes(seed)?;
//! let fvk = FullViewingKey::from(&sk);
//! let recipient = OrchardAddress::from_raw_bytes(
//!     &fvk.address_at(0, Scope::External).to_raw_address_bytes(),
//! );
//!
//! // Build a shield transition
//! let pk = ProvingKey::build();
//! let st = build_shield_transition(
//!     &recipient, shield_amount, inputs, fee_strategy,
//!     &signer, 0, &pk, [0u8; 36], platform_version,
//! )?;
//! ```

mod shield;
mod shield_from_asset_lock;
mod shielded_transfer;
mod shielded_withdrawal;
mod unshield;

pub use self::shield::build_shield_transition;
pub use shield_from_asset_lock::build_shield_from_asset_lock_transition;
pub use shielded_transfer::build_shielded_transfer_transition;
pub use shielded_withdrawal::build_shielded_withdrawal_transition;
pub use unshield::build_unshield_transition;

use grovedb_commitment_tree::{
    Anchor, Authorized, Builder, Bundle, BundleType, DashMemo, Flags as OrchardFlags,
    FullViewingKey, MerklePath, Note, NoteValue, PaymentAddress, ProvingKey, SpendAuthorizingKey,
};
use rand::rngs::OsRng;

use crate::address_funds::OrchardAddress;
use crate::shielded::{compute_platform_sighash, SerializedAction};
use crate::ProtocolError;

/// Trait abstracting over Orchard proof generation.
///
/// This follows the same pattern as `Signer` — callers provide an implementation
/// that holds (and potentially caches) the expensive `ProvingKey`, and the builder
/// functions use it via this trait.
pub trait OrchardProver {
    /// Returns a reference to the Halo 2 proving key for the Orchard circuit.
    fn proving_key(&self) -> &ProvingKey;
}

/// A note that can be spent in a shielded transaction, paired with its
/// Merkle inclusion path in the commitment tree.
pub struct SpendableNote {
    /// The Orchard note to spend.
    pub note: Note,
    /// Merkle path proving the note's commitment exists in the tree.
    pub merkle_path: MerklePath,
}

/// The serialized fields extracted from an authorized Orchard bundle,
/// ready for use by state transition constructors.
pub struct SerializedBundle {
    /// Serialized Orchard actions (spends + outputs).
    pub actions: Vec<SerializedAction>,
    /// Bundle flags byte.
    pub flags: u8,
    /// Net value balance (positive = value leaving the shielded pool).
    pub value_balance: i64,
    /// Sinsemilla root of the Orchard note commitment tree (32 bytes).
    /// This is the Orchard `Anchor` — the root hash of the depth-32 Sinsemilla
    /// Merkle tree over extracted note commitments (cmx values).
    pub anchor: [u8; 32],
    /// Halo 2 proof bytes.
    pub proof: Vec<u8>,
    /// Binding signature (64 bytes).
    pub binding_signature: [u8; 64],
}

impl From<&OrchardAddress> for PaymentAddress {
    fn from(address: &OrchardAddress) -> Self {
        *address.inner()
    }
}

/// Serializes an authorized Orchard bundle into the raw fields used by
/// state transition constructors.
pub fn serialize_authorized_bundle(bundle: &Bundle<Authorized, i64, DashMemo>) -> SerializedBundle {
    let actions: Vec<SerializedAction> = bundle
        .actions()
        .iter()
        .map(|action| {
            let enc = action.encrypted_note();
            let mut encrypted_note = Vec::with_capacity(216);
            encrypted_note.extend_from_slice(&enc.epk_bytes);
            encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
            encrypted_note.extend_from_slice(&enc.out_ciphertext);
            SerializedAction {
                nullifier: action.nullifier().to_bytes(),
                rk: <[u8; 32]>::from(action.rk()),
                cmx: action.cmx().to_bytes(),
                encrypted_note,
                cv_net: action.cv_net().to_bytes(),
                spend_auth_sig: <[u8; 64]>::from(action.authorization()),
            }
        })
        .collect();
    let flags = bundle.flags().to_byte();
    let value_balance = *bundle.value_balance();
    let anchor = bundle.anchor().to_bytes();
    let proof = bundle.authorization().proof().as_ref().to_vec();
    let binding_signature = <[u8; 64]>::from(bundle.authorization().binding_signature());
    SerializedBundle {
        actions,
        flags,
        value_balance,
        anchor,
        proof,
        binding_signature,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Builds an output-only Orchard bundle (no spends).
///
/// Used by Shield and ShieldFromAssetLock transitions where funds enter
/// the shielded pool from transparent sources.
pub(crate) fn build_output_only_bundle<P: OrchardProver>(
    recipient: &OrchardAddress,
    amount: u64,
    memo: [u8; 36],
    prover: &P,
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let payment_address = PaymentAddress::from(recipient);
    let anchor = Anchor::empty_tree();
    let mut builder = Builder::<DashMemo>::new(
        BundleType::Transactional {
            flags: OrchardFlags::SPENDS_DISABLED,
            bundle_required: false,
        },
        anchor,
    );

    builder
        .add_output(None, payment_address, NoteValue::from_raw(amount), memo)
        .map_err(|e| ProtocolError::Generic(format!("failed to add output: {:?}", e)))?;

    prove_and_sign_bundle(builder, prover, &[], &[])
}

/// Builds a spend+output Orchard bundle.
///
/// Used by ShieldedTransfer, Unshield, and ShieldedWithdrawal where funds
/// are spent from existing notes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_spend_bundle<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    output_amount: u64,
    memo: [u8; 36],
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    extra_sighash_data: &[u8],
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let payment_address = PaymentAddress::from(recipient);

    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| ProtocolError::Generic(format!("failed to add spend: {:?}", e)))?;
    }

    builder
        .add_output(
            None,
            payment_address,
            NoteValue::from_raw(output_amount),
            memo,
        )
        .map_err(|e| ProtocolError::Generic(format!("failed to add output: {:?}", e)))?;

    prove_and_sign_bundle(
        builder,
        prover,
        std::slice::from_ref(ask),
        extra_sighash_data,
    )
}

/// Takes a configured Builder, generates the proof, computes the platform
/// sighash, and applies signatures.
pub(crate) fn prove_and_sign_bundle<P: OrchardProver>(
    builder: Builder<DashMemo>,
    prover: &P,
    signing_keys: &[SpendAuthorizingKey],
    extra_sighash_data: &[u8],
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let mut rng = OsRng;

    let (unauthorized, _) = builder
        .build::<i64>(&mut rng)
        .map_err(|e| ProtocolError::Generic(format!("failed to build bundle: {:?}", e)))?
        .ok_or_else(|| ProtocolError::Generic("bundle was empty after build".to_string()))?;

    let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
    let sighash = compute_platform_sighash(&bundle_commitment, extra_sighash_data);

    let proven = unauthorized
        .create_proof(prover.proving_key(), &mut rng)
        .map_err(|e| ProtocolError::Generic(format!("failed to create proof: {:?}", e)))?;

    proven
        .apply_signatures(rng, sighash, signing_keys)
        .map_err(|e| ProtocolError::Generic(format!("failed to apply signatures: {:?}", e)))
}

/// Shared test utilities for builder tests.
#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use grovedb_commitment_tree::{
        FullViewingKey, Hashable, MerkleHashOrchard, Note, NoteValue, ProvingKey, RandomSeed, Rho,
        Scope, SpendingKey, NOTE_COMMITMENT_TREE_DEPTH,
    };
    use std::sync::OnceLock;

    static PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

    /// Returns a cached ProvingKey (~30s to build on first call).
    pub fn proving_key() -> &'static ProvingKey {
        PROVING_KEY.get_or_init(ProvingKey::build)
    }

    /// Test implementation of `OrchardProver` backed by the cached proving key.
    pub struct TestProver;

    impl super::OrchardProver for TestProver {
        fn proving_key(&self) -> &ProvingKey {
            proving_key()
        }
    }

    /// Creates a test OrchardAddress from a deterministic spending key.
    pub fn test_orchard_address() -> OrchardAddress {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let payment_address = fvk.address_at(0u32, Scope::External);
        OrchardAddress::from_raw_bytes(&payment_address.to_raw_address_bytes())
            .expect("valid orchard address bytes")
    }

    /// Creates a SpendableNote with the given value.
    ///
    /// The note is cryptographically valid (has a valid commitment) but uses
    /// an all-zeros Merkle path, so it will only pass the Orchard circuit when
    /// paired with `Anchor::empty_tree()`. Suitable for both error-path tests
    /// (where the proving key is never reached) and happy-path tests.
    pub fn test_spendable_note(value: u64) -> SpendableNote {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let payment_address = fvk.address_at(0u32, Scope::External);

        // Construct a valid Rho from the zero element (always valid in pallas)
        let rho: Rho =
            Option::from(Rho::from_bytes(&[0u8; 32])).expect("zero is valid pallas::Base");
        let rseed: RandomSeed =
            Option::from(RandomSeed::from_bytes([1u8; 32], &rho)).expect("valid random seed");
        let note: Note = Option::from(Note::from_parts(
            payment_address,
            NoteValue::from_raw(value),
            rho,
            rseed,
        ))
        .expect("note commitment should be valid");

        // All-zeros merkle path at position 0 — consistent with Anchor::empty_tree()
        let auth_path = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        let merkle_path = MerklePath::from_parts(0, auth_path);

        SpendableNote { note, merkle_path }
    }
}
