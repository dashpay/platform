//! Masternode contested-resource vote broadcast.
//!
//! This is the *write* counterpart to the contested-resource query FFIs in
//! `crate::contested_resource::queries`. It assembles a
//! [`MasternodeVoteTransition`] from a contested-document vote poll
//! (`contract_id`, `document_type_name`, `index_name`, `index_values`) plus a
//! [`ResourceVoteChoice`] and broadcasts it via the existing rs-sdk
//! [`PutVote`] entry point. Nothing is stitched together here that the SDK
//! does not already expose as a single call.
//!
//! # Who can actually cast a vote
//!
//! A contested-resource (DPNS) vote is cast by a **masternode** using its
//! masternode *voting key* — an `ECDSA_HASH160` key whose 20-byte `data` is
//! the hash160 of the voting public key, tied to the masternode's
//! `pro_tx_hash` (see `get_voter_identity_key_v0` in rs-drive-abci). The
//! voter identity is derived deterministically as
//! `create_voter_identifier(pro_tx_hash, voting_address)`.
//!
//! This FFI therefore takes the raw 32-byte **voting private key** plus the
//! 32-byte `pro_tx_hash`. From the private key it derives:
//!   * a [`SingleKeySigner`] that signs the transition, and
//!   * `hash160(pubkey)` — the voting address, which identifies both the voter
//!     identity (`create_voter_identifier`) and the key on it.
//!
//! The voting [`IdentityPublicKey`] is then **fetched from the voter
//! identity**, not fabricated: its key id is only 0 for an identity Platform
//! created and never rotated, so assuming 0 fails with an opaque
//! "Public key 0 doesn't exist" whenever the identity is absent or the key
//! moved. Looking it up also lets a mismatched key be reported as such.
//!
//! The `SingleKeySigner::can_sign_with` check for `ECDSA_HASH160` recomputes
//! `hash160(pubkey)` from the same private key, so the fetched key and the
//! signer agree.
//!
//! A regular wallet is **not** a masternode and has no voting key, so a vote
//! broadcast from such a wallet reaches a deterministic *authorization*
//! rejection on the platform side (the voter identity / masternode is not
//! found), which is the expected end state when testing without real
//! masternode credentials. The construction + signing + broadcast path is
//! fully exercised regardless.

use crate::sdk::SDKWrapper;
use crate::types::{FFINetwork, Network, SDKHandle};
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
use dash_sdk::dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dash_sdk::dpp::identifier::MasternodeIdentifiers;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose};
use dash_sdk::dpp::platform_value::{Identifier, Value};
use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::dpp::voting::votes::Vote;
use dash_sdk::platform::transition::vote::PutVote;
use dash_sdk::platform::Fetch;
use simple_signer::SingleKeySigner;
use std::ffi::{c_char, CStr};
use zeroize::Zeroizing;

/// Resource vote choice discriminant, mirroring
/// `ResourceVoteChoice::try_from((i32, Option<Vec<u8>>))` in rs-dpp:
/// `0` = TowardsIdentity (requires `contender_identity_id`), `1` = Abstain,
/// `2` = Lock.
///
/// This `#[repr(u8)]` enum is the Rust-internal single source of truth for the
/// `vote_choice` discriminants used by [`cast_vote_inner`] (its `as u8`
/// discriminants drive the value validation / comparison). It is intentionally
/// **not** exported to the generated C header: the FFI parameter is a plain
/// `u8`, and the Swift side mirrors the discriminants in
/// `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ContestVoteState.swift`.
///
/// The `vote_choice` FFI parameter is deliberately a plain `u8`, **not** this
/// enum: a C/Swift caller can pass any byte, and materializing an
/// out-of-range value as a `#[repr(u8)]` enum is undefined behavior. Keeping
/// the boundary type a `u8` lets [`cast_vote_inner`] validate the value and
/// reject anything outside `0..=2` with `InvalidParameter` instead of risking
/// UB. The enum's `as u8` discriminants are the comparison source of truth.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContestedResourceVoteChoiceFFI {
    /// Vote for a specific contender; requires `contender_identity_id`.
    TowardsIdentity = 0,
    /// Abstain from the contest.
    Abstain = 1,
    /// Vote that nobody should win the contested resource.
    Lock = 2,
}

/// Cast a masternode contested-resource vote and wait for the response.
///
/// Builds a `ResourceVote` over the contested-document vote poll
/// `(contract_id, document_type_name, index_name, index_values)` with the
/// given `vote_choice`, signs it with the masternode voting key derived from
/// `voting_private_key`, and broadcasts it for `voter_pro_tx_hash`.
///
/// # Parameters
/// * `sdk_handle` — handle to an initialized SDK instance.
/// * `contract_id` — base58-encoded contested resource's data-contract id
///   (DPNS for username contests).
/// * `document_type_name` — contested document type (e.g. `"domain"`).
/// * `index_name` — contested index name (e.g. `"parentNameAndLabel"`).
/// * `index_values_json` — JSON array of index values. Each element is decoded
///   via an explicit type tag: a `"0x"`-prefixed string is hex-decoded to
///   `Value::Bytes`, and any other string is taken verbatim as `Value::Text`.
///   DPNS index values are text labels, so typical callers pass plain text
///   (no `0x`). This matches the parsing used by the vote-state query FFI.
/// * `vote_choice` — discriminant byte matching
///   [`ContestedResourceVoteChoiceFFI`]: `TowardsIdentity` (`0`), `Abstain`
///   (`1`) or `Lock` (`2`). Any other value is rejected with
///   `InvalidParameter`.
/// * `contender_identity_id` — base58-encoded contender identity; required
///   (and only used) when `vote_choice == 0`, otherwise ignored / may be null.
/// * `voter_pro_tx_hash` — pointer to the masternode's 32-byte pro_tx_hash.
/// * `voting_private_key` — pointer to the 32-byte masternode voting private
///   key. Both the signer and the matching `ECDSA_HASH160` voting public key
///   are derived from this; the key never leaves this call as raw bytes.
/// * `network` — network used for WIF encoding inside `SingleKeySigner`
///   (does not affect signing or the derived key data).
///
/// # Returns
/// `DashSDKResult` with no data on success, or an error. A vote from a
/// non-masternode wallet is expected to fail with an authorization-style
/// error from the platform — that is the correct deterministic outcome.
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized
///   `SDKHandle`.
/// - `contract_id`, `document_type_name`, `index_name`, `index_values_json`
///   must be valid NUL-terminated C strings for the duration of the call.
/// - `contender_identity_id` may be null unless `vote_choice == 0`.
/// - `voter_pro_tx_hash` and `voting_private_key` must each point to 32
///   readable bytes for the duration of the call.
/// - All pointers must reference readable memory; passing dangling or
///   misaligned pointers is undefined behavior.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_contested_resource_cast_vote(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    vote_choice: u8,
    contender_identity_id: *const c_char,
    voter_pro_tx_hash: *const u8,
    voting_private_key: *const u8,
    network: FFINetwork,
) -> DashSDKResult {
    match cast_vote_inner(
        sdk_handle,
        contract_id,
        document_type_name,
        index_name,
        index_values_json,
        vote_choice,
        contender_identity_id,
        voter_pro_tx_hash,
        voting_private_key,
        network,
    ) {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn cast_vote_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    vote_choice: u8,
    contender_identity_id: *const c_char,
    voter_pro_tx_hash: *const u8,
    voting_private_key: *const u8,
    network: FFINetwork,
) -> Result<(), FFIError> {
    // ---- Null checks ---------------------------------------------------------
    if sdk_handle.is_null() {
        return Err(invalid("SDK handle is null"));
    }
    if contract_id.is_null()
        || document_type_name.is_null()
        || index_name.is_null()
        || index_values_json.is_null()
    {
        return Err(invalid(
            "contract_id, document_type_name, index_name and index_values_json must be non-null",
        ));
    }
    if voter_pro_tx_hash.is_null() {
        return Err(invalid("voter_pro_tx_hash is null"));
    }
    if voting_private_key.is_null() {
        return Err(invalid("voting_private_key is null"));
    }

    // ---- Parse the vote poll strings ----------------------------------------
    let contract_id_str = cstr(contract_id, "contract_id")?;
    let document_type_name_str = cstr(document_type_name, "document_type_name")?;
    let index_name_str = cstr(index_name, "index_name")?;
    let index_values_str = cstr(index_values_json, "index_values_json")?;

    let contract_id_bytes = bs58::decode(contract_id_str)
        .into_vec()
        .map_err(|e| invalid(&format!("Failed to decode contract id: {}", e)))?;
    let contract_id_arr: [u8; 32] = contract_id_bytes
        .try_into()
        .map_err(|_| invalid("contract id must be exactly 32 bytes"))?;
    let contract_identifier = Identifier::new(contract_id_arr);

    // Same index-value parsing as the vote-state query FFI, via the shared
    // `parse_index_value` helper: a `"0x"`-prefixed value is hex-decoded to
    // `Value::Bytes`, anything else is taken verbatim as `Value::Text`. Using
    // the single shared helper keeps the read and write paths in lockstep, so a
    // poll a caller can read with `get_vote_state` is the same poll it votes on
    // here. An explicit `0x` tag is required (rather than guessing by content)
    // because this is a signing surface: a legit DPNS text label that happens
    // to be even-length all-hex must not be silently re-encoded as bytes.
    let index_values_array: Vec<String> = serde_json::from_str(index_values_str)
        .map_err(|e| invalid(&format!("Failed to parse index_values JSON: {}", e)))?;
    let index_values: Vec<Value> = index_values_array
        .into_iter()
        .map(crate::contested_resource::parse_index_value)
        .collect::<Result<Vec<Value>, String>>()
        .map_err(|e| invalid(&e))?;

    let vote_poll =
        VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
            contract_id: contract_identifier,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            index_values,
        });

    // ---- Resolve the vote choice --------------------------------------------
    // Compare the raw `u8` against the enum's `as u8` discriminants rather than
    // taking the enum by value: a C caller can hand us any byte, and an
    // out-of-range value is not a valid `#[repr(u8)]` discriminant. Matching on
    // the byte keeps a defensive `other` arm that rejects such values with
    // `InvalidParameter` instead of risking undefined behavior.
    let resource_vote_choice = match vote_choice {
        x if x == ContestedResourceVoteChoiceFFI::TowardsIdentity as u8 => {
            if contender_identity_id.is_null() {
                return Err(invalid(
                    "contender_identity_id is required when vote_choice is TowardsIdentity (0)",
                ));
            }
            let contender_str = cstr(contender_identity_id, "contender_identity_id")?;
            let contender_bytes = bs58::decode(contender_str)
                .into_vec()
                .map_err(|e| invalid(&format!("Failed to decode contender id: {}", e)))?;
            let contender_arr: [u8; 32] = contender_bytes
                .try_into()
                .map_err(|_| invalid("contender id must be exactly 32 bytes"))?;
            ResourceVoteChoice::TowardsIdentity(Identifier::new(contender_arr))
        }
        x if x == ContestedResourceVoteChoiceFFI::Abstain as u8 => ResourceVoteChoice::Abstain,
        x if x == ContestedResourceVoteChoiceFFI::Lock as u8 => ResourceVoteChoice::Lock,
        other => {
            return Err(invalid(&format!(
                "vote_choice must be 0 (TowardsIdentity), 1 (Abstain) or 2 (Lock); got {}",
                other
            )));
        }
    };

    let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll,
        resource_vote_choice,
    }));

    // ---- pro_tx_hash ---------------------------------------------------------
    let pro_tx_hash_slice = std::slice::from_raw_parts(voter_pro_tx_hash, 32);
    let mut pro_tx_hash_arr = [0u8; 32];
    pro_tx_hash_arr.copy_from_slice(pro_tx_hash_slice);
    let pro_tx_hash = Identifier::new(pro_tx_hash_arr);

    // ---- Derive the masternode voting key + signer --------------------------
    let network: Network = network.into();

    // Hold the private key in a zeroizing buffer so it does not linger.
    let key_slice = std::slice::from_raw_parts(voting_private_key, 32);
    let mut key_array = Zeroizing::new([0u8; 32]);
    key_array.copy_from_slice(key_slice);

    let signer = SingleKeySigner::new_from_slice(key_array.as_slice(), network)
        .map_err(|e| invalid(&format!("Invalid voting private key: {}", e)))?;

    // The masternode voting key is the 20-byte hash160 of the (compressed)
    // public key. `MasternodeVoteTransition` calls
    // `masternode_voting_key.public_key_hash()` to derive the voter
    // identifier, and `SingleKeySigner::can_sign_with` recomputes the same
    // hash160 from the private key, so the two agree by construction.
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_byte_array(&key_array)
        .map_err(|e| invalid(&format!("Invalid voting private key: {}", e)))?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let voting_address = hash160::Hash::hash(&public_key.serialize()).to_byte_array();

    // ---- Resolve the real voting key off the voter identity -----------------
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = &wrapper.sdk;

    let voter_identifier = Identifier::create_voter_identifier(&pro_tx_hash_arr, &voting_address);

    let masternode_voting_key: IdentityPublicKey = wrapper.runtime.block_on(async {
        // The key id is NOT reliably 0. Platform assigns 0 when it *creates* a
        // voter identity, but a voting-key rotation disables the old
        // identity's keys and creates a fresh identity for the new address
        // (`update_voter_identity_v0`), and nothing guarantees the caller's key
        // matches the identity that actually exists. Fabricating id 0 turned
        // every one of those cases into Platform's opaque
        // "Public key 0 doesn't exist" at broadcast time.
        let identity = Identity::fetch(sdk, voter_identifier)
            .await
            .map_err(FFIError::from)?
            .ok_or_else(|| {
                FFIError::InvalidParameter(format!(
                    "No voting identity exists on Platform for masternode {} with this voting key \
                     (expected voter identity {}). Either the voting key does not match the \
                     masternode's registered voting address, or Platform has not created the \
                     voter identity yet.",
                    pro_tx_hash, voter_identifier
                ))
            })?;

        // Match on the key's own data, not on position: that is what the
        // signer can actually sign with.
        identity
            .public_keys()
            .values()
            .find(|key| {
                key.purpose() == Purpose::VOTING
                    && key.key_type() == KeyType::ECDSA_HASH160
                    && key.data().as_slice() == voting_address
                    && key.disabled_at().is_none()
            })
            .cloned()
            .ok_or_else(|| {
                FFIError::InvalidParameter(format!(
                    "Voter identity {} has no enabled ECDSA_HASH160 voting key matching this \
                     private key. The masternode's voting key may have been rotated.",
                    voter_identifier
                ))
            })
    })?;

    // ---- Broadcast ----------------------------------------------------------
    wrapper.runtime.block_on(async {
        vote.put_to_platform_and_wait_for_response(
            pro_tx_hash,
            &masternode_voting_key,
            sdk,
            &signer,
            None,
        )
        .await
        .map_err(FFIError::from)
    })?;

    Ok(())
}

fn invalid(message: &str) -> FFIError {
    FFIError::InvalidParameter(message.to_string())
}

unsafe fn cstr<'a>(ptr: *const c_char, field: &str) -> Result<&'a str, FFIError> {
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|e| invalid(&format!("Invalid UTF-8 in {}: {}", field, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use std::ffi::CString;

    /// Real base58 DPNS data-contract id (32 bytes). Using the genuine id
    /// means the bs58-decode + 32-byte length check passes, so tests reach
    /// the branches they claim to cover (missing-contender, vote-choice match)
    /// instead of short-circuiting on a bogus contract id.
    const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";

    #[test]
    fn test_cast_vote_null_handle() {
        // Bind every CString to a local so the backing buffer outlives the FFI
        // call; passing `CString::new(..).unwrap().as_ptr()` inline drops the
        // temporary before the call reads the pointer (use-after-free).
        let contract_id = CString::new(DPNS_CONTRACT_ID).unwrap();
        let document_type_name = CString::new("domain").unwrap();
        let index_name = CString::new("parentNameAndLabel").unwrap();
        let index_values = CString::new(r#"["dash","alice"]"#).unwrap();
        unsafe {
            let pro_tx = [1u8; 32];
            let priv_key = [2u8; 32];
            let result = dash_sdk_contested_resource_cast_vote(
                std::ptr::null(),
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                index_name.as_ptr(),
                index_values.as_ptr(),
                ContestedResourceVoteChoiceFFI::Abstain as u8,
                std::ptr::null(),
                pro_tx.as_ptr(),
                priv_key.as_ptr(),
                FFINetwork::Testnet,
            );
            assert!(!result.error.is_null());
            // Free the DashSDKError (struct + message string) the result owns;
            // models the caller contract and keeps leak sanitizers happy.
            dash_sdk_error_free(result.error);
        }
    }

    #[test]
    fn test_cast_vote_towards_identity_requires_contender() {
        let handle = create_mock_sdk_handle();
        // Bind every CString to a local so the backing buffer outlives the FFI
        // call; passing `CString::new(..).unwrap().as_ptr()` inline drops the
        // temporary before the call reads the pointer (use-after-free).
        let contract_id = CString::new(DPNS_CONTRACT_ID).unwrap();
        let document_type_name = CString::new("domain").unwrap();
        let index_name = CString::new("parentNameAndLabel").unwrap();
        let index_values = CString::new(r#"["dash","alice"]"#).unwrap();
        unsafe {
            let pro_tx = [1u8; 32];
            let priv_key = [2u8; 32];
            let result = dash_sdk_contested_resource_cast_vote(
                handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                index_name.as_ptr(),
                index_values.as_ptr(),
                ContestedResourceVoteChoiceFFI::TowardsIdentity as u8,
                std::ptr::null(), // missing contender id
                pro_tx.as_ptr(),
                priv_key.as_ptr(),
                FFINetwork::Testnet,
            );
            assert!(!result.error.is_null());
            let err = &*result.error;
            assert_eq!(err.code, DashSDKErrorCode::InvalidParameter);
            // Free the DashSDKError (struct + message string) the result owns,
            // after the last use of `err`; models the caller contract and keeps
            // leak sanitizers happy.
            dash_sdk_error_free(result.error);
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn test_cast_vote_invalid_choice() {
        let handle = create_mock_sdk_handle();
        // Bind every CString to a local so the backing buffer outlives the FFI
        // call; passing `CString::new(..).unwrap().as_ptr()` inline drops the
        // temporary before the call reads the pointer (use-after-free).
        let contract_id = CString::new(DPNS_CONTRACT_ID).unwrap();
        let document_type_name = CString::new("domain").unwrap();
        let index_name = CString::new("parentNameAndLabel").unwrap();
        let index_values = CString::new(r#"["dash","alice"]"#).unwrap();
        unsafe {
            let pro_tx = [1u8; 32];
            let priv_key = [2u8; 32];
            // A C caller can pass any byte for the u8 `vote_choice`; an
            // out-of-range value must hit the defensive `other` arm and be
            // rejected with `InvalidParameter`.
            let result = dash_sdk_contested_resource_cast_vote(
                handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                index_name.as_ptr(),
                index_values.as_ptr(),
                99, // invalid discriminant
                std::ptr::null(),
                pro_tx.as_ptr(),
                priv_key.as_ptr(),
                FFINetwork::Testnet,
            );
            assert!(!result.error.is_null());
            let err = &*result.error;
            assert_eq!(err.code, DashSDKErrorCode::InvalidParameter);
            // Free the DashSDKError (struct + message string) the result owns,
            // after the last use of `err`; models the caller contract and keeps
            // leak sanitizers happy.
            dash_sdk_error_free(result.error);
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
