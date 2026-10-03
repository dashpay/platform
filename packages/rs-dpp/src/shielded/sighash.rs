//! Platform sighash preimage construction for shielded transitions.
//!
//! Shielded transitions carry NO platform identity signature — authorization is the Orchard proof +
//! per-action spend-auth signatures + the RedPallas binding signature over the platform sighash.
//! These helpers build the transparent `extra_data` each transition binds into that sighash so the
//! signing (client/builder) and verifying (consensus) sides commit to identical bytes. The byte
//! layouts are consensus-critical and versioned via `dpp.methods.shielded_extra_sighash_data`; the
//! credit pool's outputs-only bundles via `dpp.methods.credit_pool_bundle_binding`.

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::AddressNonce;
use crate::shielded::{serialized_actions_digest, SerializedAction};
use crate::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Domain separator for Platform sighash computation.
const SIGHASH_DOMAIN: &[u8] = b"DashPlatformSighash";

/// The state transition type byte the token bundle of a `TokenShieldedTransferWithShieldedFee`
/// commits to (`StateTransitionType::TokenShieldedTransferWithShieldedFee`).
pub const TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE: u8 = 26;
/// The state transition type byte the token bundle of a `TokenUnshieldWithShieldedFee` commits to.
pub const TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE: u8 = 27;
/// The state transition type byte the token bundle of a `TokenPurchaseFromShieldedPool` commits to.
pub const TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE: u8 = 28;

/// Domain tag an outputs-only token pool bundle commits to. Unlike the three constants above
/// these are not `StateTransitionType` bytes — every one of these bundles rides inside a batch
/// transition — yet they share a preimage slot with them at the same length. They are drawn
/// from a high range that space has not reached, which nothing in the type system enforces:
/// `StateTransitionType` is `repr(u8)` and could be given one of these bytes. What holds the
/// reservation is `outputs_only_token_pool_tags_cannot_collide_with_state_transition_types`,
/// which asks the enum and fails the build's tests the day one is assigned here.
pub const TOKEN_SHIELD_BUNDLE_TAG: u8 = 0x80;
/// Domain tag of a `TokenMintToPool` bundle. See [`TOKEN_SHIELD_BUNDLE_TAG`].
pub const TOKEN_MINT_TO_POOL_BUNDLE_TAG: u8 = 0x81;
/// Domain tag of a `TokenClaimToPool` bundle. See [`TOKEN_SHIELD_BUNDLE_TAG`].
pub const TOKEN_CLAIM_TO_POOL_BUNDLE_TAG: u8 = 0x82;
/// Domain tag of a `TokenDirectPurchaseToPool` bundle. See [`TOKEN_SHIELD_BUNDLE_TAG`].
pub const TOKEN_DIRECT_PURCHASE_TO_POOL_BUNDLE_TAG: u8 = 0x83;

/// Domain tag of a credit pool `Shield` bundle. The credit pool's outputs-only tags continue the
/// token pool range above: they share the same preimage slot at the same length, so every tag
/// in both sets must stay distinct from each other and from every `StateTransitionType` byte.
/// `credit_pool_outputs_only_tags_cannot_collide_with_state_transition_types` and
/// `outputs_only_bundle_tags_are_pairwise_distinct` hold both reservations.
pub const SHIELD_BUNDLE_TAG: u8 = 0x84;
/// Domain tag of a `ShieldFromIdentity` bundle. See [`SHIELD_BUNDLE_TAG`].
pub const SHIELD_FROM_IDENTITY_BUNDLE_TAG: u8 = 0x85;
/// Domain tag of a `ShieldFromAssetLock` bundle. See [`SHIELD_BUNDLE_TAG`].
pub const SHIELD_FROM_ASSET_LOCK_BUNDLE_TAG: u8 = 0x86;

/// Computes the platform sighash from an Orchard bundle commitment and optional
/// transparent field data.
///
/// The sighash is computed as:
///   `SHA-256(SIGHASH_DOMAIN || bundle_commitment || extra_data)`
///
/// This binds transparent state transition fields (like `output_address` in unshield
/// or `output_script` in shielded withdrawal) to the Orchard signatures, preventing
/// replay attacks where an attacker substitutes transparent fields while reusing a
/// valid Orchard bundle.
///
/// It also binds a bundle that has no transparent fields to the one context it was proved
/// for, which an outputs-only bundle cannot do on its own: having no spends, its anchor is
/// never checked against a pool — the client builds it against the empty tree — so it verifies
/// against every pool.
///
/// The same computation must be used on both the signing (client) and verification (platform)
/// sides. `extra_data` is empty only for the credit pool's `ShieldedTransfer`, and for the credit
/// pool's outputs-only bundles at protocol versions that predate their binding (see
/// [`shield_extra_sighash_data`]); each other transition has a builder in this module that
/// spells out its layout. `ShieldedTransfer` is the one that needs no layout of its own: it
/// spends, so it carries an anchor and nullifiers that pin it to one pool and one set of notes.
pub fn compute_platform_sighash(bundle_commitment: &[u8; 32], extra_data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SIGHASH_DOMAIN);
    hasher.update(bundle_commitment);
    hasher.update(extra_data);
    hasher.finalize().into()
}

/// Builds the transparent `extra_data` bound into a ShieldedWithdrawal's platform
/// sighash, with the byte layout
/// `output_script || unshielding_amount (u64 LE) || core_fee_per_byte (u32 LE) || pooling (u8)`.
///
/// Every field here is written verbatim by the transformer into the queued withdrawal
/// document that constructs the Core asset-unlock TxOut. Binding all of them into the
/// Orchard sighash means the binding signature authorizes them: since ShieldedWithdrawal
/// has no identity-key signature and no address-witness check, the Orchard signature is
/// the only authorization boundary, so a relay or block proposer cannot malleate
/// `core_fee_per_byte` (or `pooling`, were it ever unpinned from `Never`) — e.g. flip a
/// user's `core_fee_per_byte = 1` to a much larger Fibonacci value to redirect the
/// withdrawn amount into L1 miner fees — without invalidating the proof.
///
/// The signing (client/builder) and verifying (consensus) sides MUST produce identical
/// bytes, so both call this single function.
///
/// The layout places the variable-length `output_script` first with no length prefix. This
/// is unambiguous only because `validate_structure` runs before proof verification and pins
/// `output_script` to a canonical, fixed-length P2PKH (25 bytes) or P2SH (23 bytes); the
/// remaining fields are fixed-width, so the preimage is well-defined for every accepted
/// transition. If that script-shape restriction is ever relaxed, add a length prefix here.
/// Dispatches on the platform-versioned `dpp.methods.shielded_extra_sighash_data` so the
/// consensus-critical byte layout can evolve across protocol versions without breaking older
/// transitions — the same versioning the sibling shielded fee methods use. The signing
/// (client/builder) and verifying (consensus) sides both call this single function with the same
/// `platform_version`, so they can never produce divergent preimages.
pub fn shielded_withdrawal_extra_sighash_data(
    output_script: &[u8],
    unshielding_amount: u64,
    core_fee_per_byte: u32,
    pooling: Pooling,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(shielded_withdrawal_extra_sighash_data_v0(
            output_script,
            unshielding_amount,
            core_fee_per_byte,
            pooling,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "shielded_withdrawal_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`shielded_withdrawal_extra_sighash_data`] (see that function's doc comment for
/// the layout and rationale). Frozen: never mutate; a layout change requires a new `_v1` + version.
pub fn shielded_withdrawal_extra_sighash_data_v0(
    output_script: &[u8],
    unshielding_amount: u64,
    core_fee_per_byte: u32,
    pooling: Pooling,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(output_script.len() + 8 + 4 + 1);
    data.extend_from_slice(output_script);
    data.extend_from_slice(&unshielding_amount.to_le_bytes());
    data.extend_from_slice(&core_fee_per_byte.to_le_bytes());
    data.push(pooling as u8);
    data
}

/// Builds the transparent `extra_data` bound into an Unshield's platform sighash, with the
/// byte layout `output_address || unshielding_amount (u64 LE)`.
///
/// As with [`shielded_withdrawal_extra_sighash_data`], the signing (client/builder) and
/// verifying (consensus) sides MUST produce identical bytes, so both call this single
/// function. Unshield credits a transparent platform address (not a Core asset-unlock
/// `TxOut`), so it carries no `core_fee_per_byte`/`pooling` to bind.
pub fn unshield_extra_sighash_data(
    output_address: &[u8],
    unshielding_amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(unshield_extra_sighash_data_v0(
            output_address,
            unshielding_amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "unshield_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`unshield_extra_sighash_data`] (see that function's doc comment for the layout
/// and rationale). Frozen: never mutate; a layout change requires a new `_v1` + version bump.
pub fn unshield_extra_sighash_data_v0(output_address: &[u8], unshielding_amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(output_address.len() + 8);
    data.extend_from_slice(output_address);
    data.extend_from_slice(&unshielding_amount.to_le_bytes());
    data
}

/// Builds the transparent `extra_data` bound into an `IdentityTopUpFromShieldedPool`'s platform
/// sighash, with the byte layout `identity_id (32) || top_up_amount (u64 LE)`.
///
/// Like `Unshield`, the transition carries no platform signature, so the state-determining
/// transparent fields (which identity is credited, and the gross amount leaving the pool) must be
/// committed into the Orchard binding sighash; otherwise a relayer could take a valid spend bundle
/// and re-point it at a different identity. The client builder and the consensus verifier both
/// call this single function.
pub fn identity_top_up_from_shielded_extra_sighash_data(
    identity_id: &[u8; 32],
    top_up_amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(identity_top_up_from_shielded_extra_sighash_data_v0(
            identity_id,
            top_up_amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "identity_top_up_from_shielded_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`identity_top_up_from_shielded_extra_sighash_data`]. Frozen: never mutate;
/// a layout change requires a new `_v1` + version bump.
pub fn identity_top_up_from_shielded_extra_sighash_data_v0(
    identity_id: &[u8; 32],
    top_up_amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 8);
    data.extend_from_slice(identity_id);
    data.extend_from_slice(&top_up_amount.to_le_bytes());
    data
}

/// Builds the transparent `extra_data` bound into an `IdentityCreateFromShieldedPool`'s platform
/// sighash, with the byte layout
/// `identity_id (32) || denomination (u64 LE)
///   || send_to_address_on_creation_failure (tag u8: 0=P2pkh, 1=P2sh || hash 20)
///   || num_keys (u16 LE)
///   || for each key in supplied order: key_id (u32 LE) || purpose (u8) || security_level (u8)
///   || key_type (u8) || key_data_len (u16 LE) || key_data || read_only (u8)
///   || contract_bounds (tag u8: 0=None, 1=SingleContract id(32), 2=SingleContractDocumentType
///   id(32) name_len(u16 LE) name, 3=ContractGroup id(32))`.
///
/// Tag 3 is never reached: `IdentityCreateFromShieldedPool` refuses a key bound to a contract
/// group before this preimage is built (consensus in `validate_shielded_proof` v1, the builder
/// up front). The arm only keeps the encoder total without a panic on a block-execution path,
/// so the v0 bytes of every reachable input are unchanged.
///
/// The budget and the expiry of a version 1 key are not in the layout either, and for the same
/// reason never need to be: a key that carries either is refused at the same two places, so
/// every key that reaches this preimage is fully described by the fields above. A version 1 key
/// without limits binds the same bytes as its version 0 equivalent.
///
/// `IdentityCreateFromShieldedPool` carries NO platform identity signature: authorization is 100%
/// the Orchard proof + per-action spend-auth signatures + binding signature over this sighash. The
/// transparent, state-determining fields — the new identity id, the exit denomination, and the
/// FULL public-key set — must therefore be committed into the Orchard sighash, exactly as the
/// `surplus_output` field is committed into `ShieldFromAssetLock`'s ECDSA signature. Without this
/// binding a relay or block proposer could take a valid bundle exiting a denomination and re-point
/// it at a DIFFERENT identity id, or swap in DIFFERENT keys they control, stealing the credited
/// balance (the per-key proofs-of-possession alone do NOT prevent this — a relayer keeps valid PoP
/// sigs for their own keys while swapping the bundle). Binding `(this spend → these exact keys →
/// this id → this denomination)` here makes the redirection atomic-or-invalid.
///
/// The signing (client/builder) and verifying (consensus) sides MUST produce identical bytes, so
/// both call this single function. Unlike the fixed-length withdrawal/unshield helpers, the
/// variable-length key list is fully length-prefixed (both the key count and each key's data) so
/// the preimage is unambiguous for any key set.
pub fn identity_create_from_shielded_extra_sighash_data(
    identity_id: &[u8; 32],
    denomination: u64,
    send_to_address_on_creation_failure: &PlatformAddress,
    public_keys: &[IdentityPublicKeyInCreation],
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(identity_create_from_shielded_extra_sighash_data_v0(
            identity_id,
            denomination,
            send_to_address_on_creation_failure,
            public_keys,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "identity_create_from_shielded_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`identity_create_from_shielded_extra_sighash_data`] (see that function's doc
/// comment for the layout and rationale). Frozen: never mutate; a layout change requires a new `_v1`
/// + version bump.
pub fn identity_create_from_shielded_extra_sighash_data_v0(
    identity_id: &[u8; 32],
    denomination: u64,
    send_to_address_on_creation_failure: &PlatformAddress,
    public_keys: &[IdentityPublicKeyInCreation],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 8 + 21 + 2 + public_keys.len() * 44);
    data.extend_from_slice(identity_id);
    data.extend_from_slice(&denomination.to_le_bytes());
    // Bind the fallback address (type tag || 20-byte hash) so a relayer cannot redirect the
    // failure credit. Mirrors the way `unshield`/`withdrawal` bind their output address.
    match send_to_address_on_creation_failure {
        PlatformAddress::P2pkh(hash) => {
            data.push(0u8);
            data.extend_from_slice(hash);
        }
        PlatformAddress::P2sh(hash) => {
            data.push(1u8);
            data.extend_from_slice(hash);
        }
    }
    data.extend_from_slice(&(public_keys.len() as u16).to_le_bytes());
    for key in public_keys {
        data.extend_from_slice(&key.id().to_le_bytes());
        data.push(key.purpose() as u8);
        data.push(key.security_level() as u8);
        data.push(key.key_type() as u8);
        let key_data = key.data().as_slice();
        data.extend_from_slice(&(key_data.len() as u16).to_le_bytes());
        data.extend_from_slice(key_data);
        // Also bind `read_only` and `contract_bounds`. These are state-determining key fields that
        // ARE in the transition's signable_bytes, but the per-key proof-of-possession does NOT bind
        // them for hash-based key types (which accept an empty signature). Committing them into the
        // Orchard binding sighash makes them un-malleable for EVERY key type, so a relayer/proposer
        // cannot flip `read_only` or alter `contract_bounds` on an observed transition.
        data.push(key.read_only() as u8);
        match key.contract_bounds() {
            None => data.push(0u8),
            Some(ContractBounds::SingleContract { id }) => {
                data.push(1u8);
                data.extend_from_slice(id.as_bytes());
            }
            Some(ContractBounds::SingleContractDocumentType {
                id,
                document_type_name,
            }) => {
                data.push(2u8);
                data.extend_from_slice(id.as_bytes());
                let name = document_type_name.as_bytes();
                data.extend_from_slice(&(name.len() as u16).to_le_bytes());
                data.extend_from_slice(name);
            }
            Some(ContractBounds::ContractGroup { id }) => {
                // Unreachable: refused before the preimage is built (see the layout doc).
                data.push(3u8);
                data.extend_from_slice(id.as_bytes());
            }
        }
    }
    data
}

/// Builds the transparent `extra_data` bound into a `TokenUnshield`'s platform sighash, with the
/// byte layout `token_id (32) || owner_id (32) || recipient_id (32) || amount (u64 LE)`.
///
/// A token unshield rides inside an identity-signed batch, but the note owner who authorizes
/// the Orchard spend is not necessarily that identity. The spend-auth and binding signatures
/// must therefore commit to the pool the notes leave (`token_id`), the identity paying the
/// credits fee and submitting the batch (`owner_id`), the identity credited (`recipient_id`)
/// and the gross amount, so a bundle observed in the mempool cannot be re-wrapped by another
/// submitter to a different recipient or against another token's pool (every token pool
/// starts from the same empty-tree anchor).
pub fn token_unshield_extra_sighash_data(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    recipient_id: &[u8; 32],
    amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_unshield_extra_sighash_data_v0(
            token_id,
            owner_id,
            recipient_id,
            amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_unshield_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`token_unshield_extra_sighash_data`]. Frozen: never mutate; a layout
/// change requires a new `_v1` + version bump.
pub fn token_unshield_extra_sighash_data_v0(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    recipient_id: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 32 + 32 + 8);
    data.extend_from_slice(token_id);
    data.extend_from_slice(owner_id);
    data.extend_from_slice(recipient_id);
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Extra sighash data of a batch `TokenBurnFromPool`: the token id, the burner and the amount
/// destroyed, so a bundle proven for one burn cannot be replayed for another token, burner or
/// amount (72 bytes; no other layout has that length).
///
/// `burner_id` is the identity the burn is attributed to: the batch owner of a direct burn, or
/// the proposer of a group action burn. A group action pins the digest of the bundle's actions
/// (spend authorization signatures included), so every other signer submits the proposer's
/// bundle unchanged and the sighash must not depend on whose batch carries it.
pub fn token_burn_from_pool_extra_sighash_data(
    token_id: &[u8; 32],
    burner_id: &[u8; 32],
    amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_burn_from_pool_extra_sighash_data_v0(
            token_id, burner_id, amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_burn_from_pool_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `token_id (32) || burner_id (32) || amount (8, little endian)`.
pub fn token_burn_from_pool_extra_sighash_data_v0(
    token_id: &[u8; 32],
    burner_id: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 32 + 8);
    data.extend_from_slice(token_id);
    data.extend_from_slice(burner_id);
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Extra sighash data of a document action paid from a token shielded pool
/// (`TokenPaymentInfo::V1`): the token id, the batch owner, the document's contract and id
/// and the amount paid, so a bundle proven for one document cannot be replayed for another
/// document, batch owner, token or cost.
pub fn document_token_payment_extra_sighash_data(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    data_contract_id: &[u8; 32],
    document_id: &[u8; 32],
    amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(document_token_payment_extra_sighash_data_v0(
            token_id,
            owner_id,
            data_contract_id,
            document_id,
            amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "document_token_payment_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `token_id (32) || owner_id (32) || data_contract_id (32) ||
/// document_id (32) || amount (8, little endian)`. Frozen: never mutate; a layout change
/// requires a new `_v1` + version bump.
pub fn document_token_payment_extra_sighash_data_v0(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    data_contract_id: &[u8; 32],
    document_id: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 * 4 + 8);
    data.extend_from_slice(token_id);
    data.extend_from_slice(owner_id);
    data.extend_from_slice(data_contract_id);
    data.extend_from_slice(document_id);
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Extra sighash data of the token bundle of a `TokenShieldedTransferWithShieldedFee`: the
/// state transition type and the token id, so the bundle is pinned to one token pool and one
/// transition kind.
pub fn token_shielded_transfer_with_shielded_fee_extra_sighash_data(
    token_id: &[u8; 32],
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_shielded_transfer_with_shielded_fee_extra_sighash_data_v0(token_id)),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_shielded_transfer_with_shielded_fee_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `state transition type (1, = 26) || token_id (32)`. Frozen.
pub fn token_shielded_transfer_with_shielded_fee_extra_sighash_data_v0(
    token_id: &[u8; 32],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32);
    data.push(TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE);
    data.extend_from_slice(token_id);
    data
}

/// Extra sighash data of the token bundle of a `TokenUnshieldWithShieldedFee`: the state
/// transition type, the token id, the recipient and the amount, so the bundle cannot be
/// replayed against another token, recipient or amount.
pub fn token_unshield_with_shielded_fee_extra_sighash_data(
    token_id: &[u8; 32],
    recipient_id: &[u8; 32],
    amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_unshield_with_shielded_fee_extra_sighash_data_v0(
            token_id,
            recipient_id,
            amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_unshield_with_shielded_fee_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `state transition type (1, = 27) || token_id (32) || recipient_id (32)
/// || amount (8, little endian)`. Frozen.
pub fn token_unshield_with_shielded_fee_extra_sighash_data_v0(
    token_id: &[u8; 32],
    recipient_id: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32 + 32 + 8);
    data.push(TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE);
    data.extend_from_slice(token_id);
    data.extend_from_slice(recipient_id);
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Extra sighash data of the token bundle of a `TokenPurchaseFromShieldedPool`: the state
/// transition type, the token id, the token count and the agreed price.
pub fn token_purchase_from_shielded_pool_extra_sighash_data(
    token_id: &[u8; 32],
    token_count: u64,
    total_agreed_price: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_purchase_from_shielded_pool_extra_sighash_data_v0(
            token_id,
            token_count,
            total_agreed_price,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_purchase_from_shielded_pool_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `state transition type (1, = 28) || token_id (32) || token_count (8, LE)
/// || total_agreed_price (8, LE)`. Frozen.
pub fn token_purchase_from_shielded_pool_extra_sighash_data_v0(
    token_id: &[u8; 32],
    token_count: u64,
    total_agreed_price: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32 + 8 + 8);
    data.push(TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE);
    data.extend_from_slice(token_id);
    data.extend_from_slice(&token_count.to_le_bytes());
    data.extend_from_slice(&total_agreed_price.to_le_bytes());
    data
}

/// Extra sighash data of the credit pool fee bundle of an identity-less token pool transition:
/// the state transition type, the token id and a digest of the token bundle's actions, so the
/// fee bundle can only ever pay for that exact token bundle.
pub fn token_pool_fee_bundle_extra_sighash_data(
    state_transition_type: u8,
    token_id: &[u8; 32],
    token_actions: &[SerializedAction],
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_pool_fee_bundle_extra_sighash_data_v0(
            state_transition_type,
            token_id,
            &serialized_actions_digest(token_actions),
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_pool_fee_bundle_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `state transition type (1) || token_id (32) || token actions digest (32)`.
/// Frozen.
pub fn token_pool_fee_bundle_extra_sighash_data_v0(
    state_transition_type: u8,
    token_id: &[u8; 32],
    token_actions_digest: &[u8; 32],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32 + 32);
    data.push(state_transition_type);
    data.extend_from_slice(token_id);
    data.extend_from_slice(token_actions_digest);
    data
}

/// Builds the transparent `extra_data` bound into a `TokenShieldedTransfer`'s platform sighash,
/// with the byte layout `token_id (32) || owner_id (32)`.
///
/// Nothing leaves the pool, so there is no amount or destination to bind, but the bundle is
/// still pinned to one token pool and to the identity that pays for it: the same bundle
/// cannot be resubmitted under another fee payer, and the pool it spends from is explicit.
pub fn token_shielded_transfer_extra_sighash_data(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_shielded_transfer_extra_sighash_data_v0(
            token_id, owner_id,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_shielded_transfer_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`token_shielded_transfer_extra_sighash_data`]. Frozen: never mutate; a
/// layout change requires a new `_v1` + version bump.
pub fn token_shielded_transfer_extra_sighash_data_v0(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(token_id);
    data.extend_from_slice(owner_id);
    data
}

/// Extra sighash data of an outputs-only token pool bundle — `TokenShield`, `TokenMintToPool`,
/// `TokenClaimToPool` and `TokenDirectPurchaseToPool`: the bundle's domain tag, the token id
/// and the identity the bundle is attributed to.
///
/// These bundles have no spends, so nothing pins them to a pool: their anchor is never checked
/// against one — the client builds it against the empty tree — and the proof and binding
/// signature verify against any pool. Without this data the authorized bundle bytes are a
/// free-standing, self-verifying object that anybody can lift out of the mempool into a
/// transition of their own. Each of the three fields closes one direction of that:
///
/// - **The token id** keeps a bundle out of every other token's pool. That pool has its own
///   nullifier tree, so a copy there would leave both notes spendable; the harm is that the
///   recipient would hold notes derived from one `rho` in two pools, which links their spends
///   across pools.
/// - **The tag** keeps the kinds' preimages disjoint in the same pool. Bundles of different
///   kinds are otherwise indistinguishable to the proof — same flags, same empty-tree anchor,
///   and the same value balance whenever the amounts match. A same-pool reuse is also caught by
///   the nullifier record described below.
/// - **The owner** keeps it out of every other identity's transition of the same kind into the
///   same pool. Without it a copier could take a bundle from the mempool and land it ahead of
///   its author, funding the notes the author built with its own tokens, credits or claim, or
///   minting them with its own authority. The author's own transition would then be refused on
///   its recorded dummy nullifier and charged its fee.
///
/// `owner_id` is the batch owner, except for a group action mint, where it is the group
/// action's proposer: a group action pins the digest of the bundle's actions, so every other
/// signer submits the proposer's bundle unchanged and the preimage must not depend on whose
/// batch carries it. `TokenShield`, `TokenClaimToPool` and `TokenDirectPurchaseToPool` are
/// never group actions.
///
/// The preimage does not stop the owner from submitting its own bundle twice. That, and any
/// other repeat of a bundle into the same pool, consensus refuses on the state side: every
/// token pool write that takes an outputs-only bundle records the bundle's dummy nullifiers in
/// the pool's nullifier tree, and a bundle whose dummy nullifier is already there is refused
/// with `NullifierAlreadySpentError`. A repeat that got through would land a second note with
/// the same commitment and the same `rho`, hence the same nullifier, of which only one could
/// ever be spent.
pub fn token_pool_output_only_extra_sighash_data(
    action_type: TokenTransitionActionType,
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_pool_output_only_extra_sighash_data_v0(
            outputs_only_bundle_tag_v0(action_type)?,
            token_id,
            owner_id,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_pool_output_only_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// The v0 domain tag of an outputs-only token pool bundle. Frozen: the mapping is part of the
/// sighash preimage, so a kind keeps its byte forever and a new kind takes a new one.
///
/// The bytes are written out here rather than derived from the variant's position, so the
/// preimage does not depend on the enum's layout at all. `TokenTransitionActionType` is marked
/// append-only, but that marker is a convention for clients; resting a consensus preimage on it
/// would turn an accidental reordering into a silent wire change.
///
/// Every other kind is named rather than swept up by a catch-all: a kind added to the enum must
/// fail to compile here, so that whoever adds it decides whether it carries a bundle. A
/// catch-all would silently answer "no" and surface as a rejected block instead.
fn outputs_only_bundle_tag_v0(action_type: TokenTransitionActionType) -> Result<u8, ProtocolError> {
    match action_type {
        TokenTransitionActionType::Shield => Ok(TOKEN_SHIELD_BUNDLE_TAG),
        TokenTransitionActionType::MintToPool => Ok(TOKEN_MINT_TO_POOL_BUNDLE_TAG),
        TokenTransitionActionType::ClaimToPool => Ok(TOKEN_CLAIM_TO_POOL_BUNDLE_TAG),
        TokenTransitionActionType::DirectPurchaseToPool => {
            Ok(TOKEN_DIRECT_PURCHASE_TO_POOL_BUNDLE_TAG)
        }
        other @ (TokenTransitionActionType::Burn
        | TokenTransitionActionType::Mint
        | TokenTransitionActionType::Transfer
        | TokenTransitionActionType::Freeze
        | TokenTransitionActionType::Unfreeze
        | TokenTransitionActionType::DestroyFrozenFunds
        | TokenTransitionActionType::Claim
        | TokenTransitionActionType::EmergencyAction
        | TokenTransitionActionType::ConfigUpdate
        | TokenTransitionActionType::DirectPurchase
        | TokenTransitionActionType::SetPriceForDirectPurchase
        | TokenTransitionActionType::BurnFromPool
        | TokenTransitionActionType::Unshield
        | TokenTransitionActionType::ShieldedTransfer) => {
            Err(ProtocolError::InvalidStateTransitionType(format!(
                "{other} is not an outputs-only token pool transition and has no bundle tag"
            )))
        }
    }
}

/// Version 0 layout: `bundle tag (1) || token_id (32) || owner_id (32)`. Frozen: never
/// mutate; a layout change requires a new `_v1` + version bump.
pub fn token_pool_output_only_extra_sighash_data_v0(
    bundle_tag: u8,
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32 + 32);
    data.push(bundle_tag);
    data.extend_from_slice(token_id);
    data.extend_from_slice(owner_id);
    data
}

/// Extra sighash data of a credit pool `Shield` bundle: its kind tag and a digest of the platform
/// addresses that fund it. See [`credit_pool_output_only_extra_sighash_data_v0`] for why the
/// credit pool's outputs-only bundles bind anything at all.
///
/// A `Shield` has no identity, so its owner is its funding: the SHA-256 of its input addresses,
/// each in its 21-byte encoding ([`PlatformAddress::to_bytes`]), in the order the `inputs` map
/// holds them. That order is part of the wire format. It is the map's key order, which is also
/// the order the transition serializes its inputs in, so a client that assembles the same set in
/// any other order still binds the same bytes.
///
/// The nonces and the contributed amounts are deliberately left out. The owner answers "who
/// funds this", not "which transition carries it", exactly as `ShieldFromIdentity` binds its
/// identity and not its nonce: the preimage is there to stop somebody else from re-wrapping the
/// bundle. Binding the nonce would not stop the funder's own duplicates either, because a sender
/// can rebuild the same notes under any preimage (see
/// [`credit_pool_output_only_extra_sighash_data_v0`]); closing that would need the bundles' dummy
/// nullifiers recorded and checked, which consensus does not do. Adding the nonce later would
/// change the layout under every bundle already built for this one.
///
/// Dispatches on `dpp.methods.credit_pool_bundle_binding`, which the client builder and the
/// consensus verifier both read: `None` binds nothing (the empty preimage every shipped verifier
/// expects), `Some(0)` binds `tag (1) || funding digest (32)`.
pub fn shield_extra_sighash_data(
    inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.credit_pool_bundle_binding {
        None => Ok(Vec::new()),
        Some(0) => Ok(credit_pool_output_only_extra_sighash_data_v0(
            SHIELD_BUNDLE_TAG,
            &shield_funding_digest_v0(inputs),
        )),
        Some(version) => Err(ProtocolError::UnknownVersionMismatch {
            method: "shield_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// The version 0 owner of a `Shield` bundle: SHA-256 over the 21-byte encoding of each input
/// address, in the map's key order. Frozen: never mutate; see [`shield_extra_sighash_data`].
fn shield_funding_digest_v0(
    inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for address in inputs.keys() {
        hasher.update(address.to_bytes());
    }
    hasher.finalize().into()
}

/// Extra sighash data of a `ShieldFromIdentity` bundle: its kind tag and the identity whose
/// balance funds it. See [`credit_pool_output_only_extra_sighash_data_v0`].
///
/// The identity signature already covers the bundle, but only this transition's: nothing in the
/// bundle itself says which identity it was proved for, so without this preimage any other
/// identity could sign a transition of its own around the same proved bytes. The funding identity
/// itself can still land the same bundle again under a new nonce, which is what a client retrying
/// with a fresh nonce does; stopping that would need the bundles' dummy nullifiers recorded and
/// checked, which consensus does not do.
///
/// Dispatches on `dpp.methods.credit_pool_bundle_binding` like [`shield_extra_sighash_data`].
pub fn shield_from_identity_extra_sighash_data(
    identity_id: &[u8; 32],
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.credit_pool_bundle_binding {
        None => Ok(Vec::new()),
        Some(0) => Ok(credit_pool_output_only_extra_sighash_data_v0(
            SHIELD_FROM_IDENTITY_BUNDLE_TAG,
            identity_id,
        )),
        Some(version) => Err(ProtocolError::UnknownVersionMismatch {
            method: "shield_from_identity_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Extra sighash data of a `ShieldFromAssetLock` bundle: its kind tag and the identifier of the
/// asset lock that funds it ([`AssetLockProof::create_identifier`], the double SHA-256 of the
/// locked outpoint). See [`credit_pool_output_only_extra_sighash_data_v0`].
///
/// Binding the full outpoint, rather than the transaction id, makes this binding single-use. A
/// successful shield consumes the whole lock, so a given bundle can land at most once, whoever
/// wraps it and however often it is resubmitted: a copier would have to fund it from this very
/// lock, and so would the sender's own retry. Binding only the transaction id would let anybody
/// holding another credit output of the same asset-lock transaction lift the bundle. A sender who
/// deliberately rebuilds the same notes under another lock is still not stopped; see
/// [`credit_pool_output_only_extra_sighash_data_v0`].
///
/// The identifier is the same one an identity created from that lock would get; the kind tag keeps
/// this preimage apart from a `ShieldFromIdentity` of that identity.
///
/// Dispatches on `dpp.methods.credit_pool_bundle_binding` like [`shield_extra_sighash_data`].
pub fn shield_from_asset_lock_extra_sighash_data(
    asset_lock_proof: &AssetLockProof,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.credit_pool_bundle_binding {
        None => Ok(Vec::new()),
        Some(0) => Ok(credit_pool_output_only_extra_sighash_data_v0(
            SHIELD_FROM_ASSET_LOCK_BUNDLE_TAG,
            &asset_lock_proof.create_identifier()?.to_buffer(),
        )),
        Some(version) => Err(ProtocolError::UnknownVersionMismatch {
            method: "shield_from_asset_lock_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout of the credit pool's outputs-only bundles — `Shield`, `ShieldFromIdentity` and
/// `ShieldFromAssetLock`: `bundle tag (1) || owner (32)`. Frozen: never mutate; a layout change
/// requires a new `credit_pool_bundle_binding` version.
///
/// These bundles have no spends, so they carry no anchor to pin them to anything: the proof and
/// the binding signature verify wherever they are submitted. Unbound, the authorized bundle bytes
/// are a free-standing, self-verifying object: anybody can lift them out of the mempool into a
/// transition of their own, funded by their own credits, and any bundle ever published can be
/// replayed. The copier pays the full amount to the original recipient and gains nothing, but the
/// copy lands a second note with the same commitment and the same nullifier in the credit pool,
/// whose notes share one nullifier set, so only one of the two can ever be spent.
///
/// The owner is what a third party cannot authorize: the addresses, identity or asset lock whose
/// signatures fund the original. The tag stops a bundle proved for one kind from being submitted as another,
/// which the owner alone would not: the three kinds are otherwise indistinguishable to the proof,
/// and an identity created from an asset lock has that lock's identifier as its id.
///
/// This does not reach Faerie Gold. The `rho` of an outputs-only note is the nullifier of its
/// bundle's dummy spend, so a sender who builds and signs a fresh bundle reusing the same dummy
/// spend note and `rseed` gets the same commitment and the same nullifier under any preimage, and
/// a recipient counting deposits by commitment credits two where only one can be spent. Nor does
/// it stop the funder landing their own bundle twice through a new transition, as a client that
/// retries with a fresh nonce does — except for `ShieldFromAssetLock`, whose lock can fund one
/// successful shield only (see [`shield_from_asset_lock_extra_sighash_data`]). Closing either
/// would need the bundles' dummy nullifiers recorded and checked, which consensus does not do.
pub fn credit_pool_output_only_extra_sighash_data_v0(bundle_tag: u8, owner: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32);
    data.push(bundle_tag);
    data.extend_from_slice(owner);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::core_script::CoreScript;
    use crate::state_transition::StateTransitionType;
    use crate::withdrawal::Pooling;
    // These tests pin the v0 preimage directly (they assert exact bytes), so resolve the bare helper
    // names to the `_v0` impls rather than the version-dispatching public wrappers.
    use crate::shielded::shielded_withdrawal_extra_sighash_data_v0 as shielded_withdrawal_extra_sighash_data;
    use crate::shielded::unshield_extra_sighash_data_v0 as unshield_extra_sighash_data;

    #[test]
    fn withdrawal_sighash_data_binds_core_fee_per_byte() {
        let script = CoreScript::new_p2pkh([1u8; 20]);
        let a = shielded_withdrawal_extra_sighash_data(script.as_bytes(), 1000, 1, Pooling::Never);
        let b = shielded_withdrawal_extra_sighash_data(script.as_bytes(), 1000, 2, Pooling::Never);
        assert_ne!(
            a, b,
            "changing core_fee_per_byte must change the sighash preimage"
        );
    }

    #[test]
    fn withdrawal_sighash_data_binds_pooling() {
        // `pooling` is pinned to `Never` by `validate_structure`, so this binding is currently
        // dead defense-in-depth; assert it is nonetheless mixed into the preimage so a future
        // unpinning would still be authorized by the Orchard binding signature.
        let script = CoreScript::new_p2pkh([1u8; 20]);
        let a = shielded_withdrawal_extra_sighash_data(script.as_bytes(), 1000, 1, Pooling::Never);
        let b = shielded_withdrawal_extra_sighash_data(
            script.as_bytes(),
            1000,
            1,
            Pooling::IfAvailable,
        );
        assert_ne!(a, b, "changing pooling must change the sighash preimage");
    }

    #[test]
    fn withdrawal_sighash_data_layout() {
        // output_script(2) || unshielding_amount(8) || core_fee_per_byte(4) || pooling(1)
        let d = shielded_withdrawal_extra_sighash_data(&[0xAA, 0xBB], 1, 2, Pooling::Never);
        assert_eq!(d.len(), 2 + 8 + 4 + 1);
        assert_eq!(&d[0..2], &[0xAA, 0xBB]);
        assert_eq!(&d[2..10], &1u64.to_le_bytes());
        assert_eq!(&d[10..14], &2u32.to_le_bytes());
        assert_eq!(d[14], Pooling::Never as u8);
    }

    #[test]
    fn token_unshield_sighash_data_layout() {
        use crate::shielded::token_unshield_extra_sighash_data_v0;
        // token_id(32) || owner_id(32) || recipient_id(32) || amount(8)
        let d =
            token_unshield_extra_sighash_data_v0(&[0xAAu8; 32], &[0xBBu8; 32], &[0xCCu8; 32], 7);
        assert_eq!(d.len(), 32 + 32 + 32 + 8);
        assert_eq!(&d[0..32], &[0xAAu8; 32]);
        assert_eq!(&d[32..64], &[0xBBu8; 32]);
        assert_eq!(&d[64..96], &[0xCCu8; 32]);
        assert_eq!(&d[96..104], &7u64.to_le_bytes());
        // Every bound field changes the preimage.
        assert_ne!(
            d,
            token_unshield_extra_sighash_data_v0(&[0xADu8; 32], &[0xBBu8; 32], &[0xCCu8; 32], 7)
        );
        assert_ne!(
            d,
            token_unshield_extra_sighash_data_v0(&[0xAAu8; 32], &[0xB0u8; 32], &[0xCCu8; 32], 7)
        );
        assert_ne!(
            d,
            token_unshield_extra_sighash_data_v0(&[0xAAu8; 32], &[0xBBu8; 32], &[0xC0u8; 32], 7)
        );
        assert_ne!(
            d,
            token_unshield_extra_sighash_data_v0(&[0xAAu8; 32], &[0xBBu8; 32], &[0xCCu8; 32], 8)
        );
    }

    #[test]
    fn token_shielded_transfer_sighash_data_layout() {
        use crate::shielded::token_shielded_transfer_extra_sighash_data_v0;
        // token_id(32) || owner_id(32)
        let d = token_shielded_transfer_extra_sighash_data_v0(&[0x11u8; 32], &[0x22u8; 32]);
        assert_eq!(d.len(), 64);
        assert_eq!(&d[0..32], &[0x11u8; 32]);
        assert_eq!(&d[32..64], &[0x22u8; 32]);
        assert_ne!(
            d,
            token_shielded_transfer_extra_sighash_data_v0(&[0x12u8; 32], &[0x22u8; 32])
        );
        assert_ne!(
            d,
            token_shielded_transfer_extra_sighash_data_v0(&[0x11u8; 32], &[0x23u8; 32])
        );
    }

    #[test]
    fn unshield_sighash_data_layout() {
        // output_address || unshielding_amount(8)
        let d = unshield_extra_sighash_data(&[0xAA, 0xBB, 0xCC], 5);
        assert_eq!(d.len(), 3 + 8);
        assert_eq!(&d[0..3], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(&d[3..11], &5u64.to_le_bytes());
    }

    mod identity_create_sighash {
        use super::*;
        // Pin the v0 preimage directly (see the note in the parent test module).
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::shielded::identity_create_from_shielded_extra_sighash_data_v0 as identity_create_from_shielded_extra_sighash_data;
        use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
        use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
        use platform_value::BinaryData;

        fn mk_key(id: u32, data_byte: u8) -> IdentityPublicKeyInCreation {
            IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
                id,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(vec![data_byte; 33]),
                signature: BinaryData::new(vec![]),
            })
        }

        #[test]
        fn layout_is_length_prefixed() {
            // identity_id(32) || denomination(8)
            //   || send_to_address_on_creation_failure (tag(1) || hash(20))
            //   || num_keys(2)
            //   || [key_id(4)|purpose|sec|type|len(2)|data|read_only(1)|contract_bounds_tag(1)]
            let id = [0x11u8; 32];
            let keys = vec![mk_key(7, 0xAB)];
            let fallback = PlatformAddress::P2pkh([0x5Cu8; 20]);
            let d = identity_create_from_shielded_extra_sighash_data(
                &id,
                10_000_000_000,
                &fallback,
                &keys,
            );
            assert_eq!(&d[0..32], &id);
            assert_eq!(&d[32..40], &10_000_000_000u64.to_le_bytes());
            // Fallback address: tag(0=P2pkh) at offset 40, 20-byte hash at 41..61.
            assert_eq!(d[40], 0u8, "fallback address P2pkh tag");
            assert_eq!(&d[41..61], &[0x5Cu8; 20], "fallback address hash");
            assert_eq!(&d[61..63], &1u16.to_le_bytes());
            assert_eq!(&d[63..67], &7u32.to_le_bytes());
            assert_eq!(d[67], Purpose::AUTHENTICATION as u8);
            assert_eq!(d[68], SecurityLevel::MASTER as u8);
            assert_eq!(d[69], KeyType::ECDSA_SECP256K1 as u8);
            assert_eq!(&d[70..72], &33u16.to_le_bytes());
            assert_eq!(&d[72..105], &[0xAB; 33]);
            assert_eq!(d[105], 0u8, "read_only=false");
            assert_eq!(d[106], 0u8, "contract_bounds=None tag");
            assert_eq!(d.len(), 32 + 8 + 21 + 2 + (4 + 1 + 1 + 1 + 2 + 33 + 1 + 1));
        }

        #[test]
        fn binds_identity_id_denomination_and_keys() {
            let id_a = [0x11u8; 32];
            let id_b = [0x22u8; 32];
            let keys = vec![mk_key(0, 0xAA)];
            let fallback = PlatformAddress::P2pkh([0x01u8; 20]);
            let base = identity_create_from_shielded_extra_sighash_data(
                &id_a,
                10_000_000_000,
                &fallback,
                &keys,
            );

            // Changing the identity id changes the preimage (anti-redirection to a different id).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_b,
                    10_000_000_000,
                    &fallback,
                    &keys
                ),
                "identity id must be bound"
            );
            // Changing the denomination changes the preimage.
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    25_000_000_000,
                    &fallback,
                    &keys
                ),
                "denomination must be bound"
            );
            // Changing the fallback failure address changes the preimage (anti-redirection of the
            // failure credit: a relayer cannot point the penalty-charged spend at a different
            // address than the one each key's proof-of-possession signed).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &PlatformAddress::P2pkh([0x02u8; 20]),
                    &keys
                ),
                "fallback failure address hash must be bound"
            );
            // Changing only the fallback address TYPE (P2pkh -> P2sh, same hash) changes the
            // preimage too (the type tag is bound, not just the hash).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &PlatformAddress::P2sh([0x01u8; 20]),
                    &keys
                ),
                "fallback failure address type tag must be bound"
            );
            // Swapping in a different key changes the preimage (anti-key-swap).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &fallback,
                    &[mk_key(0, 0xBB)]
                ),
                "key data must be bound"
            );
            // Adding a key changes the preimage (the full set is bound, not just the count).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &fallback,
                    &[mk_key(0, 0xAA), mk_key(1, 0xCC)]
                ),
                "the full key set must be bound"
            );
        }

        #[test]
        fn binds_read_only_and_contract_bounds() {
            use crate::identity::identity_public_key::contract_bounds::ContractBounds;
            use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
            let id = [0x11u8; 32];
            let fallback = PlatformAddress::P2pkh([0x01u8; 20]);
            let base = identity_create_from_shielded_extra_sighash_data(
                &id,
                10_000_000_000,
                &fallback,
                &[mk_key(0, 0xAA)],
            );

            // Flipping read_only changes the preimage (un-malleable for every key type).
            let mut ro_key = mk_key(0, 0xAA);
            ro_key.set_read_only(true);
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id,
                    10_000_000_000,
                    &fallback,
                    &[ro_key]
                ),
                "read_only must be bound"
            );

            // Attaching contract_bounds changes the preimage.
            let mut cb_key = mk_key(0, 0xAA);
            cb_key.set_contract_bounds(Some(ContractBounds::SingleContract {
                id: platform_value::Identifier::new([0x33; 32]),
            }));
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id,
                    10_000_000_000,
                    &fallback,
                    &[cb_key]
                ),
                "contract_bounds must be bound"
            );
        }

        #[test]
        fn should_encode_the_reserved_contract_group_tag_at_the_end_of_the_key() {
            use crate::identity::identity_public_key::contract_bounds::ContractBounds;
            use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
            // Consensus and the builder refuse a group-bound key before this preimage is built;
            // the arm exists so the encoder stays total. Pin what it writes.
            let mut key = mk_key(0, 0xAA);
            key.set_contract_bounds(Some(ContractBounds::ContractGroup {
                id: platform_value::Identifier::new([0x44; 32]),
            }));
            let data = identity_create_from_shielded_extra_sighash_data(
                &[0x11u8; 32],
                10_000_000_000,
                &PlatformAddress::P2pkh([0x01u8; 20]),
                &[key],
            );
            assert_eq!(data[data.len() - 33], 3);
            assert_eq!(&data[data.len() - 32..], &[0x44u8; 32]);
        }
    }

    #[test]
    fn document_token_payment_layout_is_token_owner_contract_document_amount() {
        let data = document_token_payment_extra_sighash_data_v0(
            &[1u8; 32], &[2u8; 32], &[3u8; 32], &[4u8; 32], 10,
        );
        assert_eq!(data.len(), 136);
        assert_eq!(&data[..32], &[1u8; 32]);
        assert_eq!(&data[32..64], &[2u8; 32]);
        assert_eq!(&data[64..96], &[3u8; 32]);
        assert_eq!(&data[96..128], &[4u8; 32]);
        assert_eq!(&data[128..], &10u64.to_le_bytes());
    }

    #[test]
    fn token_pool_paid_layouts_start_with_the_state_transition_type() {
        let transfer = token_shielded_transfer_with_shielded_fee_extra_sighash_data_v0(&[1u8; 32]);
        assert_eq!(transfer.len(), 33);
        assert_eq!(transfer[0], 26);
        assert_eq!(&transfer[1..], &[1u8; 32]);

        let unshield =
            token_unshield_with_shielded_fee_extra_sighash_data_v0(&[1u8; 32], &[2u8; 32], 300);
        assert_eq!(unshield.len(), 73);
        assert_eq!(unshield[0], 27);
        assert_eq!(&unshield[1..33], &[1u8; 32]);
        assert_eq!(&unshield[33..65], &[2u8; 32]);
        assert_eq!(&unshield[65..], &300u64.to_le_bytes());

        let purchase = token_purchase_from_shielded_pool_extra_sighash_data_v0(&[1u8; 32], 5, 900);
        assert_eq!(purchase.len(), 49);
        assert_eq!(purchase[0], 28);
        assert_eq!(&purchase[33..41], &5u64.to_le_bytes());
        assert_eq!(&purchase[41..], &900u64.to_le_bytes());

        let fee = token_pool_fee_bundle_extra_sighash_data_v0(27, &[1u8; 32], &[3u8; 32]);
        assert_eq!(fee.len(), 65);
        assert_eq!(fee[0], 27);
        assert_eq!(&fee[1..33], &[1u8; 32]);
        assert_eq!(&fee[33..], &[3u8; 32]);
    }

    #[test]
    fn token_burn_from_pool_layout_is_token_burner_amount() {
        let data = token_burn_from_pool_extra_sighash_data_v0(&[1u8; 32], &[2u8; 32], 300);
        assert_eq!(data.len(), 72);
        assert_eq!(&data[..32], &[1u8; 32]);
        assert_eq!(&data[32..64], &[2u8; 32]);
        assert_eq!(&data[64..], &300u64.to_le_bytes());
    }

    #[test]
    fn outputs_only_token_pool_sighash_data_pins_its_v0_layout() {
        // Every byte here is consensus: a kind that changes its tag invalidates every bundle
        // already proved for it, so the four are pinned literally, not to the constants.
        let token_id = [7u8; 32];
        let owner_id = [5u8; 32];
        let version = PlatformVersion::latest();
        for (action_type, tag) in [
            (TokenTransitionActionType::Shield, 0x80u8),
            (TokenTransitionActionType::MintToPool, 0x81),
            (TokenTransitionActionType::ClaimToPool, 0x82),
            (TokenTransitionActionType::DirectPurchaseToPool, 0x83),
        ] {
            let mut expected = Vec::with_capacity(65);
            expected.push(tag);
            expected.extend_from_slice(&token_id);
            expected.extend_from_slice(&owner_id);
            assert_eq!(
                token_pool_output_only_extra_sighash_data(
                    action_type,
                    &token_id,
                    &owner_id,
                    version
                )
                .expect("outputs-only kind"),
                expected,
                "v0 layout is frozen for {action_type}: tag (1) || token_id (32) || owner_id (32)"
            );
        }
    }

    #[test]
    fn outputs_only_token_pool_sighash_data_separates_pools() {
        // The whole point of the data: an outputs-only bundle carries no anchor, so without it
        // the same proved bytes verify against every token pool.
        let version = PlatformVersion::latest();
        let a = token_pool_output_only_extra_sighash_data(
            TokenTransitionActionType::Shield,
            &[1u8; 32],
            &[5u8; 32],
            version,
        )
        .expect("shield tag");
        let b = token_pool_output_only_extra_sighash_data(
            TokenTransitionActionType::Shield,
            &[2u8; 32],
            &[5u8; 32],
            version,
        )
        .expect("shield tag");
        assert_ne!(
            a, b,
            "a bundle must not verify against another token's pool"
        );
    }

    #[test]
    fn outputs_only_token_pool_sighash_data_separates_transition_kinds() {
        // All four kinds bind the same token id and the same owner, so only the tag keeps a
        // shield bundle from being resubmitted by its owner as a mint, a claim or a purchase
        // into the very same pool.
        let version = PlatformVersion::latest();
        let token_id = [9u8; 32];
        let owner_id = [5u8; 32];
        let built: Vec<Vec<u8>> = [
            TokenTransitionActionType::Shield,
            TokenTransitionActionType::MintToPool,
            TokenTransitionActionType::ClaimToPool,
            TokenTransitionActionType::DirectPurchaseToPool,
        ]
        .into_iter()
        .map(|action_type| {
            token_pool_output_only_extra_sighash_data(action_type, &token_id, &owner_id, version)
                .expect("outputs-only kind")
        })
        .collect();

        for (i, a) in built.iter().enumerate() {
            for b in built.iter().skip(i + 1) {
                assert_ne!(a, b, "each transition kind must get its own preimage");
            }
        }
    }

    #[test]
    fn outputs_only_token_pool_sighash_data_refuses_every_other_kind() {
        // The kinds come from an enum shared with clients that carries fourteen other
        // variants. Handing one of those in is a caller bug, and it must be loud: silently
        // falling back to some default byte would hand two kinds the same preimage.
        let version = PlatformVersion::latest();
        // All fourteen, not a sample: match exhaustiveness protects against a kind being
        // *added* and forgotten, but nothing stops an existing arm being edited into the
        // accepting half, and a sample would not notice.
        for action_type in [
            TokenTransitionActionType::Burn,
            TokenTransitionActionType::Mint,
            TokenTransitionActionType::Transfer,
            TokenTransitionActionType::Freeze,
            TokenTransitionActionType::Unfreeze,
            TokenTransitionActionType::DestroyFrozenFunds,
            TokenTransitionActionType::Claim,
            TokenTransitionActionType::EmergencyAction,
            TokenTransitionActionType::ConfigUpdate,
            TokenTransitionActionType::DirectPurchase,
            TokenTransitionActionType::SetPriceForDirectPurchase,
            TokenTransitionActionType::BurnFromPool,
            TokenTransitionActionType::Unshield,
            TokenTransitionActionType::ShieldedTransfer,
        ] {
            let error = token_pool_output_only_extra_sighash_data(
                action_type,
                &[1u8; 32],
                &[5u8; 32],
                version,
            )
            .expect_err("only outputs-only pool bundles have a tag");
            assert!(
                matches!(error, ProtocolError::InvalidStateTransitionType(_)),
                "{action_type} must be refused, got {error:?}"
            );
        }
    }

    #[test]
    fn outputs_only_token_pool_tags_cannot_collide_with_state_transition_types() {
        // The tags share a preimage slot with the `StateTransitionType` byte the identity-less
        // token bundles commit to, and one of those layouts — the credit pool fee bundle's
        // `type || token_id || token actions digest` — has the same 1 + 32 + 32 length, so a
        // colliding tag would make the two layouts byte-for-byte the same shape. Asking the enum
        // itself — rather than comparing against the three types that exist today — is what
        // makes this break on the day someone assigns a transition type inside the tag range.
        for tag in [
            TOKEN_SHIELD_BUNDLE_TAG,
            TOKEN_MINT_TO_POOL_BUNDLE_TAG,
            TOKEN_CLAIM_TO_POOL_BUNDLE_TAG,
            TOKEN_DIRECT_PURCHASE_TO_POOL_BUNDLE_TAG,
        ] {
            assert!(
                StateTransitionType::try_from(tag).is_err(),
                "state transition type {tag:#04x} now collides with an outputs-only bundle tag"
            );
        }
    }

    mod credit_pool_outputs_only {
        use super::*;
        use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use crate::util::hash::hash_double;
        use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
        use dashcore::transaction::special_transaction::TransactionPayload;
        use dashcore::{InstantLock, OutPoint, ScriptBuf, Transaction, TxOut};

        fn protocol_version(version: u32) -> &'static PlatformVersion {
            PlatformVersion::get(version).expect("known protocol version")
        }

        fn chain_asset_lock_proof(outpoint: [u8; 36]) -> AssetLockProof {
            AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 100,
                out_point: OutPoint::from(outpoint),
            })
        }

        fn inputs(
            entries: &[(PlatformAddress, AddressNonce, Credits)],
        ) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
            entries
                .iter()
                .map(|(address, nonce, amount)| (*address, (*nonce, *amount)))
                .collect()
        }

        #[test]
        fn should_pin_the_v0_layout_of_all_three_kinds() {
            // Every byte here is consensus: a kind whose tag or owner changes invalidates every
            // bundle already proved for it, so the tags are written literally, not through the
            // constants, and the owners are derived independently of the code under test.
            let version = PlatformVersion::latest();

            // The map orders P2pkh before P2sh whatever their hash bytes, and each address is its
            // 21-byte encoding: variant index, then the 20-byte hash.
            let shield_inputs = inputs(&[
                (PlatformAddress::P2sh([0x00; 20]), 1, 10),
                (PlatformAddress::P2pkh([0x11; 20]), 2, 20),
            ]);
            let mut funding = Vec::new();
            funding.push(0x00);
            funding.extend_from_slice(&[0x11; 20]);
            funding.push(0x01);
            funding.extend_from_slice(&[0x00; 20]);
            let mut expected = vec![0x84];
            expected.extend_from_slice(&Sha256::digest(&funding));
            assert_eq!(
                shield_extra_sighash_data(&shield_inputs, version).expect("shield"),
                expected,
                "Shield: 0x84 || SHA-256 of the input addresses in map order"
            );

            let identity_id = [0x22u8; 32];
            let mut expected = vec![0x85];
            expected.extend_from_slice(&identity_id);
            assert_eq!(
                shield_from_identity_extra_sighash_data(&identity_id, version)
                    .expect("shield from identity"),
                expected,
                "ShieldFromIdentity: 0x85 || identity id"
            );

            let outpoint = [0x33u8; 36];
            let mut expected = vec![0x86];
            expected.extend_from_slice(&hash_double(outpoint));
            assert_eq!(
                shield_from_asset_lock_extra_sighash_data(
                    &chain_asset_lock_proof(outpoint),
                    version
                )
                .expect("shield from asset lock"),
                expected,
                "ShieldFromAssetLock: 0x86 || double SHA-256 of the locked outpoint"
            );
        }

        #[test]
        fn should_bind_the_same_shield_funding_whatever_order_the_inputs_were_assembled_in() {
            // The funding digest hashes the addresses in the map's key order, and that order is
            // wire format: a client that collected the same addresses in another order must
            // still produce the bytes the verifier rebuilds from the transition.
            let version = PlatformVersion::latest();
            let a = (PlatformAddress::P2pkh([0x01; 20]), 1, 100);
            let b = (PlatformAddress::P2sh([0x02; 20]), 2, 200);
            let c = (PlatformAddress::P2pkh([0x03; 20]), 3, 300);

            let forward = shield_extra_sighash_data(&inputs(&[a, b, c]), version).expect("shield");
            let backward = shield_extra_sighash_data(&inputs(&[c, b, a]), version).expect("shield");
            let shuffled = shield_extra_sighash_data(&inputs(&[b, c, a]), version).expect("shield");

            assert_eq!(forward, backward);
            assert_eq!(forward, shuffled);
        }

        #[test]
        fn should_bind_who_funds_a_shield_but_not_its_nonces_or_amounts() {
            // The owner is the funding, not the transition: the same addresses with other nonces
            // or other contributions bind the same bytes, so a later "tightening" that adds the
            // nonce shows up here as a layout change rather than slipping in.
            let version = PlatformVersion::latest();
            let first = PlatformAddress::P2pkh([0x01; 20]);
            let second = PlatformAddress::P2pkh([0x02; 20]);
            let base =
                shield_extra_sighash_data(&inputs(&[(first, 1, 100)]), version).expect("shield");

            assert_eq!(
                base,
                shield_extra_sighash_data(&inputs(&[(first, 9, 100)]), version).expect("shield"),
                "the nonce must not be bound"
            );
            assert_eq!(
                base,
                shield_extra_sighash_data(&inputs(&[(first, 1, 999)]), version).expect("shield"),
                "the contributed amount must not be bound"
            );
            assert_ne!(
                base,
                shield_extra_sighash_data(&inputs(&[(second, 1, 100)]), version).expect("shield"),
                "another funding address is another owner"
            );
            assert_ne!(
                base,
                shield_extra_sighash_data(&inputs(&[(first, 1, 100), (second, 1, 100)]), version)
                    .expect("shield"),
                "an added funding address is another owner"
            );
            assert_ne!(
                base,
                shield_extra_sighash_data(
                    &inputs(&[(PlatformAddress::P2sh([0x01; 20]), 1, 100)]),
                    version
                )
                .expect("shield"),
                "the address type is part of the owner, not only its hash"
            );
        }

        #[test]
        fn should_keep_kinds_apart_when_their_owners_coincide() {
            // An identity created from an asset lock has that lock's identifier as its id, so a
            // `ShieldFromAssetLock` and a `ShieldFromIdentity` can bind the very same owner bytes.
            // Only the tag keeps a bundle proved for one from being accepted as the other.
            let version = PlatformVersion::latest();
            let proof = chain_asset_lock_proof([0x44; 36]);
            let identity_id = proof.create_identifier().expect("identifier").to_buffer();

            let from_lock =
                shield_from_asset_lock_extra_sighash_data(&proof, version).expect("asset lock");
            let from_identity =
                shield_from_identity_extra_sighash_data(&identity_id, version).expect("identity");

            assert_eq!(&from_lock[1..], &from_identity[1..], "the owners coincide");
            assert_ne!(from_lock, from_identity, "the kinds must not");
        }

        #[test]
        fn should_bind_the_whole_outpoint_of_the_asset_lock_not_only_its_transaction() {
            // A lock transaction can carry several credit outputs, each its own asset lock. Bound
            // to the transaction id alone, a bundle could be lifted by whoever holds another
            // output of the same transaction. The owner is the outpoint, and it is the same owner
            // whichever kind of proof presents it.
            let version = PlatformVersion::latest();
            let credit_output = |value| TxOut {
                value,
                script_pubkey: ScriptBuf::new(),
            };
            let transaction = Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                    AssetLockPayload {
                        version: 0,
                        credit_outputs: vec![credit_output(100_000), credit_output(200_000)],
                    },
                )),
            };
            let txid = transaction.txid();
            let instant = |output_index| {
                AssetLockProof::Instant(InstantAssetLockProof::new(
                    InstantLock::default(),
                    transaction.clone(),
                    output_index,
                ))
            };
            let bound = |proof: &AssetLockProof| {
                shield_from_asset_lock_extra_sighash_data(proof, version).expect("asset lock")
            };

            assert_ne!(
                bound(&instant(0)),
                bound(&instant(1)),
                "another output of the same lock transaction is another owner"
            );
            assert_eq!(
                bound(&instant(1)),
                bound(&AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: 100,
                    out_point: OutPoint::new(txid, 1),
                })),
                "the same outpoint is the same owner whichever proof presents it"
            );
        }

        #[test]
        fn should_bind_nothing_at_protocol_versions_before_the_binding() {
            // Protocol versions 12 and 13 run the credit pool with verifiers that rebuild an empty
            // preimage. A client building for one of them reads the same field, so it must get the
            // empty preimage back, or every shield it makes there is rejected.
            let lock = chain_asset_lock_proof([0x55; 36]);
            let funding = inputs(&[(PlatformAddress::P2pkh([0x01; 20]), 1, 100)]);
            for version in [12, 13] {
                let version = protocol_version(version);
                assert_eq!(version.dpp.methods.credit_pool_bundle_binding, None);
                assert!(shield_extra_sighash_data(&funding, version)
                    .expect("shield")
                    .is_empty());
                assert!(
                    shield_from_identity_extra_sighash_data(&[0x66; 32], version)
                        .expect("shield from identity")
                        .is_empty()
                );
                assert!(shield_from_asset_lock_extra_sighash_data(&lock, version)
                    .expect("shield from asset lock")
                    .is_empty());
            }
        }

        #[test]
        fn should_refuse_an_unknown_binding_version() {
            // Neither side may guess: a builder that fell back to one layout while the verifier
            // chose another would reject every honest shield.
            let mut version = PlatformVersion::latest().clone();
            version.dpp.methods.credit_pool_bundle_binding = Some(1);
            let lock = chain_asset_lock_proof([0x77; 36]);

            for result in [
                shield_extra_sighash_data(&BTreeMap::new(), &version),
                shield_from_identity_extra_sighash_data(&[0x88; 32], &version),
                shield_from_asset_lock_extra_sighash_data(&lock, &version),
            ] {
                assert_matches::assert_matches!(
                    result,
                    Err(ProtocolError::UnknownVersionMismatch { received: 1, .. })
                );
            }
        }

        #[test]
        fn credit_pool_outputs_only_tags_cannot_collide_with_state_transition_types() {
            // The tags share a preimage slot with the `StateTransitionType` byte the identity-less
            // token bundles commit to, at the same 1 + 32 length. Asking the enum itself is what
            // makes this break on the day a transition type is assigned inside the tag range.
            for tag in [
                SHIELD_BUNDLE_TAG,
                SHIELD_FROM_IDENTITY_BUNDLE_TAG,
                SHIELD_FROM_ASSET_LOCK_BUNDLE_TAG,
            ] {
                assert!(
                    StateTransitionType::try_from(tag).is_err(),
                    "state transition type {tag:#04x} now collides with a credit pool bundle tag"
                );
            }
        }

        #[test]
        fn outputs_only_bundle_tags_are_pairwise_distinct() {
            // All seven outputs-only layouts are `tag (1) || 32 bytes`, so two kinds sharing a
            // tag would share a preimage whenever their 32 bytes coincide.
            let tags = [
                TOKEN_SHIELD_BUNDLE_TAG,
                TOKEN_MINT_TO_POOL_BUNDLE_TAG,
                TOKEN_CLAIM_TO_POOL_BUNDLE_TAG,
                TOKEN_DIRECT_PURCHASE_TO_POOL_BUNDLE_TAG,
                SHIELD_BUNDLE_TAG,
                SHIELD_FROM_IDENTITY_BUNDLE_TAG,
                SHIELD_FROM_ASSET_LOCK_BUNDLE_TAG,
            ];
            for (i, a) in tags.iter().enumerate() {
                for b in tags.iter().skip(i + 1) {
                    assert_ne!(a, b, "outputs-only bundle tag {a:#04x} is used twice");
                }
            }
        }
    }
}
