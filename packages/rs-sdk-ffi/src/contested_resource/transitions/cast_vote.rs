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
//! The voting [`IdentityPublicKey`] is **built locally, not fetched**.
//! Platform always assigns a voter identity's voting key id 0:
//! `create_voter_identity_v0` passes 0, and a rotation creates a *different*
//! identity — the identifier includes the voting address — whose key is
//! likewise 0. So the key Platform holds is knowable without a round trip, and
//! `SingleKeySigner::can_sign_with` for `ECDSA_HASH160` recomputes the same
//! `hash160(pubkey)` from the private key, so key and signer agree by
//! construction.
//!
//! # Diagnosis is deferred to the failure path
//!
//! Two things do fail, and Platform reports both as the same opaque
//! "Public key 0 doesn't exist":
//!
//!   * no voter identity exists for this `(pro_tx_hash, voting address)` pair,
//!     or
//!   * after a rotation `update_voter_identity_v0` **disabled** the old
//!     identity's keys, so key 0 exists but is unusable.
//!
//! Telling those apart needs the identity, but fetching it before every cast
//! would spend a Platform round trip per (node, contest) on runs that
//! overwhelmingly succeed — a bulk vote of 6 nodes across 10 names is 60
//! fetches. So the fetch happens only after a broadcast has already failed,
//! where the cost is paid on a path that is already lost. A diagnosis replaces
//! the opaque error; an unrelated failure (network, fees, a closed poll)
//! survives unchanged rather than being recast as a key problem.
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
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::dpp::platform_value::{Identifier, Value};
use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::dpp::voting::votes::Vote;
use dash_sdk::platform::transition::masternode_vote_keys::{
    diagnose_voting_key_failure, is_voting_key_failure, voter_identity_voting_key, VotingKeyProblem,
};
use dash_sdk::platform::transition::vote::PutVote;
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
/// * `voter_pro_tx_hash` — pointer to the masternode's 32-byte pro_tx_hash in
///   **wire order**: the orientation `Txid` stores, which is what a parsed
///   ProRegTx yields (`reg.txid()`) and what a wallet holds internally. This is
///   NOT the byte order of the hex Core displays, which is its reverse.
///
///   The two are not interchangeable. Platform identifies masternodes by the
///   opposite orientation — `ProTxHash` is declared `#[hash_newtype(forward)]`
///   while `Txid` is not — so this function reverses these bytes before
///   deriving the voter identity and building the transition. Passing display
///   order asks Platform for an identity that has never existed, and the vote
///   is rejected as having no voter identity.
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
    // Callers pass WIRE order — the orientation `Txid` stores, which is what a
    // parsed ProRegTx yields (`reg.txid()`) and what the iOS wallet holds.
    //
    // Platform identifies masternodes by the OTHER orientation. `ProTxHash` is
    // declared `#[hash_newtype(forward)]` while `Txid` is not, so
    // `ProTxHash::to_byte_array()` is display order — the reverse of a `Txid`'s
    // bytes for the same transaction. `rpc-json`'s `MasternodeListItem` holds
    // both conventions side by side (`pro_tx_hash: ProTxHash`,
    // `collateral_hash: Txid`), and drive-abci builds the voter identity from
    // `masternode.pro_tx_hash.to_byte_array()`.
    //
    // Feeding wire order to `create_voter_identifier` therefore asks Platform
    // for an identity that has never existed, and the vote is rejected as
    // having no voter identity — with the real one sitting under the reversed
    // hash. Reverse once, here, and use the corrected value for both the
    // identifier and the transition so they cannot drift apart.
    let pro_tx_hash_slice = std::slice::from_raw_parts(voter_pro_tx_hash, 32);
    let mut pro_tx_hash_arr = [0u8; 32];
    pro_tx_hash_arr.copy_from_slice(pro_tx_hash_slice);
    pro_tx_hash_arr.reverse();
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

    // ---- Broadcast, then diagnose only on failure ---------------------------
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = &wrapper.sdk;

    // The key Platform holds, built without a lookup — see the rs-sdk helper
    // for why the id is knowable. `SingleKeySigner::can_sign_with` recomputes
    // the same hash160 from the private key, so key and signer agree by
    // construction.
    let masternode_voting_key = voter_identity_voting_key(&voting_address);

    let broadcast = wrapper.runtime.block_on(async {
        vote.put_to_platform_and_wait_for_response(
            pro_tx_hash,
            &masternode_voting_key,
            sdk,
            &signer,
            None,
        )
        .await
    });

    let Err(broadcast_error) = broadcast else {
        return Ok(());
    };

    // Only a signature failure about the voting key is worth explaining. A
    // closed poll, a fee failure or a transport error says nothing about the
    // identity, and diagnosing those would let an absent voter identity
    // masquerade as their cause — reporting "no voting identity" for a vote
    // that actually arrived too late.
    if !is_voting_key_failure(&broadcast_error) {
        return Err(FFIError::from(broadcast_error));
    }

    // A missing voter identity and a rotated (disabled) key are
    // indistinguishable from Platform's side — both surface as
    // "Public key 0 doesn't exist". Fetching the identity says which, but only
    // here: doing it before every cast would spend a round trip per
    // (node, contest) on runs that overwhelmingly succeed.
    let problem = wrapper.runtime.block_on(diagnose_voting_key_failure(
        sdk,
        ProTxHash::from_byte_array(pro_tx_hash_arr),
        &voting_address,
    ));

    // Still fall back to the original when the identity and key both check out
    // — the signature failure was about something else on the key path.
    Err(problem
        .map(describe_voting_key_problem)
        .unwrap_or_else(|| FFIError::from(broadcast_error)))
}

fn invalid(message: &str) -> FFIError {
    FFIError::InvalidParameter(message.to_string())
}

unsafe fn cstr<'a>(ptr: *const c_char, field: &str) -> Result<&'a str, FFIError> {
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|e| invalid(&format!("Invalid UTF-8 in {}: {}", field, e)))
}

/// Render a [`VotingKeyProblem`] as the message a host surfaces.
///
/// rs-sdk reports the fact; phrasing it is a binding concern, so the sentence
/// lives here rather than in the SDK where every caller would inherit it.
fn describe_voting_key_problem(problem: VotingKeyProblem) -> FFIError {
    match problem {
        VotingKeyProblem::NoVoterIdentity {
            pro_tx_hash,
            expected_voter_identity,
        } => FFIError::InvalidParameter(format!(
            "No voting identity exists on Platform for masternode {} with this voting key \
             (expected voter identity {}). Either the voting key does not match the \
             masternode's registered voting address, or Platform has not created the \
             voter identity yet.",
            Identifier::new(pro_tx_hash.to_byte_array()),
            expected_voter_identity
        )),
        VotingKeyProblem::NoUsableVotingKey { voter_identity } => {
            FFIError::InvalidParameter(format!(
                "Voter identity {} has no enabled ECDSA_HASH160 voting key matching this \
                 private key. The masternode's voting key may have been rotated.",
                voter_identity
            ))
        }
    }
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

    // ---- describe_voting_key_problem ----------------------------------------
    //
    // Selection, the failure gate and the diagnosis itself now live in rs-sdk
    // (`platform::transition::masternode_vote_keys`) and are tested there. What
    // remains here is the one thing this layer owns: turning the SDK's typed
    // fact into the sentence a host shows.

    use dash_sdk::dpp::dashcore::ProTxHash;
    use dash_sdk::platform::transition::masternode_vote_keys::VotingKeyProblem;

    #[test]
    fn missing_identity_message_names_both_identifiers() {
        let pro_tx = ProTxHash::from_byte_array([1u8; 32]);
        let expected = Identifier::new([3u8; 32]);
        let msg = format!(
            "{:?}",
            describe_voting_key_problem(VotingKeyProblem::NoVoterIdentity {
                pro_tx_hash: pro_tx,
                expected_voter_identity: expected,
            })
        );

        assert!(msg.contains(&format!("{}", Identifier::new(pro_tx.to_byte_array()))));
        assert!(msg.contains(&format!("{}", expected)));
        // The message crosses the FFI boundary verbatim and is shown to users,
        // so it must read as prose — an earlier version rendered with blank
        // gaps from an unescaped multi-line literal.
        assert!(!msg.contains("  "), "message has whitespace runs: {msg}");
    }

    #[test]
    fn rotated_key_message_names_the_identity_and_the_cause() {
        let voter = Identifier::new([5u8; 32]);
        let msg = format!(
            "{:?}",
            describe_voting_key_problem(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: voter,
            })
        );

        assert!(msg.contains(&format!("{}", voter)));
        assert!(msg.contains("rotated"));
        assert!(!msg.contains("  "), "message has whitespace runs: {msg}");
    }
}
