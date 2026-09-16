//! Platform sighash preimage construction for shielded transitions.
//!
//! Shielded transitions carry NO platform identity signature — authorization is the Orchard proof +
//! per-action spend-auth signatures + the RedPallas binding signature over the platform sighash.
//! These helpers build the transparent `extra_data` each transition binds into that sighash so the
//! signing (client/builder) and verifying (consensus) sides commit to identical bytes. The byte
//! layouts are consensus-critical and versioned via `dpp.methods.shielded_extra_sighash_data`.

use crate::address_funds::PlatformAddress;
use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::shielded::{serialized_actions_digest, SerializedAction};
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use sha2::{Digest, Sha256};

/// Domain separator for Platform sighash computation.
const SIGHASH_DOMAIN: &[u8] = b"DashPlatformSighash";

/// The state transition type byte the token bundle of a `TokenShieldedTransferWithShieldedFee`
/// commits to (`StateTransitionType::TokenShieldedTransferWithShieldedFee`).
pub const TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE: u8 = 23;
/// The state transition type byte the token bundle of a `TokenUnshieldWithShieldedFee` commits to.
pub const TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE: u8 = 24;
/// The state transition type byte the token bundle of a `TokenPurchaseFromShieldedPool` commits to.
pub const TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE: u8 = 25;

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
/// The same computation must be used on both the signing (client) and verification
/// (platform) sides. For transitions without transparent fields (shield and
/// shielded_transfer), `extra_data` is empty.
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
///   id(32) name_len(u16 LE) name)`.
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

/// Extra sighash data of a batch `TokenBurnFromPool`: the token id, the burning identity and the
/// amount destroyed, so a bundle proven for one burn cannot be replayed for another token,
/// signer or amount (72 bytes; no other layout has that length).
pub fn token_burn_from_pool_extra_sighash_data(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(token_burn_from_pool_extra_sighash_data_v0(
            token_id, owner_id, amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "token_burn_from_pool_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Version 0 layout: `token_id (32) || owner_id (32) || amount (8, little endian)`.
pub fn token_burn_from_pool_extra_sighash_data_v0(
    token_id: &[u8; 32],
    owner_id: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 32 + 8);
    data.extend_from_slice(token_id);
    data.extend_from_slice(owner_id);
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

/// Version 0 layout: `state transition type (1, = 23) || token_id (32)`. Frozen.
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

/// Version 0 layout: `state transition type (1, = 24) || token_id (32) || recipient_id (32)
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

/// Version 0 layout: `state transition type (1, = 25) || token_id (32) || token_count (8, LE)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::core_script::CoreScript;
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
        assert_eq!(transfer[0], 23);
        assert_eq!(&transfer[1..], &[1u8; 32]);

        let unshield =
            token_unshield_with_shielded_fee_extra_sighash_data_v0(&[1u8; 32], &[2u8; 32], 300);
        assert_eq!(unshield.len(), 73);
        assert_eq!(unshield[0], 24);
        assert_eq!(&unshield[1..33], &[1u8; 32]);
        assert_eq!(&unshield[33..65], &[2u8; 32]);
        assert_eq!(&unshield[65..], &300u64.to_le_bytes());

        let purchase = token_purchase_from_shielded_pool_extra_sighash_data_v0(&[1u8; 32], 5, 900);
        assert_eq!(purchase.len(), 49);
        assert_eq!(purchase[0], 25);
        assert_eq!(&purchase[33..41], &5u64.to_le_bytes());
        assert_eq!(&purchase[41..], &900u64.to_le_bytes());

        let fee = token_pool_fee_bundle_extra_sighash_data_v0(24, &[1u8; 32], &[3u8; 32]);
        assert_eq!(fee.len(), 65);
        assert_eq!(fee[0], 24);
        assert_eq!(&fee[1..33], &[1u8; 32]);
        assert_eq!(&fee[33..], &[3u8; 32]);
    }

    #[test]
    fn token_burn_from_pool_layout_is_token_owner_amount() {
        let data = token_burn_from_pool_extra_sighash_data_v0(&[1u8; 32], &[2u8; 32], 300);
        assert_eq!(data.len(), 72);
        assert_eq!(&data[..32], &[1u8; 32]);
        assert_eq!(&data[32..64], &[2u8; 32]);
        assert_eq!(&data[64..], &300u64.to_le_bytes());
    }
}
