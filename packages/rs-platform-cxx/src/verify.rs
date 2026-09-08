// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Drive proved-response verification for the DAPI queries exposed by this
//! binding, built on drive-proof-verifier's `FromProof`. Each function takes
//! the exact protobuf request the transport sent plus the full protobuf
//! response it received, reconstructs the query from the request, replays the
//! GroveDB proof, and verifies the Tenderdash BLS quorum threshold signature
//! against the keys served by [`crate::provider`].
//!
//! Every byte that enters here comes from an untrusted node, and the GroveDB
//! replay necessarily runs before the signature check (the root hash only
//! exists after replay). Inputs are therefore size-capped at the door, and
//! the caller's freshness policy runs on the returned [`Meta`] — the
//! signature binds those fields, nothing here decides whether they are recent
//! enough.

use dapi_grpc::platform::v0::{
    GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse,
    GetDocumentsRequest, GetDocumentsResponse, GetIdentityByPublicKeyHashRequest,
    GetIdentityByPublicKeyHashResponse, GetIdentityContractNonceRequest,
    GetIdentityContractNonceResponse, GetIdentityNonceRequest, GetIdentityNonceResponse,
    GetIdentityRequest, GetIdentityResponse, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider as _;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identity::Identity;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo as WinnerInfo;
use drive_proof_verifier::from_request::TryFromRequest;
use drive_proof_verifier::types::{
    Contenders, Documents, IdentityContractNonceFetcher, IdentityNonceFetcher,
};
use drive_proof_verifier::{DocumentWireQuery, FromProof, RequestedDocuments};
use platform_version::version::PlatformVersion;
use prost::Message;

use crate::decode::identity_info;
use crate::provider::{context, provider};
use crate::types::{ContestedVoteState, IdentityInfo, Meta};

/// Largest request or response the bridge will decode. DAPI's gRPC servers
/// cap messages well below this; a larger blob is not a Platform response
/// and would only be an attempt to make the decoders allocate.
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

fn decode_message<T: Message + Default>(bytes: &[u8], what: &str) -> Result<T, String> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(format!(
            "{what} is {} bytes, above the {MAX_MESSAGE_BYTES}-byte limit",
            bytes.len()
        ));
    }
    T::decode(bytes).map_err(|e| format!("unable to decode {what}: {e}"))
}

fn meta_from(mtd: &ResponseMetadata) -> Meta {
    Meta {
        height: mtd.height,
        core_chain_locked_height: mtd.core_chain_locked_height,
        time_ms: mtd.time_ms,
        protocol_version: mtd.protocol_version,
        chain_id: mtd.chain_id.clone(),
    }
}

/// The platform version the response claims to be produced under. The claim
/// is authenticated after the fact: `protocol_version` enters the signed
/// StateId, so a lie fails the quorum signature check. A version this build
/// does not know cannot be verified at all.
fn response_version<R: VersionedGrpcResponse>(
    response: &R,
) -> Result<&'static PlatformVersion, String>
where
    <R as VersionedGrpcResponse>::Error: std::fmt::Display,
{
    let mtd = response
        .metadata()
        .map_err(|e| format!("response has no metadata: {e}"))?;
    PlatformVersion::get(mtd.protocol_version).map_err(|e| {
        format!(
            "response claims protocol version {} this build does not know: {e}",
            mtd.protocol_version
        )
    })
}

macro_rules! verify_with {
    ($request:expr, $response:expr, $req_ty:ty, $resp_ty:ty, $out_ty:ty, $what:literal) => {{
        let request: $req_ty = decode_message($request, concat!($what, " request"))?;
        let response: $resp_ty = decode_message($response, concat!($what, " response"))?;
        let version = response_version(&response)?;
        let (value, mtd, _) = <$out_ty as FromProof<$req_ty>>::maybe_from_proof_with_metadata(
            request,
            response,
            context()?.network,
            version,
            provider(),
        )
        .map_err(|e| format!(concat!($what, " proof verification failed: {}"), e))?;
        (value, meta_from(&mtd), version)
    }};
}

pub fn verify_get_identity_nonce(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<u64>, Meta), String> {
    let (nonce, meta, _) = verify_with!(
        request,
        response,
        GetIdentityNonceRequest,
        GetIdentityNonceResponse,
        IdentityNonceFetcher,
        "identity nonce"
    );
    Ok((nonce.map(|fetcher| fetcher.0), meta))
}

pub fn verify_get_identity_contract_nonce(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<u64>, Meta), String> {
    let (nonce, meta, _) = verify_with!(
        request,
        response,
        GetIdentityContractNonceRequest,
        GetIdentityContractNonceResponse,
        IdentityContractNonceFetcher,
        "identity contract nonce"
    );
    Ok((nonce.map(|fetcher| fetcher.0), meta))
}

/// getIdentity: one proof covering balance, revision and keys. `None` =
/// proven absent.
pub fn verify_get_identity(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<IdentityInfo>, Meta), String> {
    let (identity, meta, _) = verify_with!(
        request,
        response,
        GetIdentityRequest,
        GetIdentityResponse,
        Identity,
        "identity"
    );
    Ok((identity.as_ref().map(identity_info), meta))
}

/// getIdentityByPublicKeyHash: one proof resolving the unique key hash to
/// the full identity. `None` = no identity registered that key hash.
pub fn verify_get_identity_by_pubkey_hash(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<IdentityInfo>, Meta), String> {
    let (identity, meta, _) = verify_with!(
        request,
        response,
        GetIdentityByPublicKeyHashRequest,
        GetIdentityByPublicKeyHashResponse,
        Identity,
        "identity-by-public-key-hash"
    );
    Ok((identity.as_ref().map(identity_info), meta))
}

/// getDocuments: the query is reconstructed from the wire request by
/// drive-proof-verifier (`FromProof<GetDocumentsRequest>`), verified, and
/// the matched documents are returned re-serialized in platform form (the
/// input to the decode_* functions). No matching documents is an empty
/// vector.
pub fn verify_get_documents(
    request: &[u8],
    response: &[u8],
) -> Result<(Vec<Vec<u8>>, Meta), String> {
    let wire_request: GetDocumentsRequest = decode_message(request, "documents request")?;
    // The verifier binds the proof to the request's contract and document
    // type; re-serializing the verified documents needs the same pair.
    let wire_query = DocumentWireQuery::try_from_request(wire_request.clone())
        .map_err(|e| format!("documents request: {e}"))?;

    let (documents, meta, version) = verify_with!(
        request,
        response,
        GetDocumentsRequest,
        GetDocumentsResponse,
        RequestedDocuments,
        "documents"
    );
    let Some(documents) = documents else {
        return Ok((Vec::new(), meta));
    };
    let contract = provider()
        .get_data_contract(&wire_query.data_contract_id, version)
        .map_err(|e| format!("unable to resolve data contract: {e}"))?
        .ok_or("document queries support only the pinned system contracts")?;
    let document_type = contract
        .document_type_for_name(&wire_query.document_type_name)
        .map_err(|e| {
            format!(
                "unknown document type {}: {e}",
                wire_query.document_type_name
            )
        })?;
    let serialized = Documents::from(documents)
        .into_iter()
        .filter_map(|(_, document)| document)
        .map(|document| {
            document
                .serialize(document_type, &contract, version)
                .map_err(|e| format!("unable to re-serialize verified document: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((serialized, meta))
}

/// getContestedResourceVoteState (VoteTally result type). The query shape —
/// contract, document type, index values, tally options, count — is
/// reconstructed from the request itself.
pub fn verify_get_contested_vote_state(
    request: &[u8],
    response: &[u8],
) -> Result<(ContestedVoteState, Meta), String> {
    let (contenders, meta, _) = verify_with!(
        request,
        response,
        GetContestedResourceVoteStateRequest,
        GetContestedResourceVoteStateResponse,
        Contenders,
        "contested vote state"
    );
    let mut state = ContestedVoteState::default();
    if let Some(contenders) = contenders {
        state.contest_found = true;
        state.contenders = contenders
            .contenders
            .iter()
            .map(|(id, contender)| (id.to_buffer(), contender.vote_tally()))
            .collect();
        state.abstain_votes = contenders.abstain_vote_tally;
        state.lock_votes = contenders.lock_vote_tally;
        if let Some((winner_info, finalization_block)) = contenders.winner {
            state.finished = true;
            state.finished_at_time_ms = finalization_block.time_ms;
            match winner_info {
                WinnerInfo::WonByIdentity(id) => state.winner = Some(id.to_buffer()),
                WinnerInfo::Locked => state.locked = true,
                WinnerInfo::NoWinner => {}
            }
        }
    }
    Ok((state, meta))
}
