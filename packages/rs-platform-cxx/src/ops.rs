// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The proved reads and the broadcast, one SDK request each. Every read
//! goes through the SDK's `Fetch` / `FetchMany`, so the SDK verifies the
//! response against the query it built, then through [`accept`]: the
//! chain-id check, the shell's Platform-height watermark and the
//! unsupported-protocol-version signal, in that order, on metadata the
//! quorum signature already covers.
//!
//! Absence is proven, not inferred: `ProvenAbsent` comes back only after
//! the SDK verified a proof of it. Every failure is classified into a
//! `Status` kind on the innermost `dash_sdk::Error`, never on its text.

use std::future::Future;

use dash_sdk::dapi_client::transport::TransportError;
use dash_sdk::dapi_client::{
    CanRetry, DapiClientError, DapiRequest, DapiRequestExecutor, RequestSettings,
};
use dash_sdk::dpp::consensus::codes::ErrorWithCode;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dash_sdk::dpp::util::strings::convert_to_homograph_safe_chars;
use dash_sdk::dpp::version::v14::PROTOCOL_VERSION_14;
use dash_sdk::dpp::version::{PlatformVersion, LATEST_VERSION};
use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
use dash_sdk::platform::proto::get_documents_request::get_documents_request_v0::Start;
use dash_sdk::platform::proto::{BroadcastStateTransitionRequest, ResponseMetadata};
use dash_sdk::platform::types::identity::PublicKeyHash;
use dash_sdk::platform::{DocumentQuery, Fetch, FetchMany};
use dash_sdk::query_types::{Contenders, Documents, IdentityContractNonceFetcher};
use dash_sdk::{Error, ProofVerifierError, Sdk};

use crate::client::Client;
use crate::ffi::{self, Status, StatusKind};
use crate::provider;

/// Documents per page; Drive's query limit.
pub const PAGE_SIZE: u32 = 100;
/// Contenders requested from a contested-name vote state.
const CONTESTED_VOTE_COUNT: u16 = 100;
/// The one parent domain DPNS names live under.
const DPNS_PARENT_DOMAIN: &str = "dash";
/// `normalizedLabel.maxLength` in the DPNS contract.
const MAX_LABEL_CHARS: usize = 63;
/// Largest state transition the bridge submits; dpp refuses to deserialize
/// anything above its own 100 KiB limit, so this only bounds a caller bug.
const MAX_STATE_TRANSITION_BYTES: usize = 100 * 1024;

/// A `Verified*` result of the bridge: a status and the verified metadata
/// around a value.
pub trait Verified: Default {
    fn with_status(status: Status) -> Self {
        let mut result = Self::default();
        *result.status_mut() = status;
        result
    }
    fn status_mut(&mut self) -> &mut Status;
    fn meta_mut(&mut self) -> &mut ffi::Meta;
}

macro_rules! verified {
    ($($result:ty),*) => {
        $(impl Verified for $result {
            fn status_mut(&mut self) -> &mut Status {
                &mut self.status
            }
            fn meta_mut(&mut self) -> &mut ffi::Meta {
                &mut self.meta
            }
        })*
    };
}

verified!(
    ffi::VerifiedIdentity,
    ffi::VerifiedU64,
    ffi::VerifiedDpnsName,
    ffi::VerifiedDpnsNames,
    ffi::VerifiedProfile,
    ffi::VerifiedContactRequests,
    ffi::VerifiedContested
);

/// The result a panicking bridge read reports.
pub fn failed<R: Verified>(message: String) -> R {
    R::with_status(Status::internal(message))
}

fn meta(metadata: &ResponseMetadata) -> ffi::Meta {
    ffi::Meta {
        height: metadata.height,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time_ms: metadata.time_ms,
        protocol_version: metadata.protocol_version,
        chain_id: metadata.chain_id.clone(),
    }
}

/// Classifies an SDK error by its innermost variant.
pub fn classify(error: &Error) -> Status {
    let kind = match error {
        Error::NoAvailableAddressesToRetry(inner) => return classify(inner),
        // The provider refuses inside proof verification (wrapped in
        // `Proof`) or on a direct contract lookup; same mapping either way.
        Error::ContextProviderError(inner)
        | Error::Proof(ProofVerifierError::ContextProviderError(inner)) => {
            provider::status_kind(inner)
        }
        Error::Proof(_) | Error::DriveProofError(..) | Error::InvalidProvedResponse(_) => {
            StatusKind::Rejected
        }
        Error::StaleNode(_) => StatusKind::Rejected,
        // A definitive (non-retryable) gRPC refusal: the node answered and
        // said no, without a consensus error; nothing was retried.
        Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(status)))
            if !status.can_retry() =>
        {
            StatusKind::Rejected
        }
        Error::DapiClientError(DapiClientError::Transport(_))
        | Error::DapiClientError(DapiClientError::NoAvailableAddresses)
        | Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(_))
        | Error::TimeoutReached(..)
        | Error::Cancelled(_) => StatusKind::Unavailable,
        Error::AlreadyExists(_) => StatusKind::AlreadyExists,
        Error::Protocol(ProtocolError::ConsensusError(consensus)) => {
            return Status {
                kind: StatusKind::Consensus,
                consensus_code: consensus.code(),
                message: consensus.to_string(),
            }
        }
        Error::StateTransitionBroadcastError(broadcast) => {
            return Status {
                kind: StatusKind::Consensus,
                consensus_code: broadcast.code,
                message: broadcast.message.clone(),
            }
        }
        _ => StatusKind::Internal,
    };
    Status::new(kind, error.to_string())
}

/// The shell's post-verification checks, in order: the chain id, then the
/// height watermark (so a foreign chain never moves it), then the verified
/// protocol version the builders use, then the protocol-version signal,
/// which still returns the value.
fn accept(client: &Client, metadata: &ResponseMetadata) -> Result<Status, Status> {
    if metadata.chain_id != client.tenderdash_chain_id() {
        return Err(Status::new(
            StatusKind::ChainIdMismatch,
            format!(
                "response signed for tenderdash chain {:?}, expected {:?}",
                metadata.chain_id,
                client.tenderdash_chain_id()
            ),
        ));
    }
    if !client.observe_height(metadata.height) {
        return Err(Status::new(
            StatusKind::Rejected,
            format!(
                "stale response: platform height {} trails the highest verified height {} by \
                 more than {} blocks",
                metadata.height,
                client.last_seen_height(),
                crate::client::HEIGHT_TOLERANCE
            ),
        ));
    }
    client.observe_protocol_version(metadata.protocol_version);
    if metadata.protocol_version > LATEST_VERSION {
        return Ok(Status::new(
            StatusKind::UnsupportedProtocolVersion,
            format!(
                "the network runs protocol version {}, this build knows up to {LATEST_VERSION}",
                metadata.protocol_version
            ),
        ));
    }
    Ok(Status::ok())
}

/// Runs one metadata-returning SDK operation and applies [`accept`] to the
/// verified metadata. `fill` turns the value into the bridge result, whose
/// status and metadata are set here; on `None` (proven absence) the status
/// is `ProvenAbsent`, also under an unsupported protocol version, which
/// [`accept`] has recorded by then (the builders refuse) and which
/// `meta.protocol_version` shows; the default value could not tell absence
/// apart. A proven absence is as authenticated as a value and carries the
/// same metadata.
fn fetch<R, F, Fut, T>(client: &Client, op: F, fill: impl FnOnce(T) -> Result<R, String>) -> R
where
    R: Verified,
    F: FnOnce(Sdk) -> Fut,
    Fut: Future<Output = Result<(Option<T>, ResponseMetadata), Error>> + Send + 'static,
    T: Send + 'static,
{
    let attempt = || -> Result<R, Status> {
        client.check_ready_for_proofs()?;
        let (value, metadata) = client.run(op)?.map_err(|error| classify(&error))?;
        let status = accept(client, &metadata)?;
        let (mut result, status) = match value {
            Some(value) => (fill(value).map_err(Status::internal)?, status),
            None => (
                R::default(),
                Status::new(StatusKind::ProvenAbsent, "proven absent"),
            ),
        };
        *result.meta_mut() = meta(&metadata);
        *result.status_mut() = status;
        Ok(result)
    };
    attempt().unwrap_or_else(R::with_status)
}

// --- identities ------------------------------------------------------------

fn key_bounds(bounds: Option<&ContractBounds>) -> ffi::ContractBounds {
    match bounds {
        None => ffi::ContractBounds::default(),
        Some(ContractBounds::SingleContract { id }) => ffi::ContractBounds {
            kind: ffi::BoundsKind::SingleContract,
            contract_id: id.to_buffer(),
            document_type: String::new(),
        },
        Some(ContractBounds::SingleContractDocumentType {
            id,
            document_type_name,
        }) => ffi::ContractBounds {
            kind: ffi::BoundsKind::SingleContractDocumentType,
            contract_id: id.to_buffer(),
            document_type: document_type_name.clone(),
        },
        Some(ContractBounds::ContractGroup { id }) => ffi::ContractBounds {
            kind: ffi::BoundsKind::ContractGroup,
            contract_id: id.to_buffer(),
            document_type: String::new(),
        },
    }
}

fn identity_key(key: &IdentityPublicKey) -> ffi::IdentityKey {
    ffi::IdentityKey {
        id: key.id(),
        purpose: key.purpose() as u8,
        security_level: key.security_level() as u8,
        key_type: key.key_type() as u8,
        read_only: key.read_only(),
        data: key.data().to_vec(),
        disabled_at: key.disabled_at().unwrap_or(0),
        bounds: key_bounds(key.contract_bounds()),
    }
}

fn verified_identity(identity: Identity) -> Result<ffi::VerifiedIdentity, String> {
    Ok(ffi::VerifiedIdentity {
        value: ffi::Identity {
            id: identity.id().to_buffer(),
            balance: identity.balance(),
            revision: identity.revision(),
            keys: identity.public_keys().values().map(identity_key).collect(),
        },
        ..Default::default()
    })
}

pub fn get_identity(client: &Client, id: [u8; 32]) -> ffi::VerifiedIdentity {
    let id = Identifier::from(id);
    fetch(
        client,
        move |sdk| async move { Identity::fetch_with_metadata(&sdk, id, None).await },
        verified_identity,
    )
}

pub fn get_identity_by_pubkey_hash(client: &Client, hash: [u8; 20]) -> ffi::VerifiedIdentity {
    fetch(
        client,
        move |sdk| async move { Identity::fetch_with_metadata(&sdk, PublicKeyHash(hash), None).await },
        verified_identity,
    )
}

/// The nonce masked with `IDENTITY_NONCE_VALUE_FILTER`, as the SDK's own
/// nonce cache does: the high bits record missing revisions, not the value.
/// `ProvenAbsent` = the identity has not used the contract yet, i.e. 0.
pub fn get_identity_contract_nonce(
    client: &Client,
    id: [u8; 32],
    contract_id: [u8; 32],
) -> ffi::VerifiedU64 {
    let query = (Identifier::from(id), Identifier::from(contract_id));
    fetch(
        client,
        move |sdk| async move {
            IdentityContractNonceFetcher::fetch_with_metadata(&sdk, query, None).await
        },
        |fetcher: IdentityContractNonceFetcher| {
            Ok(ffi::VerifiedU64 {
                value: fetcher.0 & IDENTITY_NONCE_VALUE_FILTER,
                ..Default::default()
            })
        },
    )
}

// --- documents -------------------------------------------------------------

/// A document query over a compiled-in system contract, loaded at this
/// build's latest protocol version rather than the one verified responses
/// have shown the network to run. The contract only shapes the query and
/// decodes the proved documents: its index definitions are the same in
/// every version of the two contracts, the properties a newer version adds
/// are optional and trail the older ones, so a newer contract reads every
/// older document (format 2 stops at the end of the bytes; format 3 also
/// carries the version stamp), while an older contract refuses a format-3
/// document stamped with properties it does not know. The SDK seeds its
/// version at the network floor and only ratchets on a verified response,
/// so the floor's contract could not decode the first profile a newer
/// network serves. Builders take the verified version instead.
fn document_query(
    contract: SystemDataContract,
    document_type: &str,
) -> Result<DocumentQuery, Status> {
    let contract = load_system_data_contract(contract, PlatformVersion::latest())
        .map_err(|e| Status::internal(format!("unable to load the system contract: {e}")))?;
    DocumentQuery::new(contract, document_type)
        .map_err(|e| Status::internal(format!("{document_type} query: {e}")))
}

fn clause(field: &str, operator: WhereOperator, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator,
        value,
    }
}

fn equals(field: &str, value: Value) -> WhereClause {
    clause(field, WhereOperator::Equal, value)
}

fn ascending(field: &str) -> OrderClause {
    OrderClause {
        field: field.to_string(),
        ascending: true,
    }
}

/// A query continuing after the document `cursor`, if any.
fn start_after(mut query: DocumentQuery, cursor: Option<[u8; 32]>) -> DocumentQuery {
    query.start = cursor.map(|id| Start::StartAfter(id.to_vec()));
    query
}

/// The proved documents of a result, in query order; a proven absence
/// comes back as entries whose document is `None`.
fn present(documents: Documents) -> Vec<Document> {
    documents.into_values().flatten().collect()
}

/// One page of documents in query order, plus the cursor for the next page.
/// A page that fills the query's limit is reported as having more: the
/// caller stops when a page comes back short, one extra request at most.
fn page<R, D>(
    client: &Client,
    query: Result<DocumentQuery, Status>,
    decode: fn(&Document) -> Result<D, String>,
    fill: impl FnOnce(Vec<D>, ffi::Page) -> R,
) -> R
where
    R: Verified,
{
    let query = match query {
        Ok(query) => query,
        Err(status) => return R::with_status(status),
    };
    let limit = query.limit as usize;
    fetch(
        client,
        move |sdk| async move {
            Document::fetch_many_with_metadata(&sdk, query, None)
                .await
                .map(|(documents, metadata)| (Some(present(documents)), metadata))
        },
        |documents| {
            let page = ffi::Page {
                next_start_after: documents
                    .last()
                    .map(|document| document.id().to_buffer())
                    .unwrap_or_default(),
                has_more: documents.len() >= limit,
            };
            let items = documents
                .iter()
                .map(decode)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(fill(items, page))
        },
    )
}

/// The single document of a `limit(1)` query, or a proven absence.
fn single<R, D>(
    client: &Client,
    query: Result<DocumentQuery, Status>,
    decode: fn(&Document) -> Result<D, String>,
    fill: impl FnOnce(D) -> R,
) -> R
where
    R: Verified,
{
    let query = match query {
        Ok(query) => query.with_limit(1),
        Err(status) => return R::with_status(status),
    };
    fetch(
        client,
        move |sdk| async move {
            Document::fetch_many_with_metadata(&sdk, query, None)
                .await
                .map(|(documents, metadata)| (present(documents).into_iter().next(), metadata))
        },
        |document| decode(&document).map(fill),
    )
}

fn text(document: &Document, name: &str) -> String {
    document
        .properties()
        .get(name)
        .and_then(Value::as_text)
        .map(str::to_string)
        .unwrap_or_default()
}

fn bytes(document: &Document, name: &str) -> Vec<u8> {
    document
        .properties()
        .get(name)
        .and_then(|value| value.to_bytes().ok())
        .unwrap_or_default()
}

fn u32_field(document: &Document, name: &str) -> Result<u32, String> {
    document
        .properties()
        .get(name)
        .map(|value| {
            value
                .to_integer::<u32>()
                .map_err(|e| format!("property {name} is not a u32: {e}"))
        })
        .unwrap_or(Ok(0))
}

fn dpns_name(document: &Document) -> Result<ffi::DpnsName, String> {
    let identity = document
        .get("records.identity")
        .ok_or("the domain document has no records.identity")?
        .to_identifier()
        .map_err(|e| format!("records.identity: {e}"))?;
    Ok(ffi::DpnsName {
        label: text(document, "label"),
        normalized_label: text(document, "normalizedLabel"),
        parent: text(document, "normalizedParentDomainName"),
        identity: identity.to_buffer(),
        document_id: document.id().to_buffer(),
        owner: document.owner_id().to_buffer(),
    })
}

fn profile(document: &Document) -> Result<ffi::Profile, String> {
    Ok(ffi::Profile {
        document_id: document.id().to_buffer(),
        owner: document.owner_id().to_buffer(),
        revision: document.revision().unwrap_or(0),
        display_name: text(document, "displayName"),
        public_message: text(document, "publicMessage"),
        avatar_url: text(document, "avatarUrl"),
        avatar_hash: bytes(document, "avatarHash"),
        avatar_fingerprint: bytes(document, "avatarFingerprint"),
        core_payment_address: bytes(document, "corePaymentAddress"),
        platform_payment_address: bytes(document, "platformPaymentAddress"),
        shielded_address: bytes(document, "shieldedAddress"),
        created_at: document.created_at().unwrap_or(0),
        updated_at: document.updated_at().unwrap_or(0),
    })
}

fn contact_request(document: &Document) -> Result<ffi::ContactRequest, String> {
    let to_user_id = document
        .properties()
        .get("toUserId")
        .ok_or("the contact request has no toUserId")?
        .to_identifier()
        .map_err(|e| format!("toUserId: {e}"))?;
    Ok(ffi::ContactRequest {
        document_id: document.id().to_buffer(),
        owner: document.owner_id().to_buffer(),
        to_user_id: to_user_id.to_buffer(),
        encrypted_public_key: bytes(document, "encryptedPublicKey"),
        sender_key_index: u32_field(document, "senderKeyIndex")?,
        recipient_key_index: u32_field(document, "recipientKeyIndex")?,
        account_reference: u32_field(document, "accountReference")?,
        encrypted_account_label: bytes(document, "encryptedAccountLabel"),
        auto_accept_proof: bytes(document, "autoAcceptProof"),
        created_at: document.created_at().unwrap_or(0),
        core_height_created_at: document.created_at_core_block_height().unwrap_or(0),
    })
}

/// The DPNS `domain` query under the "dash" parent, i.e. the
/// (normalizedParentDomainName, normalizedLabel) unique index.
fn dash_tld_query() -> Result<DocumentQuery, Status> {
    Ok(
        document_query(SystemDataContract::DPNS, "domain")?.with_where(equals(
            "normalizedParentDomainName",
            Value::Text(DPNS_PARENT_DOMAIN.to_string()),
        )),
    )
}

fn names(client: &Client, query: Result<DocumentQuery, Status>) -> ffi::VerifiedDpnsNames {
    page(client, query, dpns_name, |items, page| {
        ffi::VerifiedDpnsNames {
            items,
            page,
            ..Default::default()
        }
    })
}

/// Resolves a normalized label under "dash"; `ProvenAbsent` = unregistered.
pub fn resolve_name(client: &Client, normalized_label: &str) -> ffi::VerifiedDpnsName {
    let query = dash_tld_query().map(|query| {
        query.with_where(equals(
            "normalizedLabel",
            Value::Text(convert_to_homograph_safe_chars(normalized_label)),
        ))
    });
    single(client, query, dpns_name, |value| ffi::VerifiedDpnsName {
        value,
        ..Default::default()
    })
}

/// One page of the names whose normalized label starts with `prefix`,
/// ascending, `limit` clamped to one page. An empty prefix is a query Drive
/// refuses and one longer than a label can be matches nothing: both are
/// refused here, before anything is sent.
pub fn search_names(
    client: &Client,
    prefix: &str,
    limit: u32,
    cursor: Option<[u8; 32]>,
) -> ffi::VerifiedDpnsNames {
    let prefix = convert_to_homograph_safe_chars(prefix);
    if prefix.is_empty() || prefix.chars().count() > MAX_LABEL_CHARS {
        return ffi::VerifiedDpnsNames::with_status(Status::internal(format!(
            "a name search needs a prefix of 1 to {MAX_LABEL_CHARS} characters"
        )));
    }
    let query = dash_tld_query().map(|query| {
        start_after(query, cursor)
            .with_where(clause(
                "normalizedLabel",
                WhereOperator::StartsWith,
                Value::Text(prefix),
            ))
            .with_order_by(ascending("normalizedLabel"))
            .with_limit(limit.clamp(1, PAGE_SIZE))
    });
    names(client, query)
}

/// One page of the names whose `records.identity` is `identity`. The
/// contract's `identityId` index has that one property, so Drive orders
/// the entries under it by document id, which is the order the cursor
/// continues. Up to protocol version 13 Drive answers a continuation on
/// this index with a proven empty page (it skips every remaining entry
/// under the identity), which would read as the end of the list: a
/// continuation answered there is `Unavailable` instead, so an identity
/// with more than one page of names is not silently cut off.
pub fn names_of_identity(
    client: &Client,
    identity: [u8; 32],
    cursor: Option<[u8; 32]>,
) -> ffi::VerifiedDpnsNames {
    let query = document_query(SystemDataContract::DPNS, "domain").map(|query| {
        start_after(query, cursor)
            .with_where(equals("records.identity", Value::Identifier(identity)))
            .with_limit(PAGE_SIZE)
    });
    let mut result = names(client, query);
    if cursor.is_some()
        && result.status.kind == StatusKind::Ok
        && result.meta.protocol_version < PROTOCOL_VERSION_14
    {
        result.status = Status::unavailable(format!(
            "protocol version {} cannot continue a names_of_identity page (fixed in {})",
            result.meta.protocol_version, PROTOCOL_VERSION_14
        ));
    }
    result
}

/// The DashPay profile owned by `owner`; `ProvenAbsent` = none.
pub fn get_profile(client: &Client, owner: [u8; 32]) -> ffi::VerifiedProfile {
    let query = document_query(SystemDataContract::Dashpay, "profile")
        .map(|query| query.with_where(equals("$ownerId", Value::Identifier(owner))));
    single(client, query, profile, |value| ffi::VerifiedProfile {
        value,
        ..Default::default()
    })
}

/// One page of the contact requests sent to (`to_me`) or by `identity`,
/// created after `since_ms`, oldest first. The `$createdAt` order pins the
/// contract's `(field, $createdAt)` index, which a bare equality would not.
pub fn get_contact_requests(
    client: &Client,
    identity: [u8; 32],
    to_me: bool,
    since_ms: u64,
    cursor: Option<[u8; 32]>,
) -> ffi::VerifiedContactRequests {
    let query = document_query(SystemDataContract::Dashpay, "contactRequest").map(|query| {
        let mut query = start_after(query, cursor).with_where(equals(
            if to_me { "toUserId" } else { "$ownerId" },
            Value::Identifier(identity),
        ));
        if since_ms > 0 {
            query = query.with_where(clause(
                "$createdAt",
                WhereOperator::GreaterThan,
                Value::U64(since_ms),
            ));
        }
        query
            .with_order_by(ascending("$createdAt"))
            .with_limit(PAGE_SIZE)
    });
    page(client, query, contact_request, |items, page| {
        ffi::VerifiedContactRequests {
            items,
            page,
            ..Default::default()
        }
    })
}

// --- contested names -------------------------------------------------------

/// A finished or empty contest is a `Contenders` with no contenders; the
/// SDK folds a proven-absent contest into the same shape, so absence is
/// read off the tallies: a real contest always carries them.
fn contested_state(contenders: Contenders) -> Option<ffi::ContestedState> {
    if contenders.contenders.is_empty()
        && contenders.winner.is_none()
        && contenders.abstain_vote_tally.is_none()
        && contenders.lock_vote_tally.is_none()
    {
        return None;
    }
    let mut state = ffi::ContestedState {
        contenders: contenders
            .contenders
            .iter()
            .map(|(id, contender)| ffi::Contender {
                identity: id.to_buffer(),
                votes: contender.vote_tally().unwrap_or(0),
                has_votes: contender.vote_tally().is_some(),
            })
            .collect(),
        abstain: contenders.abstain_vote_tally.unwrap_or(0),
        lock: contenders.lock_vote_tally.unwrap_or(0),
        ..Default::default()
    };
    if let Some((winner, finalization_block)) = contenders.winner {
        state.ends_at = finalization_block.time_ms;
        match winner {
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id) => {
                state.winner_kind = ffi::WinnerKind::WonByIdentity;
                state.winner = id.to_buffer();
            }
            ContestedDocumentVotePollWinnerInfo::Locked => {
                state.winner_kind = ffi::WinnerKind::Locked;
            }
            ContestedDocumentVotePollWinnerInfo::NoWinner => {}
        }
    }
    Some(state)
}

/// The vote state of the contested name `normalized_label` (tallies,
/// including abstain and lock); `ProvenAbsent` = no contest.
pub fn get_contested_vote_state(client: &Client, normalized_label: &str) -> ffi::VerifiedContested {
    let query = ContestedDocumentVotePollDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id: SystemDataContract::DPNS.id(),
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
    fetch(
        client,
        move |sdk| async move {
            ContenderWithSerializedDocument::fetch_many_with_metadata(&sdk, query, None)
                .await
                .map(|(contenders, metadata)| (contested_state(contenders), metadata))
        },
        |value| {
            Ok(ffi::VerifiedContested {
                value,
                ..Default::default()
            })
        },
    )
}

// --- broadcast -------------------------------------------------------------

/// The typed outcome of one broadcast attempt. The SDK's error conversion
/// decodes the consensus error DAPI attaches as gRPC metadata and the
/// `AlreadyExists` code; [`classify`] does the rest.
pub fn broadcast_status(result: Result<(), DapiClientError>) -> Status {
    match result {
        Ok(()) => Status::ok(),
        Err(error) => classify(&Error::from(error)),
    }
}

/// Submits `request` through `executor` (the SDK, or a mock transport in
/// tests). A node that rejects the transition is not a failing node, so
/// banning is off.
pub async fn submit<E: DapiRequestExecutor + Sync>(
    executor: &E,
    request: BroadcastStateTransitionRequest,
) -> Status {
    let settings = RequestSettings {
        ban_failed_address: Some(false),
        ..RequestSettings::default()
    };
    broadcast_status(
        request
            .execute(executor, settings)
            .await
            .map(|_| ())
            .map_err(|execution| execution.inner),
    )
}

/// Submits a signed state transition as a raw DAPI request (the SDK's
/// broadcast helper needs the deserialized transition to refresh nonces,
/// which the embedder owns).
pub fn broadcast(client: &Client, state_transition: &[u8]) -> ffi::BroadcastResult {
    if state_transition.len() > MAX_STATE_TRANSITION_BYTES {
        return ffi::BroadcastResult {
            status: Status::internal(format!(
                "state transition is {} bytes, above the {MAX_STATE_TRANSITION_BYTES}-byte limit",
                state_transition.len()
            )),
        };
    }
    let request = BroadcastStateTransitionRequest {
        state_transition: state_transition.to_vec(),
    };
    let status = match client.run(move |sdk| async move { submit(&sdk, request).await }) {
        Ok(status) | Err(status) => status,
    };
    ffi::BroadcastResult { status }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::block::block_info::BlockInfo;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dash_sdk::dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dash_sdk::dpp::platform_value::platform_value;
    use dash_sdk::dpp::version::PlatformVersion;

    /// The `document_query` premise: a document written under an older
    /// protocol version and contract (PV13, DashPay v1, document format 2)
    /// decodes with the latest compiled-in contract (DashPay v2 under PV14),
    /// so a query built on the latest contract reads every network.
    #[test]
    fn older_documents_decode_with_the_latest_contract() {
        let older = PlatformVersion::get(13).expect("PV13");
        let latest = PlatformVersion::latest();
        assert!(older.protocol_version < latest.protocol_version);
        let older_contract = load_system_data_contract(SystemDataContract::Dashpay, older).unwrap();
        let latest_contract =
            load_system_data_contract(SystemDataContract::Dashpay, latest).unwrap();
        assert!(older_contract.version() < latest_contract.version());
        let older_type = older_contract.document_type_for_name("profile").unwrap();
        let document = older_type
            .create_document_from_data(
                platform_value!({
                    "displayName": "Alice",
                    "publicMessage": "hello",
                    "$createdAt": 1_700_000_000_000u64,
                    "$updatedAt": 1_700_000_000_000u64,
                }),
                Identifier::from([7u8; 32]),
                1,
                1,
                [9u8; 32],
                older,
            )
            .unwrap();
        let bytes = document
            .serialize(older_type, &older_contract, older)
            .unwrap();
        let latest_type = latest_contract.document_type_for_name("profile").unwrap();
        let decoded = Document::from_bytes(&bytes, latest_type, latest).unwrap();
        let profile = profile(&decoded).unwrap();
        assert_eq!(profile.display_name, "Alice");
        assert_eq!(profile.public_message, "hello");
        assert_eq!(profile.revision, 1);
    }

    #[test]
    fn empty_contenders_read_as_no_contest() {
        assert!(contested_state(Contenders::default()).is_none());
    }

    #[test]
    fn a_finished_poll_without_contenders_is_still_a_contest() {
        let state = contested_state(Contenders {
            winner: Some((
                ContestedDocumentVotePollWinnerInfo::Locked,
                BlockInfo {
                    time_ms: 77,
                    ..Default::default()
                },
            )),
            contenders: Default::default(),
            abstain_vote_tally: Some(0),
            lock_vote_tally: Some(3),
        })
        .expect("a contest");
        assert_eq!(state.winner_kind, ffi::WinnerKind::Locked);
        assert_eq!((state.lock, state.ends_at), (3, 77));
    }

    #[test]
    fn zero_tallies_are_a_contest_not_absence() {
        assert!(contested_state(Contenders {
            winner: None,
            contenders: Default::default(),
            abstain_vote_tally: Some(0),
            lock_vote_tally: Some(0),
        })
        .is_some());
    }

    #[test]
    fn classification_unwraps_the_innermost_error() {
        let inner = Error::DapiClientError(DapiClientError::NoAvailableAddresses);
        assert_eq!(classify(&inner).kind, StatusKind::Unavailable);
        let wrapped = Error::NoAvailableAddressesToRetry(Box::new(Error::Proof(
            dash_sdk::ProofVerifierError::NoProofInResult,
        )));
        assert_eq!(classify(&wrapped).kind, StatusKind::Rejected);
        assert_eq!(
            classify(&Error::AlreadyExists("x".to_string())).kind,
            StatusKind::AlreadyExists
        );
        assert_eq!(
            classify(&Error::ContextProviderError(
                dash_sdk::error::ContextProviderError::Config("no anchor".to_string())
            ))
            .kind,
            StatusKind::Unavailable
        );
        assert_eq!(
            classify(&Error::ContextProviderError(
                dash_sdk::error::ContextProviderError::InvalidQuorum("stale".to_string())
            ))
            .kind,
            StatusKind::Rejected
        );
        assert_eq!(
            classify(&Error::Generic("bug".to_string())).kind,
            StatusKind::Internal
        );
    }

    /// A node that answers with a definitive refusal is not an outage.
    #[test]
    fn a_definitive_grpc_refusal_is_a_rejection() {
        use dash_sdk::dapi_grpc::tonic::{Code, Status as GrpcStatus};
        let grpc = |code| {
            Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                GrpcStatus::new(code, "no"),
            )))
        };
        assert_eq!(
            classify(&grpc(Code::InvalidArgument)).kind,
            StatusKind::Rejected
        );
        assert_eq!(classify(&grpc(Code::OutOfRange)).kind, StatusKind::Rejected);
        assert_eq!(
            classify(&grpc(Code::Unavailable)).kind,
            StatusKind::Unavailable
        );
        assert_eq!(
            classify(&grpc(Code::DeadlineExceeded)).kind,
            StatusKind::Unavailable
        );
    }
}
