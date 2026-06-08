use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::serialization::Signable;
use crate::shielded::compute_shielded_identity_create_fee;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::shielded::OrchardBundleParams;
use crate::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::methods::IdentityCreateFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::{
    derive_identity_id_from_actions, identity_id_from_nullifiers,
    IdentityCreateFromShieldedPoolTransition,
};
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Output of [`build_identity_create_from_shielded_pool_transition`]: everything the SDK's
/// `IdentityCreateFromShieldedPool::identity_create_from_shielded_pool` broadcast helper needs.
///
/// The split (PoP-signed keys + bundle params, rather than a fully-built `StateTransition`) lets
/// the wallet feed the SDK helper directly — the helper re-assembles the transition via
/// `try_from_bundle`, which preserves the per-key proof-of-possession signatures already filled here.
pub struct IdentityCreateFromShieldedPoolBuildResult {
    /// The new identity's public keys with their per-key proof-of-possession signatures filled.
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    /// The serialized, authorized Orchard bundle (actions / anchor / proof / binding signature).
    pub bundle: OrchardBundleParams,
    /// The new identity's id (`double_sha256(sorted nullifiers)`), surfaced so the host can persist
    /// / display it without re-deriving.
    pub identity_id: Identifier,
    /// The client-predicted fee (in credits). The authoritative fee is metered at consensus.
    pub predicted_fee: Credits,
}

/// Builds an `IdentityCreateFromShieldedPool` (Type 20) state transition: spend shielded-pool
/// notes to fund a brand-new Platform identity.
///
/// The `denomination` (a member of the versioned exit-denomination set) leaves the pool EXACTLY —
/// the bundle's `value_balance` equals `denomination` (the ShieldedTransfer exact-equality model).
/// Any spent value above the denomination re-enters the pool as a single change note to
/// `change_address`. The metered fee is taken from the denomination at execution, so the new
/// identity is created holding `denomination - total_fee` (the fee is NOT subtracted from the
/// bundle here — only predicted for the caller's note-reservation math).
///
/// # Authorization
///
/// `IdentityCreateFromShieldedPool` carries NO platform identity signature. Authorization is 100%:
/// 1. the Orchard proof + per-action spend-auth signatures (the spender controls the spent notes),
/// 2. the RedPallas binding signature over the platform sighash, which commits the new identity id,
///    the denomination, and the FULL public-key set via
///    [`crate::shielded::identity_create_from_shielded_extra_sighash_data`] — so a relayer cannot
///    redirect the bundle to a different id or swap in keys it controls, and
/// 3. a per-key proof-of-possession signature over the transition's `signable_bytes`, proving the
///    creator holds every key being registered (mirrors `IdentityCreate`).
///
/// The new identity id is derived from the SORTED spend nullifiers
/// ([`derive_identity_id_from_actions`]) — fully determined by which notes are spent, so it is
/// known before the bundle is built and the same value is re-derived and checked at consensus.
///
/// # Parameters
/// - `public_keys` — the new identity's public keys, each paired with its
///   [`IdentityPublicKeyInCreation`] form (the latter goes into the transition; the former is used
///   only to look up the private key in `identity_signer`). The per-key proof-of-possession
///   signatures are filled by this function.
/// - `denomination` — the fixed exit amount (in credits) leaving the pool.
/// - `spends` — notes to spend with their Merkle paths. Their total MUST be `>= denomination`.
/// - `change_address` — Orchard address that receives the change note (`total_spent - denomination`).
/// - `fvk` / `ask` — the spender's full viewing key and spend-authorizing key (Orchard side).
/// - `anchor` — Sinsemilla root of the note commitment tree (Orchard Anchor).
/// - `prover` — Orchard prover (holds the Halo 2 proving key).
/// - `identity_signer` — produces each new key's proof-of-possession signature over the transition's
///   signable bytes.
/// - `memo` — 36-byte structured memo for the change output.
/// - `platform_version` — protocol version.
///
/// Returns the PoP-signed keys, the serialized Orchard bundle, the derived identity id, and the
/// client-predicted fee (in credits) — ready to feed the SDK's
/// `IdentityCreateFromShieldedPool::identity_create_from_shielded_pool` broadcast helper. The
/// authoritative fee is metered at consensus.
#[allow(clippy::too_many_arguments)]
pub async fn build_identity_create_from_shielded_pool_transition<P, S>(
    public_keys: Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)>,
    denomination: u64,
    spends: Vec<SpendableNote>,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    identity_signer: &S,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<IdentityCreateFromShieldedPoolBuildResult, ProtocolError>
where
    P: OrchardProver,
    S: Signer<IdentityPublicKey>,
{
    if denomination > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "denomination {} exceeds maximum allowed value {}",
            denomination,
            i64::MAX as u64
        )));
    }
    if public_keys.is_empty() {
        return Err(ProtocolError::ShieldedBuildError(
            "identity-create-from-shielded-pool requires at least one public key".to_string(),
        ));
    }

    // Checked: a large spend set could otherwise overflow u64 (release builds wrap silently).
    let total_spent = spends
        .iter()
        .try_fold(0u64, |acc, s| acc.checked_add(s.note.value().inner()))
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError(
                "identity-create-from-shielded-pool total spent value overflows u64".to_string(),
            )
        })?;
    if denomination > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "denomination {} exceeds total spendable value {}",
            denomination, total_spent
        )));
    }

    // The whole denomination leaves the pool; the excess re-enters as a single change note. There
    // is NO shielded recipient — the value funds the (transparent) new identity, not another note.
    // Cannot underflow: the `denomination > total_spent` guard above already rejected that case.
    let change_amount = total_spent - denomination;

    // Orchard's BundleType::DEFAULT pads single-spend bundles to a 2-action minimum, matching the
    // other spend-side builders. The fee predictor is only informational here (the metered fee at
    // execution is authoritative); we report it so the caller's reservation math lines up.
    let num_actions = spends.len().max(2);
    let fee =
        compute_shielded_identity_create_fee(num_actions, public_keys.len(), platform_version)?;

    // The id is derived from the SORTED spend nullifiers, which must be known BEFORE signing
    // because the id is part of the Orchard sighash. The nullifier of a spend is
    // `Note::nullifier(fvk)`, independent of bundle randomness, so compute them directly from the
    // spent notes — the same values the bundle will publish and consensus will re-derive from.
    let nullifiers: Vec<[u8; 32]> = spends
        .iter()
        .map(|s| s.note.nullifier(fvk).to_bytes())
        .collect();
    let identity_id = identity_id_from_nullifiers(&nullifiers);

    // Build the in-creation key list (transition order) and bind it — together with the id and the
    // denomination — into the Orchard sighash.
    let in_creation_keys: Vec<IdentityPublicKeyInCreation> =
        public_keys.iter().map(|(_, c)| c.clone()).collect();
    let extra_sighash_data = crate::shielded::identity_create_from_shielded_extra_sighash_data(
        &identity_id.to_buffer(),
        denomination,
        &in_creation_keys,
    );

    let bundle = build_spend_bundle(
        spends,
        change_address,
        change_amount,
        memo,
        fvk,
        ask,
        anchor,
        prover,
        &extra_sighash_data,
    )?;

    let sb = serialize_authorized_bundle(&bundle);

    // The consensus binding re-derives the id from the on-wire action nullifiers. Assert the
    // bundle's published nullifiers reduce to the same id we bound, so a mismatch is caught here
    // (cheap) rather than as an opaque InvalidShieldedProofError after the ~30 s proof.
    debug_assert_eq!(
        identity_id,
        derive_identity_id_from_actions(&sb.actions),
        "bound identity id must match the id re-derived from the bundle's published nullifiers"
    );

    // Build the transition (denomination == value_balance EXACTLY) with the unsigned key set, purely
    // to obtain the canonical signable bytes the per-key proofs-of-possession must sign.
    let mut state_transition = IdentityCreateFromShieldedPoolTransition::try_from_bundle(
        in_creation_keys,
        denomination,
        sb.actions.clone(),
        sb.anchor,
        sb.proof.clone(),
        sb.binding_signature,
        platform_version,
    )?;

    // Per-key proof-of-possession: each unique-type key signs the transition's signable bytes. The
    // signable form excludes the per-key signatures themselves (and the derived identity id), so the
    // bytes are stable across the signing loop — compute them once, mirroring `IdentityCreate`.
    let key_signable_bytes = state_transition.signable_bytes()?;

    let StateTransition::IdentityCreateFromShieldedPool(
        IdentityCreateFromShieldedPoolTransition::V0(v0),
    ) = &mut state_transition
    else {
        return Err(ProtocolError::ShieldedBuildError(
            "unexpected state transition variant after try_from_bundle".to_string(),
        ));
    };

    for (key_with_witness, (original_key, _)) in v0.public_keys.iter_mut().zip(public_keys.iter()) {
        if original_key.key_type().is_unique_key_type() {
            let signature = identity_signer
                .sign(original_key, &key_signable_bytes)
                .await?;
            key_with_witness.set_signature(signature);
        }
    }

    // Hand the PoP-signed keys + the bundle params back to the caller (the wallet), which feeds them
    // to the SDK broadcast helper. The helper re-assembles the transition via `try_from_bundle`,
    // preserving these signatures.
    let signed_public_keys = std::mem::take(&mut v0.public_keys);

    Ok(IdentityCreateFromShieldedPoolBuildResult {
        public_keys: signed_public_keys,
        bundle: OrchardBundleParams {
            actions: sb.actions,
            anchor: sb.anchor,
            proof: sb.proof,
            binding_signature: sb.binding_signature,
        },
        identity_id,
        predicted_fee: fee,
    })
}
