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
use dash_sdk::dpp::identifier::MasternodeIdentifiers;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::BinaryData;
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

    // Platform assigns a voter identity's voting key id 0 and nothing else:
    // `create_voter_identity_v0` passes 0, and a rotation creates a DIFFERENT
    // identity (the identifier includes the voting address) whose key is also
    // 0. So the happy path needs no lookup — build the key Platform holds and
    // broadcast. `SingleKeySigner::can_sign_with` recomputes the same hash160
    // from the private key, so the key and the signer agree by construction.
    let masternode_voting_key: IdentityPublicKey = IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::VOTING,
        security_level: SecurityLevel::HIGH,
        key_type: KeyType::ECDSA_HASH160,
        read_only: true,
        data: BinaryData::new(voting_address.to_vec()),
        disabled_at: None,
        contract_bounds: None,
    }
    .into();

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
    if !is_voter_key_failure(&broadcast_error) {
        return Err(FFIError::from(broadcast_error));
    }

    // A missing voter identity and a rotated (disabled) key are
    // indistinguishable from Platform's side — both surface as
    // "Public key 0 doesn't exist". Fetching the identity says which, but only
    // here: doing it before every cast would spend a round trip per
    // (node, contest) on runs that overwhelmingly succeed.
    let voter_identifier = Identifier::create_voter_identifier(&pro_tx_hash_arr, &voting_address);
    let diagnosis = wrapper.runtime.block_on(diagnose_vote_failure(
        sdk,
        &pro_tx_hash,
        &voter_identifier,
        &voting_address,
    ));

    // Still fall back to the original when the identity and key both check out
    // — the signature failure was about something else on the key path.
    Err(diagnosis.unwrap_or_else(|| FFIError::from(broadcast_error)))
}

/// Whether a broadcast failure is Platform rejecting the VOTING KEY, as opposed
/// to anything else that can fail a vote.
///
/// Matched on the typed consensus error rather than its rendered text: the
/// three signature variants below are exactly the ones the identity fetch can
/// explain, and a message match would silently start diagnosing unrelated
/// failures the first time a string changed.
fn is_voter_key_failure(error: &dash_sdk::Error) -> bool {
    use dash_sdk::dpp::consensus::signature::SignatureError;
    use dash_sdk::dpp::consensus::ConsensusError;

    fn is_key_signature_error(consensus: &ConsensusError) -> bool {
        matches!(
            consensus,
            ConsensusError::SignatureError(
                SignatureError::IdentityNotFoundError(_)
                    | SignatureError::MissingPublicKeyError(_)
                    | SignatureError::PublicKeyIsDisabledError(_)
            )
        )
    }

    match error {
        // Rejected at broadcast: the consensus error rides on the response.
        dash_sdk::Error::StateTransitionBroadcastError(e) => {
            e.cause.as_ref().is_some_and(is_key_signature_error)
        }
        // Rejected locally / surfaced as a protocol error.
        dash_sdk::Error::Protocol(dash_sdk::dpp::ProtocolError::ConsensusError(e)) => {
            is_key_signature_error(e)
        }
        _ => false,
    }
}

/// Explain a failed vote broadcast, or `None` when the voter identity and its
/// voting key are both fine and the failure lies elsewhere.
///
/// Runs only after a broadcast has already failed, so its cost is paid on the
/// path that is already lost.
async fn diagnose_vote_failure(
    sdk: &dash_sdk::Sdk,
    pro_tx_hash: &Identifier,
    voter_identifier: &Identifier,
    voting_address: &[u8; 20],
) -> Option<FFIError> {
    // A fetch that itself fails tells us nothing; leave the original error.
    let fetched = Identity::fetch(sdk, *voter_identifier).await.ok()?;

    let Some(identity) = fetched else {
        return Some(missing_voter_identity(pro_tx_hash, voter_identifier));
    };
    select_voting_key(&identity, voting_address, voter_identifier).err()
}

fn invalid(message: &str) -> FFIError {
    FFIError::InvalidParameter(message.to_string())
}

/// Platform holds no voter identity for this `(pro_tx_hash, voting address)`.
fn missing_voter_identity(pro_tx_hash: &Identifier, voter_identifier: &Identifier) -> FFIError {
    FFIError::InvalidParameter(format!(
        "No voting identity exists on Platform for masternode {} with this voting key \
         (expected voter identity {}). Either the voting key does not match the \
         masternode's registered voting address, or Platform has not created the \
         voter identity yet.",
        pro_tx_hash, voter_identifier
    ))
}

/// Pick the voting key the caller's private key can actually sign with.
///
/// Matches on the key's own data rather than its position. Platform does
/// assign the voting key id 0, so position would usually work — but it
/// silently picks the wrong key on an identity carrying other keys, and it
/// cannot tell a usable key from one `update_voter_identity_v0` disabled
/// during a rotation. `disabled_at` is therefore part of the match, not an
/// afterthought: a disabled key exists and would be selected by id.
fn select_voting_key(
    identity: &Identity,
    voting_address: &[u8; 20],
    voter_identifier: &Identifier,
) -> Result<IdentityPublicKey, FFIError> {
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

    // ---- select_voting_key --------------------------------------------------
    //
    // `Identity::fetch` needs a live SDK, so the identity *lookup* stays
    // integration-shaped. Key *selection* is the part that decides whether a
    // vote can be signed, and it is pure — these cover it directly.

    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::v0::IdentityV0;
    use dash_sdk::dpp::identity::SecurityLevel;
    use dash_sdk::dpp::platform_value::BinaryData;
    use std::collections::BTreeMap;

    const VOTING_ADDRESS: [u8; 20] = [7u8; 20];
    const OTHER_ADDRESS: [u8; 20] = [9u8; 20];

    fn voter_id() -> Identifier {
        Identifier::new([3u8; 32])
    }

    fn key(
        id: u32,
        purpose: Purpose,
        key_type: KeyType,
        data: [u8; 20],
        disabled_at: Option<u64>,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(data.to_vec()),
            disabled_at,
        })
    }

    fn identity_with(keys: Vec<IdentityPublicKey>) -> Identity {
        let mut public_keys = BTreeMap::new();
        for k in keys {
            public_keys.insert(k.id(), k);
        }
        Identity::V0(IdentityV0 {
            id: voter_id(),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    #[test]
    fn selects_the_enabled_voting_key_matching_the_private_key() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            None,
        )]);
        let selected = select_voting_key(&identity, &VOTING_ADDRESS, &voter_id())
            .expect("the matching enabled key should be selected");
        assert_eq!(selected.data().as_slice(), &VOTING_ADDRESS);
    }

    #[test]
    fn selects_by_data_not_by_position() {
        // The real id is not 0 here. Selecting by position would take the
        // AUTHENTICATION key and sign with something the signer cannot back.
        let identity = identity_with(vec![
            key(
                0,
                Purpose::AUTHENTICATION,
                KeyType::ECDSA_HASH160,
                OTHER_ADDRESS,
                None,
            ),
            key(
                4,
                Purpose::VOTING,
                KeyType::ECDSA_HASH160,
                VOTING_ADDRESS,
                None,
            ),
        ]);
        let selected = select_voting_key(&identity, &VOTING_ADDRESS, &voter_id())
            .expect("the voting key should be found at a non-zero id");
        assert_eq!(selected.id(), 4);
        assert_eq!(selected.purpose(), Purpose::VOTING);
    }

    #[test]
    fn rejects_a_disabled_voting_key() {
        // What a rotation leaves behind: `update_voter_identity_v0` disables
        // the old identity's keys rather than removing them, so the key exists
        // and would be picked by id.
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            Some(1_700_000_000),
        )]);
        let err = select_voting_key(&identity, &VOTING_ADDRESS, &voter_id())
            .expect_err("a disabled key must not be selected");
        assert!(
            format!("{:?}", err).contains("may have been rotated"),
            "expected the rotation diagnostic, got: {:?}",
            err
        );
    }

    #[test]
    fn rejects_a_voting_key_for_a_different_address() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            OTHER_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS, &voter_id()).is_err());
    }

    // ---- is_voter_key_failure -----------------------------------------------
    //
    // The gate deciding whether a failed broadcast gets a key diagnosis. Its
    // absence was a real defect: diagnosis ran on EVERY failure, so a vote that
    // arrived after the poll closed, cast by a node with no voter identity,
    // was reported as "no voting identity exists" — replacing the true cause
    // with a plausible-looking wrong one.

    use dash_sdk::dpp::consensus::signature::{
        BasicECDSAError, IdentityNotFoundError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    };
    use dash_sdk::dpp::consensus::ConsensusError;

    fn broadcast_error_with(cause: ConsensusError) -> dash_sdk::Error {
        dash_sdk::Error::StateTransitionBroadcastError(
            dash_sdk::error::StateTransitionBroadcastError {
                code: 1,
                message: "rejected".to_string(),
                cause: Some(cause),
            },
        )
    }

    #[test]
    fn key_failures_are_diagnosable() {
        for cause in [
            ConsensusError::SignatureError(SignatureError::MissingPublicKeyError(
                MissingPublicKeyError::new(0),
            )),
            ConsensusError::SignatureError(SignatureError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(0),
            )),
            ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(Identifier::new([3u8; 32])),
            )),
        ] {
            assert!(
                is_voter_key_failure(&broadcast_error_with(cause.clone())),
                "{cause:?} is exactly what the identity fetch explains"
            );
        }
    }

    /// The regression this gate exists for: a failure unrelated to the key must
    /// keep its own error even though the identity may well be absent.
    #[test]
    fn unrelated_failures_are_not_diagnosed() {
        // A signature failure that is NOT about the key's existence or state.
        // The identity fetch cannot explain it, so it must keep its own error —
        // this is the discrimination the gate exists for, not merely
        // "signature vs not signature".
        let other_signature_failure = broadcast_error_with(ConsensusError::SignatureError(
            SignatureError::BasicECDSAError(BasicECDSAError::new("bad signature".to_string())),
        ));
        assert!(!is_voter_key_failure(&other_signature_failure));

        // A transport failure carries no consensus cause at all.
        let transport = dash_sdk::Error::StateTransitionBroadcastError(
            dash_sdk::error::StateTransitionBroadcastError {
                code: 2,
                message: "connection reset".to_string(),
                cause: None,
            },
        );
        assert!(!is_voter_key_failure(&transport));
    }

    /// The purpose predicate must be load-bearing on its own.
    ///
    /// The only other AUTHENTICATION fixture also uses a different address, so
    /// it is already rejected by the address check — deleting
    /// `purpose() == VOTING` would leave every other test passing. This pins it
    /// with an AUTHENTICATION key at the CORRECT address and key type, where
    /// purpose is the only thing that can reject it.
    #[test]
    fn rejects_a_matching_address_under_the_wrong_purpose() {
        let identity = identity_with(vec![key(
            0,
            Purpose::AUTHENTICATION,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS, &voter_id()).is_err());
    }

    #[test]
    fn rejects_a_matching_address_under_the_wrong_key_type() {
        // Same 20 bytes, but not a key this signer can sign with.
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::BIP13_SCRIPT_HASH,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS, &voter_id()).is_err());
    }

    #[test]
    fn missing_voter_identity_names_both_identifiers() {
        let pro_tx = Identifier::new([1u8; 32]);
        let msg = format!("{:?}", missing_voter_identity(&pro_tx, &voter_id()));
        assert!(msg.contains(&format!("{}", pro_tx)));
        assert!(msg.contains(&format!("{}", voter_id())));
        // The diagnostic must read as prose — it crosses the FFI boundary and
        // is shown verbatim to users.
        assert!(!msg.contains("  "), "message has whitespace runs: {}", msg);
    }
}
