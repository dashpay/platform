// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The proved queries the embedder issues, expressed through `dash-sdk`'s
//! `Fetch` / `FetchMany` and the shared DPNS / DashPay query shapes. Each
//! function runs one SDK operation on [`Client`], then applies the
//! embedder's freshness policy to the verified metadata and flattens the
//! result into the plain types the bridge exposes.
//!
//! Absence is proven, not inferred: an `Ok(None)` / empty vector comes back
//! only after the SDK verified a proof of it. Transport failures, unverifiable
//! responses and stale metadata are errors.

use std::sync::Arc;

use dash_sdk::dapi_client::transport::TransportError;
use dash_sdk::dapi_client::{CanRetry, DapiClientError, DapiRequest, RequestSettings};
use dash_sdk::dpp::consensus::codes::ErrorWithCode;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dash_sdk::dpp::util::strings::convert_to_homograph_safe_chars;
use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo as WinnerInfo;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::platform::types::identity::PublicKeyHash;
use dash_sdk::platform::{Document, DocumentQuery, Fetch, FetchMany, Identity};
use dash_sdk::query_types::{Contenders, IdentityContractNonceFetcher, IdentityNonceFetcher};
use dash_sdk::Sdk;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive::query::{OrderClause, WhereClause, WhereOperator};

use crate::client::Client;
use crate::decode::{self, identity_info};
use crate::types::{
    fixed, id32, BroadcastRejection, ContactRequest, ContestedVoteState, DpnsName, IdentityInfo,
    Meta, Profile,
};

/// Upper bound on documents returned by list queries; mirrors DAPI's
/// default query limit so the locally reconstructed query matches the
/// prover's.
const DOCUMENT_LIMIT: u32 = 100;
/// Contenders requested from a contested-name vote state.
const CONTESTED_VOTE_COUNT: u16 = 100;
/// The one parent domain DPNS names live under.
const DPNS_PARENT_DOMAIN: &str = "dash";

fn system_contract(
    client: &Client,
    contract: SystemDataContract,
) -> Result<Arc<dash_sdk::platform::DataContract>, String> {
    load_system_data_contract(contract, client.platform_version()?)
        .map(Arc::new)
        .map_err(|e| format!("unable to load system data contract: {e}"))
}

// --- identities ----------------------------------------------------------

pub fn get_identity(client: &Client, id: &[u8]) -> Result<(Option<IdentityInfo>, Meta), String> {
    let id = Identifier::from(id32(id, "identity id")?);
    let (identity, meta) = client.fetch(move |sdk: Sdk| async move {
        Identity::fetch_with_metadata(&sdk, id, None).await
    })?;
    Ok((identity.as_ref().map(identity_info), meta))
}

pub fn get_identity_by_pubkey_hash(
    client: &Client,
    hash: &[u8],
) -> Result<(Option<IdentityInfo>, Meta), String> {
    let hash: [u8; 20] = fixed(hash, "public key hash")?;
    let (identity, meta) = client.fetch(move |sdk: Sdk| async move {
        Identity::fetch_with_metadata(&sdk, PublicKeyHash(hash), None).await
    })?;
    Ok((identity.as_ref().map(identity_info), meta))
}

/// Proved identity nonce; `None` = proven absent (the identity has not
/// submitted a transition yet).
pub fn get_identity_nonce(client: &Client, id: &[u8]) -> Result<(Option<u64>, Meta), String> {
    let id = Identifier::from(id32(id, "identity id")?);
    let (nonce, meta) = client.fetch(move |sdk: Sdk| async move {
        IdentityNonceFetcher::fetch_with_metadata(&sdk, id, None).await
    })?;
    Ok((nonce.map(|fetcher| fetcher.0), meta))
}

pub fn get_identity_contract_nonce(
    client: &Client,
    id: &[u8],
    contract_id: &[u8],
) -> Result<(Option<u64>, Meta), String> {
    let id = Identifier::from(id32(id, "identity id")?);
    let contract_id = Identifier::from(id32(contract_id, "contract id")?);
    let (nonce, meta) = client.fetch(move |sdk: Sdk| async move {
        IdentityContractNonceFetcher::fetch_with_metadata(&sdk, (id, contract_id), None).await
    })?;
    Ok((nonce.map(|fetcher| fetcher.0), meta))
}

// --- documents -----------------------------------------------------------

fn fetch_documents(client: &Client, query: DocumentQuery) -> Result<(Vec<Document>, Meta), String> {
    let (documents, meta) = client.fetch(move |sdk: Sdk| async move {
        Document::fetch_many_with_metadata(&sdk, query, None).await
    })?;
    // A proven absence comes back as entries whose document is `None`.
    let documents = documents
        .into_iter()
        .filter_map(|(_, document)| document)
        .collect();
    Ok((documents, meta))
}

fn document_query(
    client: &Client,
    contract: SystemDataContract,
    document_type: &str,
) -> Result<DocumentQuery, String> {
    DocumentQuery::new(system_contract(client, contract)?, document_type)
        .map_err(|e| format!("{document_type} query: {e}"))
}

/// DPNS domain query scoped to the "dash" parent domain, i.e. the
/// (normalizedParentDomainName, normalizedLabel) index. `names_of_identity`
/// must not use it: that one goes through the records.identity index.
fn dash_tld_query(client: &Client) -> Result<DocumentQuery, String> {
    Ok(
        document_query(client, SystemDataContract::DPNS, "domain")?.with_where(equals(
            "normalizedParentDomainName",
            Value::Text(DPNS_PARENT_DOMAIN.to_string()),
        )),
    )
}

// The SDK's own resolve/search/names-of-identity helpers build the same
// queries but return no ResponseMetadata, which `Client::fetch`'s chain-id
// and ChainLock checks need; hence the local shapes.
fn fetch_names(client: &Client, query: DocumentQuery) -> Result<(Vec<DpnsName>, Meta), String> {
    let (documents, meta) = fetch_documents(client, query)?;
    let names = documents
        .iter()
        .map(decode::dpns_domain)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((names, meta))
}

fn equals(field: &str, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::Equal,
        value,
    }
}

/// Resolves a normalized DPNS label under the "dash" parent; `None` =
/// proven unregistered.
pub fn resolve_name(
    client: &Client,
    normalized_label: &str,
) -> Result<(Option<DpnsName>, Meta), String> {
    let query = dash_tld_query(client)?
        .with_where(equals(
            "normalizedLabel",
            Value::Text(convert_to_homograph_safe_chars(normalized_label)),
        ))
        .with_limit(1);
    let (names, meta) = fetch_names(client, query)?;
    Ok((names.into_iter().next(), meta))
}

/// DPNS names whose normalized label starts with `prefix`, ascending.
pub fn search_names(
    client: &Client,
    prefix: &str,
    limit: u32,
) -> Result<(Vec<DpnsName>, Meta), String> {
    let query = dash_tld_query(client)?
        .with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::StartsWith,
            value: Value::Text(convert_to_homograph_safe_chars(prefix)),
        })
        .with_order_by(OrderClause {
            field: "normalizedLabel".to_string(),
            ascending: true,
        })
        .with_limit(limit.clamp(1, DOCUMENT_LIMIT));
    fetch_names(client, query)
}

/// DPNS names whose `records.identity` points at `identity`.
pub fn names_of_identity(
    client: &Client,
    identity: &[u8],
) -> Result<(Vec<DpnsName>, Meta), String> {
    let identity = id32(identity, "identity id")?;
    let query = document_query(client, SystemDataContract::DPNS, "domain")?
        .with_where(equals("records.identity", Value::Identifier(identity)))
        .with_limit(DOCUMENT_LIMIT);
    fetch_names(client, query)
}

/// The DashPay profile owned by `owner`; `None` = proven absent.
pub fn get_profile(client: &Client, owner: &[u8]) -> Result<(Option<Profile>, Meta), String> {
    let owner = id32(owner, "owner id")?;
    let query = document_query(client, SystemDataContract::Dashpay, "profile")?
        .with_where(equals("$ownerId", Value::Identifier(owner)))
        .with_limit(1);
    let (documents, meta) = fetch_documents(client, query)?;
    let profile = documents.first().map(decode::dashpay_profile).transpose()?;
    Ok((profile, meta))
}

/// Contact requests sent to (`to_me`) or by `identity`, oldest first.
pub fn get_contact_requests(
    client: &Client,
    identity: &[u8],
    to_me: bool,
) -> Result<(Vec<ContactRequest>, Meta), String> {
    let identity = id32(identity, "identity id")?;
    let query = document_query(client, SystemDataContract::Dashpay, "contactRequest")?
        .with_where(equals(
            if to_me { "toUserId" } else { "$ownerId" },
            Value::Identifier(identity),
        ))
        // A bare secondary-index equality without an order-by is proven
        // absent by Drive; ordering by $createdAt pins the contract's
        // (field, $createdAt) index.
        .with_order_by(OrderClause {
            field: "$createdAt".to_string(),
            ascending: true,
        })
        .with_limit(DOCUMENT_LIMIT);
    let (documents, meta) = fetch_documents(client, query)?;
    let requests = documents
        .iter()
        .map(decode::contact_request)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((requests, meta))
}

// --- contested names -----------------------------------------------------

/// Vote state of the contested DPNS name `normalized_label` (VoteTally with
/// locked and abstaining tallies). `contest_found == false` means the
/// contest was proven absent.
pub fn get_contested_vote_state(
    client: &Client,
    normalized_label: &str,
) -> Result<(ContestedVoteState, Meta), String> {
    let contract = system_contract(client, SystemDataContract::DPNS)?;
    let query = ContestedDocumentVotePollDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(convert_to_homograph_safe_chars(normalized_label)),
            ],
        },
        result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
        allow_include_locked_and_abstaining_vote_tally: true,
        start_at: None,
        limit: Some(CONTESTED_VOTE_COUNT),
        offset: None,
    };
    let (contenders, meta): (Contenders, _) = client.fetch(move |sdk: Sdk| async move {
        ContenderWithSerializedDocument::fetch_many_with_metadata(&sdk, query, None).await
    })?;
    Ok((contested_state(contenders), meta))
}

/// `FetchMany` folds a proven-absent contest into `Contenders::default()`;
/// a real contest always carries at least its tallies, so an all-empty value
/// reads as absent.
fn contested_state(contenders: Contenders) -> ContestedVoteState {
    let mut state = ContestedVoteState::default();
    let absent = contenders.contenders.is_empty()
        && contenders.winner.is_none()
        && contenders.abstain_vote_tally.is_none()
        && contenders.lock_vote_tally.is_none();
    if absent {
        return state;
    }
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
    state
}

// --- broadcast -----------------------------------------------------------

/// Largest state transition the bridge will submit; dpp refuses to
/// deserialize anything above its own 100 KiB limit, so this only bounds a
/// caller bug.
const MAX_STATE_TRANSITION_BYTES: usize = 100 * 1024;

/// Submits a signed state transition. `Ok(Ok(()))` means a node accepted it
/// into its mempool; `Ok(Err(rejection))` carries the node's (unproven)
/// rejection code and message; `Err` is a transport failure. Success is
/// confirmed by the embedder through a proved re-query.
pub fn broadcast_state_transition(
    client: &Client,
    state_transition: &[u8],
) -> Result<Result<(), BroadcastRejection>, String> {
    if state_transition.len() > MAX_STATE_TRANSITION_BYTES {
        return Err(format!(
            "state transition is {} bytes, above the {MAX_STATE_TRANSITION_BYTES}-byte limit",
            state_transition.len()
        ));
    }
    let request = dash_sdk::platform::proto::BroadcastStateTransitionRequest {
        state_transition: state_transition.to_vec(),
    };
    // Executed as a raw DAPI request rather than through `dash-sdk`'s
    // broadcast helper: that helper needs the deserialized transition (to
    // refresh nonces on failure), while the embedder built and signed its
    // bytes already. A node that rejects the transition is not a failing
    // node, so address banning is off for this request; the SDK's error
    // conversion decodes the consensus error DAPI attaches as gRPC metadata,
    // and that typed error, not the gRPC status bucket, is the rejection.
    let settings = RequestSettings {
        ban_failed_address: Some(false),
        ..RequestSettings::default()
    };
    client.run(move |sdk: Sdk| async move {
        match request.execute(&sdk, settings).await {
            Ok(_) => Ok(Ok(())),
            Err(execution) => match execution.inner {
                DapiClientError::Transport(TransportError::Grpc(status)) => {
                    let grpc_code = status.code() as u32;
                    let retryable = status.can_retry();
                    let error = dash_sdk::Error::from(DapiClientError::Transport(
                        TransportError::Grpc(status),
                    ));
                    match error {
                        dash_sdk::Error::Protocol(ProtocolError::ConsensusError(consensus)) => {
                            Ok(Err(BroadcastRejection {
                                code: consensus.code(),
                                message: consensus.to_string(),
                            }))
                        }
                        other if !retryable => Ok(Err(BroadcastRejection {
                            code: grpc_code,
                            message: other.to_string(),
                        })),
                        other => Err(other),
                    }
                }
                other => Err(dash_sdk::Error::from(other)),
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::block::block_info::BlockInfo;

    #[test]
    fn homograph_normalization_is_idempotent() {
        // Every query normalizes its label once more; a caller that already
        // normalized must get the same index value.
        for label in ["Alice", "a11ce", "bOb-LoL", "x0y1z"] {
            let once = convert_to_homograph_safe_chars(label);
            assert_eq!(convert_to_homograph_safe_chars(&once), once);
        }
    }

    #[test]
    fn empty_contenders_read_as_no_contest() {
        assert!(!contested_state(Contenders::default()).contest_found);
    }

    #[test]
    fn a_finished_poll_with_no_contenders_is_still_a_contest() {
        let contenders = Contenders {
            winner: Some((WinnerInfo::Locked, BlockInfo::default())),
            contenders: Default::default(),
            abstain_vote_tally: Some(0),
            lock_vote_tally: Some(3),
        };
        let state = contested_state(contenders);
        assert!(state.contest_found);
        assert!(state.finished && state.locked);
        assert_eq!(state.lock_votes, Some(3));
    }

    #[test]
    fn zero_tallies_are_a_contest_not_absence() {
        let contenders = Contenders {
            winner: None,
            contenders: Default::default(),
            abstain_vote_tally: Some(0),
            lock_vote_tally: Some(0),
        };
        assert!(contested_state(contenders).contest_found);
    }
}
