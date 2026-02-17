//! Convenience builders for constructing shielded state transitions.
//!
//! These functions encapsulate the full Orchard bundle construction pipeline:
//! builder configuration, proof generation, signature application,
//! and serialization into platform state transitions.
//!
//! Requires the `shielded-bundle-building` feature, which pulls in
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

use std::collections::BTreeMap;

use grovedb_commitment_tree::{
    Anchor, Authorized, Builder, Bundle, BundleType, DashMemo, Flags as OrchardFlags,
    FullViewingKey, MerklePath, Note, NoteValue, PaymentAddress, ProvingKey, SpendAuthorizingKey,
};
use rand::rngs::OsRng;

use crate::address_funds::AddressFundsFeeStrategy;
use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::identity::signer::Signer;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::shielded::{compute_platform_sighash, SerializedAction};
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::state_transition::StateTransition;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// A note that can be spent in a shielded transaction, paired with its
/// Merkle inclusion path in the commitment tree.
pub struct SpendableNote {
    /// The Orchard note to spend.
    pub note: Note,
    /// Merkle path proving the note's commitment exists in the tree.
    pub merkle_path: MerklePath,
}

/// Converts an [`OrchardAddress`] to an Orchard [`PaymentAddress`].
///
/// Returns an error if `pk_d` is not a valid Pallas curve point.
pub fn orchard_address_to_payment_address(
    address: &OrchardAddress,
) -> Result<PaymentAddress, ProtocolError> {
    let raw = address.to_raw_bytes();
    Option::from(PaymentAddress::from_raw_address_bytes(&raw)).ok_or_else(|| {
        ProtocolError::DecodingError(
            "OrchardAddress pk_d is not a valid Pallas curve point".to_string(),
        )
    })
}

/// Serializes an authorized Orchard bundle into the raw fields used by
/// state transition constructors.
///
/// Returns `(actions, flags, value_balance, anchor, proof, binding_signature)`.
pub fn serialize_authorized_bundle(
    bundle: &Bundle<Authorized, i64, DashMemo>,
) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
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
    let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
    (actions, flags, value_balance, anchor, proof, binding_sig)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Builds an output-only Orchard bundle (no spends).
///
/// Used by Shield and ShieldFromAssetLock transitions where funds enter
/// the shielded pool from transparent sources.
fn build_output_only_bundle(
    recipient: &OrchardAddress,
    amount: u64,
    memo: [u8; 36],
    proving_key: &ProvingKey,
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let payment_address = orchard_address_to_payment_address(recipient)?;
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

    prove_and_sign_bundle(builder, proving_key, &[], &[])
}

/// Builds a spend+output Orchard bundle.
///
/// Used by ShieldedTransfer, Unshield, and ShieldedWithdrawal where funds
/// are spent from existing notes.
fn build_spend_bundle(
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    output_amount: u64,
    memo: [u8; 36],
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    proving_key: &ProvingKey,
    extra_sighash_data: &[u8],
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let payment_address = orchard_address_to_payment_address(recipient)?;

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

    prove_and_sign_bundle(builder, proving_key, &[ask.clone()], extra_sighash_data)
}

/// Takes a configured Builder, generates the proof, computes the platform
/// sighash, and applies signatures.
fn prove_and_sign_bundle(
    builder: Builder<DashMemo>,
    proving_key: &ProvingKey,
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
        .create_proof(proving_key, &mut rng)
        .map_err(|e| ProtocolError::Generic(format!("failed to create proof: {:?}", e)))?;

    proven
        .apply_signatures(rng, sighash, signing_keys)
        .map_err(|e| ProtocolError::Generic(format!("failed to apply signatures: {:?}", e)))
}

// ---------------------------------------------------------------------------
// Public builder functions
// ---------------------------------------------------------------------------

/// Builds a Shield state transition (transparent platform addresses -> shielded pool).
///
/// Constructs an output-only Orchard bundle (no spends), proves it, signs the
/// transparent input witnesses, and returns a ready-to-broadcast `StateTransition`.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield
/// - `inputs` - Platform address inputs with their nonces and balances
/// - `fee_strategy` - How to deduct fees from the transparent inputs
/// - `signer` - Signs each input address witness (ECDSA)
/// - `user_fee_increase` - Fee multiplier (0 = 100% base fee)
/// - `proving_key` - Halo 2 proving key (cache with `OnceLock` — ~30s to build)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
pub fn build_shield_transition<S: Signer<PlatformAddress>>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    fee_strategy: AddressFundsFeeStrategy,
    signer: &S,
    user_fee_increase: UserFeeIncrease,
    proving_key: &ProvingKey,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let bundle = build_output_only_bundle(recipient, shield_amount, memo, proving_key)?;
    let (actions, flags, value_balance, anchor, proof, binding_sig) =
        serialize_authorized_bundle(&bundle);

    ShieldTransition::try_from_bundle_with_signer(
        inputs,
        actions,
        flags,
        value_balance,
        anchor,
        proof,
        binding_sig,
        fee_strategy,
        signer,
        user_fee_increase,
        platform_version,
    )
}

/// Builds a ShieldFromAssetLock state transition (core asset lock -> shielded pool).
///
/// Like Shield, constructs an output-only Orchard bundle. The funds come from
/// a core asset lock proof rather than platform address inputs.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield (from the asset lock)
/// - `asset_lock_proof` - Proof that funds are locked on core chain
/// - `asset_lock_private_key` - Private key for the asset lock (signs the transition)
/// - `user_fee_increase` - Fee multiplier (0 = 100% base fee)
/// - `proving_key` - Halo 2 proving key
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
pub fn build_shield_from_asset_lock_transition(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: crate::prelude::AssetLockProof,
    asset_lock_private_key: &[u8],
    user_fee_increase: UserFeeIncrease,
    proving_key: &ProvingKey,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let bundle = build_output_only_bundle(recipient, shield_amount, memo, proving_key)?;
    let (actions, flags, value_balance, anchor, proof, binding_sig) =
        serialize_authorized_bundle(&bundle);

    ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
        asset_lock_proof,
        asset_lock_private_key,
        actions,
        flags,
        value_balance,
        anchor,
        proof,
        binding_sig,
        user_fee_increase,
        platform_version,
    )
}

/// Builds a ShieldedTransfer state transition (shielded pool -> shielded pool).
///
/// Spends existing notes and creates a new note for the recipient. Any change
/// (total spent - transfer amount) is returned to the `change_address`.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `recipient` - Orchard address to receive the transferred note
/// - `transfer_amount` - Amount to transfer to the recipient
/// - `change_address` - Orchard address for change output (if any)
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Merkle root of the commitment tree
/// - `proving_key` - Halo 2 proving key
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
pub fn build_shielded_transfer_transition(
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    transfer_amount: u64,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    proving_key: &ProvingKey,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();
    if transfer_amount > total_spent {
        return Err(ProtocolError::Generic(format!(
            "transfer amount {} exceeds total spendable value {}",
            transfer_amount, total_spent
        )));
    }

    let change_amount = total_spent - transfer_amount;

    let recipient_payment = orchard_address_to_payment_address(recipient)?;

    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| ProtocolError::Generic(format!("failed to add spend: {:?}", e)))?;
    }

    // Primary output to recipient
    builder
        .add_output(
            None,
            recipient_payment,
            NoteValue::from_raw(transfer_amount),
            memo,
        )
        .map_err(|e| ProtocolError::Generic(format!("failed to add output: {:?}", e)))?;

    // Change output (if any)
    if change_amount > 0 {
        let change_payment = orchard_address_to_payment_address(change_address)?;
        builder
            .add_output(
                None,
                change_payment,
                NoteValue::from_raw(change_amount),
                [0u8; 36],
            )
            .map_err(|e| ProtocolError::Generic(format!("failed to add change output: {:?}", e)))?;
    }

    // ShieldedTransfer has no extra_data in sighash
    let bundle = prove_and_sign_bundle(builder, proving_key, &[ask.clone()], &[])?;
    let (actions, flags, value_balance, anchor_bytes, proof, binding_sig) =
        serialize_authorized_bundle(&bundle);

    // value_balance for shielded transfer is u64; cast from i64
    // (should be 0 when transfer_amount == total_spent, positive if fees leave pool)
    ShieldedTransferTransition::try_from_bundle(
        actions,
        flags,
        value_balance as u64,
        anchor_bytes,
        proof,
        binding_sig,
        platform_version,
    )
}

/// Builds an Unshield state transition (shielded pool -> platform address).
///
/// Spends existing notes and sends part of the value to a transparent platform
/// address. Any remaining value is returned to the shielded `change_address`.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `output_address` - Platform address to receive the unshielded funds
/// - `unshield_amount` - Amount to unshield to the platform address
/// - `change_address` - Orchard address for change output
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Merkle root of the commitment tree
/// - `proving_key` - Halo 2 proving key
/// - `memo` - 36-byte structured memo for the change output (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
pub fn build_unshield_transition(
    spends: Vec<SpendableNote>,
    output_address: PlatformAddress,
    unshield_amount: u64,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    proving_key: &ProvingKey,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();
    if unshield_amount > total_spent {
        return Err(ProtocolError::Generic(format!(
            "unshield amount {} exceeds total spendable value {}",
            unshield_amount, total_spent
        )));
    }

    let change_amount = total_spent - unshield_amount;

    // Unshield extra_data = output_address.to_bytes() || amount.to_le_bytes()
    let mut extra_sighash_data = output_address.to_bytes();
    extra_sighash_data.extend_from_slice(&unshield_amount.to_le_bytes());

    let bundle = build_spend_bundle(
        spends,
        change_address,
        change_amount,
        memo,
        fvk,
        ask,
        anchor,
        proving_key,
        &extra_sighash_data,
    )?;

    let (actions, flags, value_balance, anchor_bytes, proof, binding_sig) =
        serialize_authorized_bundle(&bundle);

    UnshieldTransition::try_from_bundle(
        output_address,
        unshield_amount,
        actions,
        flags,
        value_balance,
        anchor_bytes,
        proof,
        binding_sig,
        platform_version,
    )
}

/// Builds a ShieldedWithdrawal state transition (shielded pool -> core L1 address).
///
/// Spends existing notes and withdraws value to a core chain script output.
/// Any remaining value is returned to the shielded `change_address`.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `withdrawal_amount` - Amount to withdraw to the core chain
/// - `output_script` - Core chain script to receive the funds
/// - `core_fee_per_byte` - Core chain fee rate
/// - `pooling` - Withdrawal pooling strategy
/// - `change_address` - Orchard address for change output
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Merkle root of the commitment tree
/// - `proving_key` - Halo 2 proving key
/// - `memo` - 36-byte structured memo for the change output (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
pub fn build_shielded_withdrawal_transition(
    spends: Vec<SpendableNote>,
    withdrawal_amount: u64,
    output_script: CoreScript,
    core_fee_per_byte: u32,
    pooling: Pooling,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    proving_key: &ProvingKey,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();
    if withdrawal_amount > total_spent {
        return Err(ProtocolError::Generic(format!(
            "withdrawal amount {} exceeds total spendable value {}",
            withdrawal_amount, total_spent
        )));
    }

    let change_amount = total_spent - withdrawal_amount;

    // ShieldedWithdrawal extra_data = output_script.as_bytes() || amount.to_le_bytes()
    let mut extra_sighash_data = output_script.as_bytes().to_vec();
    extra_sighash_data.extend_from_slice(&withdrawal_amount.to_le_bytes());

    let bundle = build_spend_bundle(
        spends,
        change_address,
        change_amount,
        memo,
        fvk,
        ask,
        anchor,
        proving_key,
        &extra_sighash_data,
    )?;

    let (actions, flags, value_balance, anchor_bytes, proof, binding_sig) =
        serialize_authorized_bundle(&bundle);

    ShieldedWithdrawalTransition::try_from_bundle(
        withdrawal_amount,
        actions,
        flags,
        value_balance,
        anchor_bytes,
        proof,
        binding_sig,
        core_fee_per_byte,
        pooling,
        output_script,
        platform_version,
    )
}
