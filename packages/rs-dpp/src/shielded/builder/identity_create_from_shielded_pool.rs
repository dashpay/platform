use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::address_funds::PlatformAddress;
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

use super::{
    build_spend_bundle_with, serialize_authorized_bundle, shielded_bundle_action_count,
    OrchardProver, SpendableNote,
};

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
/// The new identity id is derived from the SORTED **published** action nullifiers
/// ([`derive_identity_id_from_actions`]) — including any padding action's dummy nullifier
/// (`BundleType::DEFAULT` pads single-spend bundles to a 2-action minimum), so it is only known
/// once the bundle's action set is fixed. It is derived inside the bundle-build hook, bound into
/// the Orchard sighash there, and the same value is re-derived and checked at consensus.
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
    send_to_address_on_creation_failure: PlatformAddress,
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

    // Reject a non-member denomination before any (expensive) proving — Type 20 exits are a
    // protocol-versioned fixed set, so an unsupported value would be rejected at `validate_structure`
    // after the Orchard proof anyway. Fail fast.
    let allowed_denominations = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .shielded_identity_create_denominations;
    if !allowed_denominations.contains(&denomination) {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "denomination {denomination} is not a member of the allowed exit-denomination set {allowed_denominations:?}"
        )));
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
    //
    // Routed through the shared predictor (1 shielded output — the change note), which is
    // numerically `spends.len().max(2)` AND enforces both consensus ceilings (the structural
    // action cap and the transition-size-derived one) BEFORE the ~30 s Halo 2 proof.
    let num_actions = shielded_bundle_action_count(spends.len(), 1, platform_version)?;
    let fee =
        compute_shielded_identity_create_fee(num_actions, public_keys.len(), platform_version)?;

    // The metered fee is carved from the denomination at execution; if the predicted fee already
    // meets/exceeds it, the new identity could not be created with a positive balance (consensus
    // rejects `total_fee >= denomination`). Fail fast rather than after proving.
    if fee >= denomination {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "predicted fee {fee} is not less than the denomination {denomination}; the new identity would have a non-positive balance"
        )));
    }

    // Build the in-creation key list (transition order) up front — it is bound, together with the
    // id and the denomination, into the Orchard sighash.
    let in_creation_keys: Vec<IdentityPublicKeyInCreation> =
        public_keys.iter().map(|(_, c)| c.clone()).collect();

    // The id is `double_sha256(sorted PUBLISHED nullifiers)`. The published set is only known once
    // the bundle is built: `BundleType::DEFAULT` pads a single-spend bundle with a dummy action
    // whose random nullifier goes on the wire, and consensus re-derives the id over ALL action
    // nullifiers (dummies are indistinguishable by design). So derive the id inside the
    // post-build hook — after the action set is fixed, before the sighash is bound.
    let mut bound_identity_id: Option<Identifier> = None;
    let bundle = build_spend_bundle_with(
        spends,
        change_address,
        change_amount,
        memo,
        fvk,
        ask,
        anchor,
        prover,
        |published_nullifiers| {
            let id = identity_id_from_nullifiers(published_nullifiers);
            let data = crate::shielded::identity_create_from_shielded_extra_sighash_data(
                &id.to_buffer(),
                denomination,
                &send_to_address_on_creation_failure,
                &in_creation_keys,
                platform_version,
            )?;
            bound_identity_id = Some(id);
            Ok(data)
        },
    )?;
    let identity_id = bound_identity_id.ok_or_else(|| {
        ProtocolError::ShieldedBuildError(
            "identity id was not derived during bundle build".to_string(),
        )
    })?;

    let sb = serialize_authorized_bundle(&bundle);

    // The consensus binding re-derives the id from the on-wire action nullifiers. Assert the
    // bundle's published nullifiers reduce to the same id we bound, so a mismatch is caught here
    // (cheap) rather than as an opaque InvalidShieldedProofError after the ~30 s proof.
    if identity_id != derive_identity_id_from_actions(&sb.actions) {
        return Err(ProtocolError::ShieldedBuildError(
            "bound identity id does not match the id re-derived from the bundle's published \
             nullifiers"
                .to_string(),
        ));
    }

    // Build the transition (denomination == value_balance EXACTLY) with the unsigned key set, purely
    // to obtain the canonical signable bytes the per-key proofs-of-possession must sign.
    let mut state_transition = IdentityCreateFromShieldedPoolTransition::try_from_bundle(
        in_creation_keys,
        denomination,
        send_to_address_on_creation_failure,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressWitness;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use grovedb_commitment_tree::{
        ExtractedNoteCommitment, Hashable, MerkleHashOrchard, MerklePath, SpendingKey,
        NOTE_COMMITMENT_TREE_DEPTH,
    };
    use platform_value::BinaryData;

    /// A dummy PoP signer producing a fixed 65-byte signature. The builder fills (and does not
    /// verify) the proof-of-possession signatures, so a stub is enough to exercise the pipeline.
    #[derive(Debug)]
    struct DummySigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for DummySigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![0u8; 65]))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::ShieldedBuildError(
                "identity PoP signer never creates address witnesses".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// One AUTHENTICATION/MASTER ECDSA key in both forms the builder takes.
    fn key_pair(id: u32) -> (IdentityPublicKey, IdentityPublicKeyInCreation) {
        let public = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0xAB; 33]),
            disabled_at: None,
        });
        let in_creation = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0xAB; 33]),
            signature: BinaryData::new(vec![]),
        });
        (public, in_creation)
    }

    /// 0.1 DASH in credits — the smallest member of the versioned exit-denomination set.
    const DENOMINATION: u64 = 10_000_000_000;

    /// The padded-bundle regression test for the dummy-nullifier bug: a SINGLE-spend bundle is
    /// padded by `BundleType::DEFAULT` to the 2-action minimum, and the padding action's random
    /// dummy nullifier is published on the wire. The identity id MUST be derived from the FULL
    /// published set (what consensus re-derives), not from the real spends alone — pre-fix this
    /// build failed its own post-proving id-consistency check on every 1-note spend.
    #[tokio::test]
    async fn single_spend_padded_bundle_derives_id_from_published_nullifiers() {
        let platform_version = PlatformVersion::latest();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let change_address = test_orchard_address();

        // A valid (note, path, anchor) triple: the anchor is the root the witness computes over
        // the note's commitment — the same trick `extract_spends_and_anchor` uses in production.
        let spend = test_spendable_note(12_000_000_000);
        let cmx = ExtractedNoteCommitment::from(spend.note.commitment());
        let anchor = spend.merkle_path.root(cmx);
        let real_nullifier = spend.note.nullifier(&fvk).to_bytes();

        let result = build_identity_create_from_shielded_pool_transition(
            vec![key_pair(0)],
            DENOMINATION,
            PlatformAddress::P2pkh([0u8; 20]),
            vec![spend],
            &change_address,
            &fvk,
            &ask,
            anchor,
            &TestProver,
            &DummySigner,
            [0u8; 36],
            platform_version,
        )
        .await
        .expect("a single-spend (padded) build must succeed");

        assert_eq!(
            result.bundle.actions.len(),
            2,
            "a single spend must be padded to the 2-action minimum"
        );
        assert!(
            result
                .bundle
                .actions
                .iter()
                .any(|action| action.nullifier == real_nullifier),
            "the real spend's nullifier must be among the published actions"
        );
        // The id must equal the consensus re-derivation over ALL published nullifiers…
        assert_eq!(
            result.identity_id,
            derive_identity_id_from_actions(&result.bundle.actions),
            "identity id must match the consensus derivation over the published actions"
        );
        // …and must NOT equal the real-spends-only derivation (the pre-fix behavior): the padding
        // action's dummy nullifier participates.
        assert_ne!(
            result.identity_id,
            identity_id_from_nullifiers(&[real_nullifier]),
            "the padding action's dummy nullifier must participate in the id derivation"
        );
        // …which is precisely what `shielded_identity_id_is_reproducible` reports: with one real
        // spend the published set contains fresh randomness, so the id cannot be re-derived
        // offline (a retry would build a different dummy and thus a different id).
        assert!(
            !crate::state_transition::identity_create_from_shielded_pool_transition::shielded_identity_id_is_reproducible(1),
            "a single-spend bundle is padded, so its id must be reported as NOT reproducible"
        );
        assert!(
            result.predicted_fee < DENOMINATION,
            "predicted fee must leave the new identity a positive balance"
        );
    }

    /// Complement: with two real spends (no padding needed), the published set IS the real set,
    /// so the id equals the real-nullifiers-only derivation.
    #[tokio::test]
    async fn two_spend_unpadded_bundle_id_matches_real_nullifier_derivation() {
        let platform_version = PlatformVersion::latest();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let change_address = test_orchard_address();

        // Two distinct notes (different values → different commitments/nullifiers) witnessed in
        // one two-leaf tree: each path's level-0 sibling is the other leaf, upper siblings shared,
        // so both witnesses compute the SAME root — a consistent shared anchor.
        let note_a = test_spendable_note(6_000_000_000).note;
        let note_b = test_spendable_note(7_000_000_000).note;
        let cmx_a = ExtractedNoteCommitment::from(note_a.commitment());
        let cmx_b = ExtractedNoteCommitment::from(note_b.commitment());

        let mut auth_path_a = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        auth_path_a[0] = MerkleHashOrchard::from_cmx(&cmx_b);
        let mut auth_path_b = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        auth_path_b[0] = MerkleHashOrchard::from_cmx(&cmx_a);
        let path_a = MerklePath::from_parts(0, auth_path_a);
        let path_b = MerklePath::from_parts(1, auth_path_b);

        let anchor = path_a.root(cmx_a);
        assert_eq!(
            anchor.to_bytes(),
            path_b.root(cmx_b).to_bytes(),
            "both witnesses must compute the same anchor"
        );

        let nf_a = note_a.nullifier(&fvk).to_bytes();
        let nf_b = note_b.nullifier(&fvk).to_bytes();
        let spends = vec![
            SpendableNote {
                note: note_a,
                merkle_path: path_a,
            },
            SpendableNote {
                note: note_b,
                merkle_path: path_b,
            },
        ];

        let result = build_identity_create_from_shielded_pool_transition(
            vec![key_pair(0)],
            DENOMINATION,
            PlatformAddress::P2pkh([0u8; 20]),
            spends,
            &change_address,
            &fvk,
            &ask,
            anchor,
            &TestProver,
            &DummySigner,
            [0u8; 36],
            platform_version,
        )
        .await
        .expect("a two-spend build must succeed");

        assert_eq!(
            result.bundle.actions.len(),
            2,
            "two spends + one change output need no padding"
        );
        assert_eq!(
            result.identity_id,
            derive_identity_id_from_actions(&result.bundle.actions),
            "identity id must match the consensus derivation over the published actions"
        );
        assert_eq!(
            result.identity_id,
            identity_id_from_nullifiers(&[nf_a, nf_b]),
            "with no padding, the published set is exactly the real spends' nullifiers"
        );
        // …which is precisely what `shielded_identity_id_is_reproducible` reports: with two real
        // spends no padding is added, so the id is a pure function of the spent notes and a retry
        // re-derives the SAME id. This is the property two-note funding buys.
        assert!(
            crate::state_transition::identity_create_from_shielded_pool_transition::shielded_identity_id_is_reproducible(2),
            "a two-spend bundle needs no padding, so its id must be reported as reproducible"
        );
    }
}
