// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Drive proved-response verification for the DAPI queries exposed by this
//! binding, built on drive-proof-verifier's `FromProof`. Each function takes
//! the exact protobuf request the transport sent plus the full protobuf
//! response it received, reconstructs the query from the request, replays the
//! GroveDB proof, and verifies the Tenderdash BLS quorum threshold signature
//! against the keys served by [`crate::provider`].

use platform_version::version::PlatformVersion;

use crate::types::{ContestedVoteState, KeyInfo};

// ---------------------------------------------------------------------------
// FromProof-driven verification over (request bytes, response bytes).
//
// The C++ transport hands over the exact protobuf request it sent and the
// full protobuf response it received; drive-proof-verifier reconstructs the
// query from the request, replays the GroveDB proof, and verifies the
// Tenderdash quorum threshold signature against the quorum keys served by
// crate::provider. The returned Meta fields are authenticated by that
// signature.
// ---------------------------------------------------------------------------

use dapi_grpc::platform::v0::get_documents_request::Version as GetDocumentsVersion;
use dapi_grpc::platform::v0::{
    GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse,
    GetDocumentsRequest, GetDocumentsResponse, GetIdentityBalanceRequest,
    GetIdentityBalanceResponse, GetIdentityByPublicKeyHashRequest,
    GetIdentityByPublicKeyHashResponse, GetIdentityContractNonceRequest,
    GetIdentityContractNonceResponse, GetIdentityKeysRequest, GetIdentityKeysResponse,
    GetIdentityNonceRequest, GetIdentityNonceResponse, GetIdentityRequest, GetIdentityResponse,
    ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider as _;
use dash_platform_queries::documents::document_query::verify_documents_response;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identity::Identity;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo as WinnerInfo;
use drive_proof_verifier::types::{
    Contenders, IdentityBalance, IdentityContractNonceFetcher, IdentityNonceFetcher,
    IdentityPublicKeys,
};
use drive_proof_verifier::FromProof;
use prost::Message;

use crate::decode::identity_info;
use crate::provider::{network, provider};
use crate::types::{IdentityInfo, Meta};

fn decode_message<T: Message + Default>(bytes: &[u8], what: &str) -> Result<T, String> {
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

/// The platform version the response claims to be produced under, falling
/// back to the crate's latest known version for responses from a newer
/// server. The claimed version is authenticated after the fact: it enters
/// the signed StateId, so a lie fails the quorum signature check.
fn response_version<R: VersionedGrpcResponse>(response: &R) -> &'static PlatformVersion
where
    <R as VersionedGrpcResponse>::Error: std::fmt::Display,
{
    response
        .metadata()
        .ok()
        .and_then(|mtd| PlatformVersion::get(mtd.protocol_version).ok())
        .unwrap_or_else(PlatformVersion::latest)
}

pub fn verify_get_identity_nonce(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<u64>, Meta), String> {
    let request: GetIdentityNonceRequest = decode_message(request, "GetIdentityNonceRequest")?;
    let response: GetIdentityNonceResponse = decode_message(response, "GetIdentityNonceResponse")?;
    let version = response_version(&response);
    let (nonce, mtd, _) = IdentityNonceFetcher::maybe_from_proof_with_metadata(
        request,
        response,
        network()?,
        version,
        provider(),
    )
    .map_err(|e| format!("identity nonce proof verification failed: {e}"))?;
    Ok((nonce.map(|fetcher| fetcher.0), meta_from(&mtd)))
}

pub fn verify_get_identity_contract_nonce(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<u64>, Meta), String> {
    let request: GetIdentityContractNonceRequest =
        decode_message(request, "GetIdentityContractNonceRequest")?;
    let response: GetIdentityContractNonceResponse =
        decode_message(response, "GetIdentityContractNonceResponse")?;
    let version = response_version(&response);
    let (nonce, mtd, _) = IdentityContractNonceFetcher::maybe_from_proof_with_metadata(
        request,
        response,
        network()?,
        version,
        provider(),
    )
    .map_err(|e| format!("identity contract nonce proof verification failed: {e}"))?;
    Ok((nonce.map(|fetcher| fetcher.0), meta_from(&mtd)))
}

/// getIdentity: one proof covering balance, revision and keys
/// (Drive::verify_full_identity_by_identity_id). `None` = proven absent.
pub fn verify_get_identity(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<IdentityInfo>, Meta), String> {
    let request: GetIdentityRequest = decode_message(request, "GetIdentityRequest")?;
    let response: GetIdentityResponse = decode_message(response, "GetIdentityResponse")?;
    let version = response_version(&response);
    let (identity, mtd, _) =
        <Identity as FromProof<GetIdentityRequest>>::maybe_from_proof_with_metadata(
            request,
            response,
            network()?,
            version,
            provider(),
        )
        .map_err(|e| format!("identity proof verification failed: {e}"))?;
    Ok((identity.as_ref().map(identity_info), meta_from(&mtd)))
}

/// getIdentityByPublicKeyHash: one proof resolving the unique key hash to
/// the full identity. `None` = no identity registered that key hash.
pub fn verify_get_identity_by_pubkey_hash(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<IdentityInfo>, Meta), String> {
    let request: GetIdentityByPublicKeyHashRequest =
        decode_message(request, "GetIdentityByPublicKeyHashRequest")?;
    let response: GetIdentityByPublicKeyHashResponse =
        decode_message(response, "GetIdentityByPublicKeyHashResponse")?;
    let version = response_version(&response);
    let (identity, mtd, _) =
        <Identity as FromProof<GetIdentityByPublicKeyHashRequest>>::maybe_from_proof_with_metadata(
            request,
            response,
            network()?,
            version,
            provider(),
        )
        .map_err(|e| format!("identity-by-public-key-hash proof verification failed: {e}"))?;
    Ok((identity.as_ref().map(identity_info), meta_from(&mtd)))
}

/// getIdentityBalance. Not bridged to C++ (the client fetches balances via
/// the full identity); exercised by the vector tests to pin the shared
/// grovedb + quorum-signature pipeline against the fixture corpus.
pub fn verify_get_identity_balance(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<u64>, Meta), String> {
    let request: GetIdentityBalanceRequest = decode_message(request, "GetIdentityBalanceRequest")?;
    let response: GetIdentityBalanceResponse =
        decode_message(response, "GetIdentityBalanceResponse")?;
    let version = response_version(&response);
    let (balance, mtd, _) = IdentityBalance::maybe_from_proof_with_metadata(
        request,
        response,
        network()?,
        version,
        provider(),
    )
    .map_err(|e| format!("identity balance proof verification failed: {e}"))?;
    Ok((balance, meta_from(&mtd)))
}

/// getIdentityKeys (all keys). Not bridged to C++ (see
/// `verify_get_identity_balance`).
pub fn verify_get_identity_keys(
    request: &[u8],
    response: &[u8],
) -> Result<(Option<Vec<KeyInfo>>, Meta), String> {
    let request: GetIdentityKeysRequest = decode_message(request, "GetIdentityKeysRequest")?;
    let response: GetIdentityKeysResponse = decode_message(response, "GetIdentityKeysResponse")?;
    let version = response_version(&response);
    let (keys, mtd, _) = IdentityPublicKeys::maybe_from_proof_with_metadata(
        request,
        response,
        network()?,
        version,
        provider(),
    )
    .map_err(|e| format!("identity keys proof verification failed: {e}"))?;
    let keys = keys.map(|keys| {
        keys.values()
            .flatten()
            .map(crate::decode::key_info)
            .collect::<Vec<_>>()
    });
    Ok((keys, meta_from(&mtd)))
}

/// getDocuments: reconstructs the query from the request
/// (DocumentQuery::try_from_request), verifies the proof, and returns the
/// matched documents re-serialized in platform form (the input to the
/// decode_* functions). Absence (no matching documents) is an empty vector.
pub fn verify_get_documents(
    request: &[u8],
    response: &[u8],
) -> Result<(Vec<Vec<u8>>, Meta), String> {
    let request: GetDocumentsRequest = decode_message(request, "GetDocumentsRequest")?;
    let response: GetDocumentsResponse = decode_message(response, "GetDocumentsResponse")?;
    let version = response_version(&response);

    let (contract_id_bytes, document_type_name) = match &request.version {
        Some(GetDocumentsVersion::V0(v0)) => {
            (v0.data_contract_id.clone(), v0.document_type.clone())
        }
        Some(GetDocumentsVersion::V1(v1)) => {
            (v1.data_contract_id.clone(), v1.document_type.clone())
        }
        None => return Err("GetDocumentsRequest has no version set".to_string()),
    };
    let contract_id = crate::types::id32(&contract_id_bytes, "contract id")?;
    let contract = provider()
        .get_data_contract(&contract_id.into(), version)
        .map_err(|e| format!("unable to resolve data contract: {e}"))?
        .ok_or("document queries support only the pinned system contracts")?;

    let (documents, mtd, _) = verify_documents_response(
        request,
        std::sync::Arc::clone(&contract),
        response,
        network()?,
        version,
        provider(),
    )
    .map_err(|e| format!("document proof verification failed: {e}"))?;

    let document_type = contract
        .document_type_for_name(&document_type_name)
        .map_err(|e| format!("unknown document type {document_type_name}: {e}"))?;
    let mut serialized = Vec::new();
    for document in documents.into_iter().flatten().filter_map(|(_, doc)| doc) {
        serialized.push(
            document
                .serialize(document_type, &contract, version)
                .map_err(|e| format!("unable to re-serialize verified document: {e}"))?,
        );
    }
    Ok((serialized, meta_from(&mtd)))
}

/// getContestedResourceVoteState (VoteTally result type). The query shape —
/// contract, document type, index values, tally options, count — is
/// reconstructed from the request itself.
pub fn verify_get_contested_vote_state(
    request: &[u8],
    response: &[u8],
) -> Result<(ContestedVoteState, Meta), String> {
    let request: GetContestedResourceVoteStateRequest =
        decode_message(request, "GetContestedResourceVoteStateRequest")?;
    let response: GetContestedResourceVoteStateResponse =
        decode_message(response, "GetContestedResourceVoteStateResponse")?;
    let version = response_version(&response);
    let (contenders, mtd, _) = Contenders::maybe_from_proof_with_metadata(
        request,
        response,
        network()?,
        version,
        provider(),
    )
    .map_err(|e| format!("contested vote state proof verification failed: {e}"))?;

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
    Ok((state, meta_from(&mtd)))
}
