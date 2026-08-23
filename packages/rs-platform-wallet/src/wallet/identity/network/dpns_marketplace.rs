//! DPNS username marketplace: wallet-level search / sell / delist /
//! purchase / transfer orchestration, per-name trade history, and the
//! local name-state bookkeeping behind them.
//!
//! The generic document-trade transitions live in `document.rs`
//! (`set_document_price_with_signer` / `purchase_document_with_signer` /
//! `transfer_document_with_signer`); this module composes them with the
//! DPNS specifics the app layer should not own:
//!
//! - resolving a name to its `domain` document (with `$price` and the
//!   document id, which the SDK's `DpnsUsername` drops),
//! - automatic signing-key selection (AUTHENTICATION / ECDSA at the
//!   document type's required security level — no hardcoded key ids),
//! - typed pre-flight checks (not-found / contested / not-for-sale /
//!   price-changed / insufficient credits),
//! - local persistence of sale state through the changeset pipeline
//!   ([`DpnsNameStateEntry`] rows + the legacy `dpns_names` label list),
//! - the trade-history timeline from the Document History system
//!   contract.
//!
//! Consensus facts this module relies on (verified against rs-drive):
//! purchase and transfer both REMOVE `$price`
//! (transfer-to-self is therefore the delist primitive); purchase
//! requires the transition price to equal the listed price;
//! `records.identity` is rewritten to the new owner by the protocol on
//! purchase/transfer; a name inside an active contested-name vote is not
//! in the documents tree at all.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use dpp::document::property_names::PRICE;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};

use dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dash_sdk::drive::query::{OrderClause, SelectProjection, WhereClause, WhereOperator};
use dash_sdk::platform::dpns_usernames::{convert_to_homograph_safe_chars, is_contested_username};
use dash_sdk::platform::{DocumentQuery, FetchMany};

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::changeset::{
    DpnsNameSaleStatus, DpnsNameStateChangeSet, DpnsNameStateEntry, PersistenceError,
};
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::key_storage::DpnsNameInfo;

use super::document::allowed_signing_security_levels;
use super::*;

/// DPNS document type carrying registered names.
const DPNS_DOCUMENT_TYPE: &str = "domain";
/// The only DPNS parent domain in production.
const DPNS_PARENT_DOMAIN: &str = "dash";

/// Document History system contract document types (see
/// `packages/document-history-contract/schema/v1/...`). All three carry
/// `dataContractId` / `documentId` and a `byDocument`
/// (dataContractId, documentId, $createdAt) index.
const HISTORY_TYPE_TRANSFER: &str = "transfer";
const HISTORY_TYPE_PURCHASE: &str = "purchase";
const HISTORY_TYPE_PRICE_UPDATE: &str = "priceUpdate";

/// Conservative fee reserve (credits) required ON TOP of the purchase
/// price before a purchase is attempted: Platform deducts the purchase
/// amount as principal first and the processing fee must fit in the
/// remainder (`validate_fees_of_event`). The observed document-batch
/// transition fee is well under 0.0005 DASH; 0.001 DASH (1 duff = 1000
/// credits) keeps a ~2x margin. The actual fee is metered at execution
/// from the buyer identity's credits; this constant only gates the
/// pre-flight, it is never broadcast.
pub const DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS: Credits = 100_000_000;

/// Default page size for marketplace search queries.
const DEFAULT_SEARCH_LIMIT: u32 = 25;
/// Page size for the per-identity domain-document sync query.
const SYNC_QUERY_LIMIT: u32 = 100;
/// Maximum number of departed names whose history is resolved in one
/// sync pass. Combined with [`SYNC_QUERY_LIMIT`], this keeps every pass
/// bounded even when an identity has accumulated a large name set.
const SYNC_DEPARTURE_LIMIT: usize = 25;
/// Page size for per-name history queries (per event type).
const HISTORY_QUERY_LIMIT: u32 = 100;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A DPNS `domain` document read straight off Platform, keeping the
/// marketplace-relevant system fields the SDK's `DpnsUsername` drops:
/// the document id (the handle every trade transition needs) and
/// `$price` (the sale state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsDomainState {
    /// The domain document id — stable across transfers and purchases.
    pub document_id: Identifier,
    /// Display label (e.g. "Alice").
    pub label: String,
    /// Homograph-normalized label (e.g. "a11ce").
    pub normalized_label: String,
    /// Normalized parent domain ("dash").
    pub normalized_parent_domain_name: String,
    /// The document's `$ownerId` — the identity that owns (and may sell)
    /// the name.
    pub owner_id: Identifier,
    /// `records.identity` — the identity the name points at. The
    /// protocol rewrites this to the new owner on purchase/transfer.
    pub records_identity_id: Option<Identifier>,
    /// Listed sale price in credits (`$price`). `None` = not for sale.
    pub price: Option<Credits>,
    /// Document `$createdAt` in ms, when carried.
    pub created_at_ms: Option<u64>,
    /// Document `$updatedAt` in ms — bumps on price changes.
    pub updated_at_ms: Option<u64>,
    /// Document `$transferredAt` in ms — set on purchase/transfer.
    pub transferred_at_ms: Option<u64>,
}

impl DpnsDomainState {
    /// Read the marketplace-relevant fields off a DPNS `domain` document.
    ///
    /// Errors (rather than fabricating defaults) when required schema
    /// fields are missing or mistyped — a malformed `$price` must not
    /// silently read as "not for sale".
    fn from_document(doc: &Document) -> Result<Self, PlatformWalletError> {
        let properties = doc.properties();
        let text = |key: &str| -> Result<String, PlatformWalletError> {
            properties
                .get(key)
                .and_then(|v| v.as_text())
                .map(str::to_string)
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "DPNS domain document {} is missing required text field {key:?}",
                        doc.id()
                    ))
                })
        };
        let price = properties
            .get_optional_integer::<Credits>(PRICE)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "DPNS domain document {} carries a malformed $price: {e}",
                    doc.id()
                ))
            })?;
        // `records.identity` — same manual map walk as the SDK's
        // `document_to_dpns_username` (the value is an identifier).
        let records_identity_id = if let Some(Value::Map(records)) = properties.get("records") {
            records
                .iter()
                .find(|(k, _)| k.as_text() == Some("identity"))
                .and_then(|(_, v)| v.to_identifier().ok())
        } else {
            None
        };
        Ok(Self {
            document_id: doc.id(),
            label: text("label")?,
            normalized_label: text("normalizedLabel")?,
            normalized_parent_domain_name: text("normalizedParentDomainName")?,
            owner_id: doc.owner_id(),
            records_identity_id,
            price,
            created_at_ms: doc.created_at(),
            updated_at_ms: doc.updated_at(),
            transferred_at_ms: doc.transferred_at(),
        })
    }

    /// Build the local persisted row for this state, tracked for
    /// `wallet_identity_id` with `status`.
    fn to_entry(
        &self,
        wallet_identity_id: Identifier,
        status: DpnsNameSaleStatus,
        now_ms: u64,
    ) -> DpnsNameStateEntry {
        DpnsNameStateEntry {
            document_id: self.document_id,
            wallet_identity_id,
            label: self.label.clone(),
            normalized_label: self.normalized_label.clone(),
            normalized_parent_domain_name: self.normalized_parent_domain_name.clone(),
            price: self.price,
            status,
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
            transferred_at_ms: self.transferred_at_ms,
            last_synced_at_ms: now_ms,
        }
    }
}

/// One event in a name's trade timeline, assembled from the Document
/// History system contract plus the domain document's own `$createdAt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameHistoryEvent {
    pub kind: DpnsNameHistoryEventKind,
    /// Block time of the event in ms (`$createdAt` of the history
    /// document; registration uses the domain document's `$createdAt`).
    pub at_ms: u64,
    /// Block height of the event, when carried.
    pub block_height: Option<u64>,
}

/// What happened at a point in a name's trade timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpnsNameHistoryEventKind {
    /// The domain document was registered (its `$createdAt`).
    Registered,
    /// The owner listed / re-priced the name (`priceUpdate` history doc).
    PriceSet { price: Credits },
    /// The name was purchased: `seller` received `price` credits from
    /// `buyer`, who became the owner (`purchase` history doc).
    Purchased {
        price: Credits,
        seller: Identifier,
        buyer: Identifier,
    },
    /// The name was transferred without payment — a gift/handover, or a
    /// transfer-to-self delist when `from == to` (`transfer` history doc).
    Transferred { from: Identifier, to: Identifier },
}

/// One name that left a wallet identity, observed by
/// [`IdentityWallet::sync_dpns_marketplace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartedDpnsName {
    pub identity_id: Identifier,
    pub label: String,
    pub document_id: Option<Identifier>,
    /// `Some(Sold { to })` or `Some(Transferred { to })` only when a
    /// direct history event names this identity as the departing party.
    /// `None` means the document was deleted or no direct event could be
    /// resolved — unknown is never reported with a fabricated counterparty.
    pub status: Option<DpnsNameSaleStatus>,
}

/// A listed-price change observed between two sync passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsPriceChange {
    pub document_id: Identifier,
    pub label: String,
    pub previous: Option<Credits>,
    pub current: Option<Credits>,
}

/// A queued departure whose resolution failed terminally this pass.
///
/// Emitted when the persistence lookup for the departed name's
/// `document_id` fails with a NON-retryable error while Platform
/// confirms the domain document is absent: the removal delta cannot be
/// built and retrying cannot make it buildable, so the departure is NOT
/// resolved — the identity keeps its label (which is what lets a later
/// pass re-detect the departure once the backend is repaired) and
/// nothing is written or removed. Reported on the summary so a pass
/// that had to skip a departure is distinguishable from a pass that
/// resolved everything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedDpnsDeparture {
    pub identity_id: Identifier,
    pub label: String,
    /// Rendered [`PersistenceError`] (the typed error is not cloneable).
    pub error: String,
}

/// Summary of one [`IdentityWallet::sync_dpns_marketplace`] pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DpnsMarketplaceSyncSummary {
    /// Name-state rows written this pass (owned names refreshed).
    pub names_tracked: u32,
    /// Labels newly observed on a wallet identity: `(identity, label)`.
    pub names_added: Vec<(Identifier, String)>,
    /// Names that left a wallet identity since the local snapshot.
    pub names_departed: Vec<DepartedDpnsName>,
    /// Departures whose resolution failed terminally this pass; their
    /// labels and durable rows are untouched, so a later pass retries.
    /// See [`FailedDpnsDeparture`].
    pub departures_failed: Vec<FailedDpnsDeparture>,
    /// Listed-price changes since the local snapshot.
    pub prices_changed: Vec<DpnsPriceChange>,
    /// Wall-clock ms at which the pass completed.
    pub sync_unix_ms: u64,
}

impl DpnsMarketplaceSyncSummary {
    pub fn is_empty_delta(&self) -> bool {
        self.names_added.is_empty()
            && self.names_departed.is_empty()
            && self.prices_changed.is_empty()
    }
}

/// Incremental scan state shared by every clone of one wallet handle.
///
/// `seen_normalized_labels` spans all pages in the current ownership
/// scan. Departures are considered only after the final page, so a name
/// on a later page is never misclassified as having left the wallet.
#[derive(Debug, Clone, Default)]
pub(crate) struct DpnsMarketplaceSyncProgress {
    pub(crate) cursor: Option<Identifier>,
    pub(crate) seen_normalized_labels: BTreeSet<String>,
    pub(crate) pending_departures: VecDeque<DpnsNameInfo>,
}

// ---------------------------------------------------------------------------
// System contracts
// ---------------------------------------------------------------------------

/// The DPNS system contract id (fixed across contract versions).
fn dpns_contract_id() -> Identifier {
    dpp::data_contracts::SystemDataContract::DPNS.id()
}

/// The Document History system contract id.
fn document_history_contract_id() -> Identifier {
    dpp::data_contracts::SystemDataContract::DocumentHistory.id()
}

impl IdentityWallet {
    /// Fetch a system contract through this wallet's active SDK/provider.
    ///
    /// Goes through `fetch_contract_arc_for_document_op`, which also
    /// registers the contract with the SDK's context provider so
    /// document-query and post-broadcast proof verification can resolve
    /// it. Fetching (rather than loading the bundled system contract)
    /// guarantees the schema matches the network's ACTIVE contract
    /// version, and makes the marketplace self-sufficient on hosts that
    /// never seed the trusted provider's known-contracts list.
    ///
    /// Do not put these contracts in a process-global cache: two SDKs on
    /// the same network can use different providers/devnets or observe a
    /// different active protocol version. The SDK context owns whatever
    /// caching is safe for its own lifetime.
    async fn system_contract(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        self.fetch_contract_arc_for_document_op(&contract_id, document_type_name)
            .await
    }

    /// The DPNS data contract for this wallet's active SDK context.
    pub(crate) async fn dpns_contract(&self) -> Result<Arc<DataContract>, PlatformWalletError> {
        self.system_contract(dpns_contract_id(), DPNS_DOCUMENT_TYPE)
            .await
    }

    /// The Document History system contract for this wallet's network —
    /// the event log DPNS v2's `keeps*History` flags write `transfer` /
    /// `purchase` / `priceUpdate` documents into. NOT the GroveDB
    /// `documentsKeepHistory` mechanism (`getDocumentHistory` returns
    /// empty for DPNS).
    pub(crate) async fn document_history_contract(
        &self,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        self.system_contract(document_history_contract_id(), HISTORY_TYPE_TRANSFER)
            .await
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Best-effort wall-clock ms (same shape as the `acquired_at` stamps in
/// `dpns.rs`). `0` only if the system clock is before the epoch.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Strip an optional ".dash" suffix so callers can pass either "alice"
/// or "alice.dash".
fn dpns_label(name: &str) -> &str {
    name.strip_suffix(".dash").unwrap_or(name)
}

/// Insert one row produced by a marketplace sync pass.
///
/// A domain document can appear twice when it moves between two identities
/// managed by the same wallet: once as the new owner's authoritative `Owned`
/// row and once as the old owner's departure. The persistent store has one row
/// per document id, so current ownership must win independently of the
/// identity-manager's deliberately unspecified iteration order.
fn insert_sync_row(rows: &mut BTreeMap<Identifier, DpnsNameStateEntry>, entry: DpnsNameStateEntry) {
    let should_replace = rows
        .get(&entry.document_id)
        .map(|existing| {
            matches!(entry.status, DpnsNameSaleStatus::Owned)
                || !matches!(existing.status, DpnsNameSaleStatus::Owned)
        })
        .unwrap_or(true);
    if should_replace {
        rows.insert(entry.document_id, entry);
    }
}

/// Resolve the `document_id` of the name `label` as last tracked for
/// `identity_id` — the id a departure's removal delta has to carry.
///
/// Two sources, in order:
///
/// 1. `previous_rows`, the snapshot of the in-memory working set taken
///    at the top of the sync pass. Authoritative when populated, and
///    free. Retained `Sold`/`Transferred` history can coexist with the
///    current row under one normalized label, so when several rows
///    match, the CURRENT one is chosen with the same deterministic
///    preference the persistence contract requires of backends:
///    `Owned` ahead of retained history, then the greatest
///    `last_synced_at_ms`, then the greatest `document_id`.
/// 2. The persister — when the snapshot has nothing AND the backend
///    actually implements `get_dpns_name_state`. Today that is the
///    SQLite backend only. `FFIPersister` has no read slot for this
///    lookup in its vtable — there is no callback a host could set —
///    so on the mobile hosts (the Android Room and iOS SwiftData
///    mirrors) the trait's `Ok(None)` default answers, step 2 finds
///    nothing, and the restart orphan described below is STILL LIVE
///    there. That holds until a `get_dpns_name_state` read callback is
///    added to the persistence vtable, planned with the other batched
///    vtable/ABI additions rather than piecemeal.
///
/// Step 2 is not belt-and-braces; it is the only source that survives a
/// restart. `PlatformWalletInfo::dpns_name_states` is session-scoped:
/// the load path builds it EMPTY and nothing rehydrates it (see
/// [`IdentityWallet::local_dpns_name_states`]). A name that departs
/// during the FIRST sync pass after a process start therefore finds an
/// empty snapshot, and without this fallback the pass emits no removal
/// while still dropping the label — the host mirror is left holding an
/// owned/listed row that no later pass will ever revisit, because the
/// label that triggers departure detection is gone.
///
/// This function reports the lookup, it does not decide policy. Notably
/// it does NOT flatten `Err` into `Ok(None)`: those two outcomes call
/// for opposite handling and the caller
/// ([`IdentityWallet::resolve_departed_name`]) is the one placed to
/// tell them apart.
///
/// - `Ok(None)` — the backend answered, and either does not index DPNS
///   rows by label or holds no row. Nothing better is coming; proceed.
/// - `Err(_)` — a read was attempted and failed. Whether a row exists
///   is UNKNOWN, so treating it as `Ok(None)` would let a confirmed
///   Platform absence remove the label with no removal delta behind it,
///   orphaning the durable row for good.
fn previous_document_id_for(
    persister: &crate::wallet::persister::WalletPersister,
    identity_id: &Identifier,
    label: &str,
    previous_rows: &BTreeMap<Identifier, DpnsNameStateEntry>,
) -> Result<Option<Identifier>, PersistenceError> {
    let normalized_label = convert_to_homograph_safe_chars(label);
    // Several snapshot rows can match: the map is keyed by document id
    // and retains `Sold`/`Transferred` history, so one identity can
    // hold a historical row AND the current row under the same
    // normalized label (delete + re-register). A first-match in
    // document-id order could hand back the historical row — removing
    // it would drop the identity's label while orphaning the actual
    // current row, and any hit here also prevents the (corrected)
    // persistence fallback from running. Apply the same deterministic
    // current-row preference the persistence contract demands of
    // `get_dpns_name_state` implementations.
    let in_memory = previous_rows
        .values()
        .filter(|entry| {
            entry.wallet_identity_id == *identity_id && entry.normalized_label == normalized_label
        })
        .max_by_key(|entry| {
            (
                matches!(entry.status, DpnsNameSaleStatus::Owned),
                entry.last_synced_at_ms,
                entry.document_id,
            )
        })
        .map(|entry| entry.document_id);
    if in_memory.is_some() {
        return Ok(in_memory);
    }
    Ok(persister
        .get_dpns_name_state(identity_id, &normalized_label)?
        .map(|entry| entry.document_id))
}

/// The exact-match domain query behind [`IdentityWallet::dpns_name_state`]:
/// one document, keyed on the parent domain plus the normalized label.
///
/// A named builder rather than an inline literal so a test can construct
/// the identical query when priming a mock SDK — an expectation is keyed
/// by the encoded request, so a hand-copied duplicate that drifted would
/// silently stop matching and leave the test asserting nothing.
fn domain_by_normalized_label_query(
    contract: Arc<DataContract>,
    normalized_label: String,
) -> DocumentQuery {
    DocumentQuery {
        select: SelectProjection::documents(),
        data_contract: contract,
        document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
        where_clauses: vec![
            WhereClause {
                field: "normalizedParentDomainName".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(DPNS_PARENT_DOMAIN.to_string()),
            },
            WhereClause {
                field: "normalizedLabel".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(normalized_label),
            },
        ],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![],
        limit: 1,
        offset: None,
        start: None,
    }
}

/// One server page of the Document History `byDocument` query behind
/// [`IdentityWallet::fetch_history_documents`]: history documents of one
/// type for one source document, in ascending creation order.
///
/// A named builder for the same reason as
/// [`domain_by_normalized_label_query`]: a mock-SDK expectation is keyed
/// by the encoded request, so a test priming the history lookup must
/// construct the exact query the production path issues, and a
/// hand-copied duplicate that drifted would silently stop matching.
fn history_by_source_document_query(
    contract: Arc<DataContract>,
    history_doc_type: &str,
    source_contract_id: &Identifier,
    source_document_id: &Identifier,
    start: Option<Start>,
) -> DocumentQuery {
    DocumentQuery {
        select: SelectProjection::documents(),
        data_contract: contract,
        document_type_name: history_doc_type.to_string(),
        where_clauses: vec![
            WhereClause {
                field: "dataContractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(source_contract_id.to_buffer()),
            },
            WhereClause {
                field: "documentId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(source_document_id.to_buffer()),
            },
        ],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![OrderClause {
            field: "$createdAt".to_string(),
            ascending: true,
        }],
        limit: HISTORY_QUERY_LIMIT,
        offset: None,
        start,
    }
}

fn direct_departure_candidate(
    event: DpnsNameHistoryEvent,
    departing_identity: &Identifier,
) -> Option<(u64, DpnsNameSaleStatus)> {
    match event.kind {
        DpnsNameHistoryEventKind::Purchased { seller, buyer, .. }
            if seller == *departing_identity =>
        {
            Some((event.at_ms, DpnsNameSaleStatus::Sold { to: buyer }))
        }
        DpnsNameHistoryEventKind::Transferred { from, to } if from == *departing_identity => {
            Some((event.at_ms, DpnsNameSaleStatus::Transferred { to }))
        }
        _ => None,
    }
}

/// How [`IdentityWallet::resolve_departed_name`] left one queued
/// departure, and therefore what the sync loop is allowed to do with it.
#[derive(Debug)]
enum DepartureResolution {
    /// Fully resolved: the caller may drop the identity's label and
    /// apply the row deltas.
    Resolved,
    /// Transient failure (network read, history classification, or a
    /// retryable persistence read): the caller requeues the departure at
    /// the front and breaks; the next pass retries with the label and
    /// the durable row untouched.
    Retry,
    /// Terminal per-item failure: a NON-retryable persistence read
    /// failure while Platform confirms the domain document absent, so
    /// the removal delta's `document_id` is unknown and will not become
    /// known by retrying. The caller MUST NOT take the
    /// successful-departure path — no label drop, no deltas, no entry in
    /// `names_departed` — because the retained label is the only trigger
    /// that lets a later pass re-detect the departure and finish the
    /// removal once the backend is repaired. Surfaced on the summary as
    /// a [`FailedDpnsDeparture`] rather than silently swallowed.
    Failed(PersistenceError),
}

struct ResolvedDepartedName {
    summary: DepartedDpnsName,
    entry: Option<DpnsNameStateEntry>,
    remove_document_id: Option<Identifier>,
    resolution: DepartureResolution,
}

impl IdentityWallet {
    // -----------------------------------------------------------------
    // Queries (network reads, sale state included)
    // -----------------------------------------------------------------

    /// Search DPNS names by prefix, returning full domain state (document
    /// id, owner, `$price`, timestamps) ordered by normalized label.
    ///
    /// An empty prefix is a valid alphabetical browse (equality on the
    /// parent domain + orderBy label). `start_after` is the cursor: pass
    /// the last row's `document_id` to fetch the next page. There is NO
    /// server-side price filter or ordering — `$price` is not indexable,
    /// so the marketplace is search-driven.
    pub async fn search_dpns_names_with_state(
        &self,
        prefix: &str,
        limit: Option<u32>,
        start_after: Option<Identifier>,
    ) -> Result<Vec<DpnsDomainState>, PlatformWalletError> {
        let contract = self.dpns_contract().await?;
        let normalized_prefix = convert_to_homograph_safe_chars(dpns_label(prefix));
        let mut where_clauses = vec![WhereClause {
            field: "normalizedParentDomainName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(DPNS_PARENT_DOMAIN.to_string()),
        }];
        if !normalized_prefix.is_empty() {
            where_clauses.push(WhereClause {
                field: "normalizedLabel".to_string(),
                operator: WhereOperator::StartsWith,
                value: Value::Text(normalized_prefix),
            });
        }
        let query = DocumentQuery {
            select: SelectProjection::documents(),
            data_contract: contract,
            document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
            where_clauses,
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![OrderClause {
                field: "normalizedLabel".to_string(),
                ascending: true,
            }],
            limit: limit.unwrap_or(DEFAULT_SEARCH_LIMIT),
            offset: None,
            start: start_after.map(|id| Start::StartAfter(id.to_vec())),
        };
        self.fetch_domain_states(query).await
    }

    /// Fetch the single DPNS domain document for `name` ("alice" or
    /// "alice.dash"), or `None` when no such document is in the tree.
    pub async fn dpns_name_state(
        &self,
        name: &str,
    ) -> Result<Option<DpnsDomainState>, PlatformWalletError> {
        let contract = self.dpns_contract().await?;
        let normalized = convert_to_homograph_safe_chars(dpns_label(name));
        if normalized.is_empty() {
            return Err(PlatformWalletError::InvalidParameter(
                "DPNS name must not be empty".to_string(),
            ));
        }
        Ok(self
            .fetch_domain_states(domain_by_normalized_label_query(contract, normalized))
            .await?
            .into_iter()
            .next())
    }

    /// Fetch the domain documents associated with `identity_id` via the
    /// `records.identity` index (the only identity-keyed index; the
    /// protocol rewrites `records.identity` to the new owner on
    /// purchase/transfer, so this stays authoritative across sales).
    /// `None` drains every server page; `Some(n)` returns at most `n`
    /// documents while still respecting the server's per-page limit.
    pub async fn dpns_domain_states_for_identity(
        &self,
        identity_id: &Identifier,
        limit: Option<u32>,
    ) -> Result<Vec<DpnsDomainState>, PlatformWalletError> {
        if limit == Some(0) {
            return Ok(Vec::new());
        }
        let contract = self.dpns_contract().await?;
        let maximum = limit.map(|value| value as usize);
        let mut states = Vec::new();
        let mut cursor: Option<Identifier> = None;

        loop {
            let remaining = maximum.map(|value| value.saturating_sub(states.len()));
            let page_limit = remaining
                .map(|value| value.min(SYNC_QUERY_LIMIT as usize))
                .unwrap_or(SYNC_QUERY_LIMIT as usize);
            if page_limit == 0 {
                break;
            }

            let (page, next_cursor, complete) = self
                .dpns_domain_states_page(
                    Arc::clone(&contract),
                    identity_id,
                    cursor,
                    page_limit as u32,
                )
                .await?;
            states.extend(page);

            if complete || maximum.is_some_and(|value| states.len() >= value) {
                break;
            }
            if cursor == next_cursor {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "DPNS identity query pagination cursor did not advance".to_string(),
                ));
            }
            cursor = next_cursor;
        }

        Ok(states)
    }

    /// Fetch exactly one identity-owned DPNS page. The returned cursor is
    /// retained by marketplace sync so one pass never drains an unbounded
    /// document set.
    async fn dpns_domain_states_page(
        &self,
        contract: Arc<DataContract>,
        identity_id: &Identifier,
        start_after: Option<Identifier>,
        page_limit: u32,
    ) -> Result<(Vec<DpnsDomainState>, Option<Identifier>, bool), PlatformWalletError> {
        let query = DocumentQuery {
            select: SelectProjection::documents(),
            data_contract: contract,
            document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
            where_clauses: vec![WhereClause {
                field: "records.identity".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(identity_id.to_buffer()),
            }],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: page_limit,
            offset: None,
            start: start_after.map(|id| Start::StartAfter(id.to_vec())),
        };
        let documents = Document::fetch_many(&self.sdk, query).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to fetch DPNS domain documents: {e}"
            ))
        })?;
        let page_len = documents.len();
        let next_cursor = documents.keys().last().copied();
        let states = documents
            .into_iter()
            .filter_map(|(_, document)| document)
            .map(|document| DpnsDomainState::from_document(&document))
            .collect::<Result<Vec<_>, _>>()?;
        let complete = page_len < page_limit as usize;
        if !complete && next_cursor.is_none() {
            return Err(PlatformWalletError::InvalidIdentityData(
                "full DPNS identity query page did not provide a pagination cursor".to_string(),
            ));
        }
        Ok((states, next_cursor, complete))
    }

    /// The tracked marketplace rows (owned names with sale state, plus
    /// retained `Sold`/`Transferred` rows), optionally filtered to one
    /// wallet identity. Reads the in-memory working set — no network.
    ///
    /// **Session-scoped.** This map starts EMPTY on every process start
    /// and is repopulated by the first
    /// [`sync_dpns_marketplace`](Self::sync_dpns_marketplace) pass; the
    /// wallet load path does not rehydrate it. That mirrors the
    /// invitations store — `SqlitePersister` does not attest
    /// `WALLET_RESTORE` and `load()` still reports
    /// `ClientStartState::wallets` in `LOAD_UNIMPLEMENTED`. The durable
    /// copy a host should render after a restart is the persister mirror
    /// (Swift `PersistentDPNSName`), which the changeset feeds; treat an
    /// empty return here as "not synced yet", never as "no names".
    pub async fn local_dpns_name_states(
        &self,
        identity_id: Option<&Identifier>,
    ) -> Result<Vec<DpnsNameStateEntry>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        Ok(info
            .dpns_name_states
            .values()
            .filter(|entry| identity_id.is_none_or(|id| entry.wallet_identity_id == *id))
            .cloned()
            .collect())
    }

    /// Run `query` and convert the returned documents, preserving server
    /// order (the result map is an `IndexMap`).
    async fn fetch_domain_states(
        &self,
        query: DocumentQuery,
    ) -> Result<Vec<DpnsDomainState>, PlatformWalletError> {
        let documents = Document::fetch_many(&self.sdk, query).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to fetch DPNS domain documents: {e}"
            ))
        })?;
        documents
            .into_iter()
            .filter_map(|(_, doc)| doc)
            .map(|doc| DpnsDomainState::from_document(&doc))
            .collect()
    }

    /// Resolve `name` to its domain state or fail typed: a name hidden
    /// inside an active contested-name vote is NOT in the documents tree
    /// (the network would answer any trade with a bare
    /// `DocumentNotFoundError`), so the miss is classified before it is
    /// reported — [`PlatformWalletError::ContestedNameNotTradable`] when
    /// an active contest holds the label,
    /// [`PlatformWalletError::DpnsNameNotFound`] otherwise.
    async fn fetch_dpns_domain_state_required(
        &self,
        name: &str,
    ) -> Result<DpnsDomainState, PlatformWalletError> {
        if let Some(state) = self.dpns_name_state(name).await? {
            return Ok(state);
        }
        let label = dpns_label(name);
        if is_contested_username(label) {
            let normalized = convert_to_homograph_safe_chars(label);
            let contests = self
                .sdk
                .get_current_dpns_contests(None, None, None)
                .await
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to check contested-name votes for {label:?}: {e}"
                    ))
                })?;
            if let Some(end_time_ms) = contests.get(&normalized) {
                return Err(PlatformWalletError::ContestedNameNotTradable {
                    label: label.to_string(),
                    ends_at_ms: *end_time_ms,
                });
            }
        }
        Err(PlatformWalletError::DpnsNameNotFound {
            name: name.to_string(),
        })
    }

    // -----------------------------------------------------------------
    // Signing-key selection
    // -----------------------------------------------------------------

    /// Auto-select the signing key for a DPNS `domain` state transition
    /// on `identity_id`: the identity's first AUTHENTICATION-purpose
    /// ECDSA_SECP256K1 key whose security level satisfies the document
    /// type's requirement (the same consensus rule
    /// [`allowed_signing_security_levels`] encodes). Replaces the app
    /// layer's hardcoded "key id 1".
    async fn select_dpns_signing_key(
        &self,
        identity_id: &Identifier,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let contract = self.dpns_contract().await?;
        let required_level = contract
            .document_type_for_name(DPNS_DOCUMENT_TYPE)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "DPNS contract has no {DPNS_DOCUMENT_TYPE:?} document type: {e}"
                ))
            })?
            .security_level_requirement();
        let allowed_levels = allowed_signing_security_levels(required_level);
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        let identity = info
            .identity_manager
            .wallet_identity(&self.wallet_id, identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        identity
            .get_first_public_key_matching(
                Purpose::AUTHENTICATION,
                allowed_levels.iter().copied().collect(),
                [KeyType::ECDSA_SECP256K1].into(),
                false,
            )
            .cloned()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "No ECDSA authentication key at a security level satisfying \
                     {required_level} found on identity {identity_id} \
                     (required to sign a DPNS domain state transition)"
                ))
            })
    }

    // -----------------------------------------------------------------
    // Local bookkeeping
    // -----------------------------------------------------------------

    /// Upsert marketplace rows (and optional removals) into the in-memory
    /// working set and emit the changeset so the host mirror persists it.
    async fn record_dpns_name_states(
        &self,
        entries: Vec<DpnsNameStateEntry>,
        removed: Vec<Identifier>,
    ) {
        if entries.is_empty() && removed.is_empty() {
            return;
        }
        let mut cs = DpnsNameStateChangeSet::default();
        for entry in entries {
            cs.names.insert(entry.document_id, entry);
        }
        cs.removed.extend(removed);
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        info.dpns_name_states.extend(cs.names.clone());
        for document_id in &cs.removed {
            info.dpns_name_states.remove(document_id);
        }
        // Same best-effort discipline as `add_dpns_name`: the in-memory
        // mutation stands for this session; a failed store is logged and
        // the next sync pass re-emits the same rows (self-healing).
        if let Err(e) = self.persister.store(cs.into()) {
            tracing::error!("Failed to persist DPNS name states: {e}");
        }
    }

    /// Add `label` to `identity_id`'s legacy label list if absent
    /// (persisting the identity snapshot). No-op when already present.
    async fn add_dpns_label_if_missing(
        &self,
        identity_id: &Identifier,
        label: &str,
        acquired_at: Option<u64>,
    ) {
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        let Some(managed) = info
            .identity_manager
            .wallet_identity_mut(&self.wallet_id, identity_id)
        else {
            return;
        };
        if managed.dpns_names.iter().any(|n| n.label == label) {
            return;
        }
        managed.add_dpns_name(
            DpnsNameInfo {
                label: label.to_string(),
                acquired_at,
            },
            &self.persister,
        );
    }

    /// Remove `label` from `identity_id`'s legacy label list (persisting
    /// the identity snapshot). No-op when absent or the identity isn't
    /// in this wallet.
    async fn remove_dpns_label(&self, identity_id: &Identifier, label: &str) {
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        let Some(managed) = info
            .identity_manager
            .wallet_identity_mut(&self.wallet_id, identity_id)
        else {
            return;
        };
        managed.remove_dpns_name(label, &self.persister);
    }

    /// Whether `identity_id` is one of this wallet's identities.
    async fn is_wallet_identity(&self, identity_id: &Identifier) -> bool {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| {
                info.identity_manager
                    .wallet_identity(&self.wallet_id, identity_id)
                    .is_some()
            })
            .unwrap_or(false)
    }

    /// Best-effort identity refresh after a trade moved credits or
    /// ownership: failures are logged, never propagated — the trade
    /// already executed on Platform and must be reported as such.
    async fn refresh_identity_after_trade(&self, identity_id: &Identifier, context: &str) {
        if let Err(e) = self.refresh_identity(identity_id).await {
            tracing::warn!(
                identity = %identity_id,
                "post-{context} identity refresh failed (will self-heal on next sync): {e}"
            );
        }
    }

    // -----------------------------------------------------------------
    // Sell / delist / transfer / purchase orchestration
    // -----------------------------------------------------------------

    /// List (or re-price) `name` for sale at `price` credits.
    ///
    /// Pre-flight: `price` must be non-zero (see below); the name must
    /// resolve to a domain document owned by `owner_identity_id` (typed
    /// contested/not-found errors otherwise). The signing key is
    /// auto-selected on the owner. On success the local sale state is
    /// persisted from the confirmed document and the updated state
    /// returned.
    ///
    /// **`price == 0` is rejected** with
    /// [`PlatformWalletError::InvalidParameter`] before any network
    /// work. Consensus would accept the listing, and it would then be
    /// purchasable by anyone for nothing — the name is gone, credited
    /// zero, and only a delist (or a race the owner loses) undoes it.
    /// There is no legitimate caller: a deliberate free handover is
    /// [`Self::transfer_dpns_name`], which names the recipient. A zero
    /// reaching here is a host-side bug or a fat-fingered amount field,
    /// so it fails loudly rather than broadcasting an irreversible
    /// giveaway.
    pub async fn set_dpns_name_price<S>(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
        price: Credits,
        signer: &S,
    ) -> Result<DpnsDomainState, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // Ahead of the operation gate and the domain fetch: nothing about
        // the on-chain state can make a zero-credit listing valid, so no
        // lock is worth holding and no round-trip is worth spending.
        if price == 0 {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} cannot be listed at 0 credits — a zero price is not a \
                 sale, it lets anyone take the name for free. Use transfer_dpns_name to \
                 hand it over deliberately, or list at a non-zero price."
            )));
        }
        let _operation = self.dpns_operation_gate.lock().await;
        let state = self.fetch_dpns_domain_state_required(name).await?;
        if state.owner_id != *owner_identity_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} is owned by {}, not by {owner_identity_id}",
                state.owner_id
            )));
        }
        let signing_key = self.select_dpns_signing_key(owner_identity_id).await?;
        let contract_id = dpns_contract_id();
        let confirmed = self
            .set_document_price_with_signer(
                owner_identity_id,
                &contract_id,
                DPNS_DOCUMENT_TYPE,
                &state.document_id,
                price,
                signing_key.id(),
                signer,
            )
            .await?;
        let confirmed_state = DpnsDomainState::from_document(&confirmed)?;
        self.record_dpns_name_states(
            vec![confirmed_state.to_entry(*owner_identity_id, DpnsNameSaleStatus::Owned, now_ms())],
            vec![],
        )
        .await;
        Ok(confirmed_state)
    }

    /// Delist `name` — a transfer to the owner's own identity, which
    /// consensus strips `$price` from while leaving ownership unchanged
    /// (DPNS has no dedicated remove-price transition and
    /// `documentsMutable=false` rules out a replace).
    ///
    /// The confirmed document is verified to actually carry no `$price`
    /// and the same owner; if consensus semantics ever change, this
    /// fails loudly instead of persisting a delist that didn't happen.
    pub async fn delist_dpns_name<S>(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
        signer: &S,
    ) -> Result<DpnsDomainState, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let _operation = self.dpns_operation_gate.lock().await;
        let state = self.fetch_dpns_domain_state_required(name).await?;
        if state.owner_id != *owner_identity_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} is owned by {}, not by {owner_identity_id}",
                state.owner_id
            )));
        }
        if state.price.is_none() {
            return Err(PlatformWalletError::DocumentNotForSale {
                document_id: state.document_id,
            });
        }
        let signing_key = self.select_dpns_signing_key(owner_identity_id).await?;
        let contract_id = dpns_contract_id();
        let confirmed = self
            .transfer_document_with_signer(
                owner_identity_id,
                &contract_id,
                DPNS_DOCUMENT_TYPE,
                &state.document_id,
                owner_identity_id,
                signing_key.id(),
                signer,
            )
            .await?;
        let confirmed_state = DpnsDomainState::from_document(&confirmed)?;
        if confirmed_state.price.is_some() || confirmed_state.owner_id != *owner_identity_id {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "delist of {name:?} broadcast a self-transfer but the confirmed document \
                 still carries price={:?} owner={} — transfer-to-self no longer clears \
                 $price; do not trust the local delist state",
                confirmed_state.price, confirmed_state.owner_id
            )));
        }
        self.record_dpns_name_states(
            vec![confirmed_state.to_entry(*owner_identity_id, DpnsNameSaleStatus::Owned, now_ms())],
            vec![],
        )
        .await;
        Ok(confirmed_state)
    }

    /// Transfer `name` to `recipient_id` (gift / off-market handover).
    /// Consensus strips any `$price` on transfer, so this also delists.
    ///
    /// Both sides are reconciled locally when they belong to this wallet:
    /// the sender loses the label (row → `Transferred`), a wallet-owned
    /// recipient gains it (row → `Owned`).
    pub async fn transfer_dpns_name<S>(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
        recipient_id: &Identifier,
        signer: &S,
    ) -> Result<DpnsDomainState, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let _operation = self.dpns_operation_gate.lock().await;
        if recipient_id == owner_identity_id {
            return Err(PlatformWalletError::InvalidParameter(
                "transfer recipient is the current owner — use delist_dpns_name for a \
                 transfer-to-self delist"
                    .to_string(),
            ));
        }
        let state = self.fetch_dpns_domain_state_required(name).await?;
        if state.owner_id != *owner_identity_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} is owned by {}, not by {owner_identity_id}",
                state.owner_id
            )));
        }
        let signing_key = self.select_dpns_signing_key(owner_identity_id).await?;
        let contract_id = dpns_contract_id();
        let confirmed = self
            .transfer_document_with_signer(
                owner_identity_id,
                &contract_id,
                DPNS_DOCUMENT_TYPE,
                &state.document_id,
                recipient_id,
                signing_key.id(),
                signer,
            )
            .await?;
        let confirmed_state = DpnsDomainState::from_document(&confirmed)?;
        let now = now_ms();
        self.remove_dpns_label(owner_identity_id, &confirmed_state.label)
            .await;
        if self.is_wallet_identity(recipient_id).await {
            // Both sides ours: the single per-document row tracks the new
            // owner; the departure is visible through the label removal.
            self.add_dpns_label_if_missing(
                recipient_id,
                &confirmed_state.label,
                confirmed_state.transferred_at_ms.or(Some(now)),
            )
            .await;
            self.record_dpns_name_states(
                vec![confirmed_state.to_entry(*recipient_id, DpnsNameSaleStatus::Owned, now)],
                vec![],
            )
            .await;
        } else {
            self.record_dpns_name_states(
                vec![confirmed_state.to_entry(
                    *owner_identity_id,
                    DpnsNameSaleStatus::Transferred { to: *recipient_id },
                    now,
                )],
                vec![],
            )
            .await;
        }
        Ok(confirmed_state)
    }

    /// Purchase `name` at exactly `expected_price` credits (the price the
    /// user confirmed) for `purchaser_identity_id`.
    ///
    /// Pre-flight, all typed: name resolution (contested-aware), a
    /// self-purchase guard, [`PlatformWalletError::DocumentNotForSale`],
    /// [`PlatformWalletError::InvalidParameter`] for a `$price` of 0
    /// (not a valid listing — see [`Self::set_dpns_name_price`], which
    /// refuses to create one),
    /// [`PlatformWalletError::DocumentPriceChanged`] when the listing no
    /// longer matches `expected_price`, and
    /// [`PlatformWalletError::InsufficientIdentityCredits`] when the
    /// buyer's local balance can't cover
    /// `expected_price + `[`DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS`].
    ///
    /// The broadcast transition carries `expected_price` — NEVER the
    /// re-read price — so a listing change between pre-flight and
    /// broadcast is rejected by consensus (code 40109) and surfaces as
    /// the same typed `DocumentPriceChanged`.
    ///
    /// On success both sides are reconciled locally: the buyer gains the
    /// label and the name-state row (`Owned`), and a wallet-owned seller
    /// loses the label. Both identities' balances are refreshed
    /// best-effort. Note the seller does NOT get a `Sold` row when both
    /// parties live in this wallet — rows are keyed by `document_id`
    /// alone, and the buyer's `Owned` row already occupies that key; the
    /// seller's departure is represented by the label removal.
    /// `Sold` rows are written by the sync pass, which sees a name leave
    /// an identity it still tracks (same keying constraint as
    /// [`Self::transfer_dpns_name`]).
    pub async fn purchase_dpns_name<S>(
        &self,
        purchaser_identity_id: &Identifier,
        name: &str,
        expected_price: Credits,
        signer: &S,
    ) -> Result<DpnsDomainState, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let _operation = self.dpns_operation_gate.lock().await;
        let state = self.fetch_dpns_domain_state_required(name).await?;
        if state.owner_id == *purchaser_identity_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "identity {purchaser_identity_id} already owns DPNS name {name:?}"
            )));
        }
        preflight_purchase_price(&state, name, expected_price)?;
        // Credit pre-flight against the local balance snapshot: Platform
        // deducts the price as principal first, then the processing fee
        // must fit in the remainder. The consensus-side
        // `IdentityInsufficientBalanceError` (typed through
        // `promote_document_trade_error`) is the backstop for a stale
        // local balance.
        let available = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .wallet_identity(&self.wallet_id, purchaser_identity_id)
                .map(|m| m.balance())
                .ok_or(PlatformWalletError::IdentityNotFound(
                    *purchaser_identity_id,
                ))?
        };
        let required = required_purchase_credits(expected_price)?;
        if available < required {
            return Err(PlatformWalletError::InsufficientIdentityCredits {
                identity_id: *purchaser_identity_id,
                required,
                available,
            });
        }
        let signing_key = self.select_dpns_signing_key(purchaser_identity_id).await?;
        let contract_id = dpns_contract_id();
        let confirmed = self
            .purchase_document_with_signer(
                purchaser_identity_id,
                &contract_id,
                DPNS_DOCUMENT_TYPE,
                &state.document_id,
                expected_price,
                signing_key.id(),
                signer,
            )
            .await?;
        let confirmed_state = DpnsDomainState::from_document(&confirmed)?;
        let now = now_ms();
        let seller_id = state.owner_id;

        // Buyer side: label + row + balance.
        self.add_dpns_label_if_missing(
            purchaser_identity_id,
            &confirmed_state.label,
            confirmed_state.transferred_at_ms.or(Some(now)),
        )
        .await;
        self.record_dpns_name_states(
            vec![confirmed_state.to_entry(*purchaser_identity_id, DpnsNameSaleStatus::Owned, now)],
            vec![],
        )
        .await;
        self.refresh_identity_after_trade(purchaser_identity_id, "purchase (buyer)")
            .await;

        // Seller side, when the seller is also one of this wallet's
        // identities: the sold name leaves the label list (the host's
        // main-username selection falls back to the remaining labels off
        // the mirrored identity row) and the seller's balance — which
        // just received the sale price — is refreshed.
        if self.is_wallet_identity(&seller_id).await {
            self.remove_dpns_label(&seller_id, &confirmed_state.label)
                .await;
            self.refresh_identity_after_trade(&seller_id, "purchase (seller)")
                .await;
        }
        Ok(confirmed_state)
    }

    // -----------------------------------------------------------------
    // History
    // -----------------------------------------------------------------

    /// The trade timeline of `name`: registration, price changes,
    /// purchases (with price + counterparties), and transfers — read
    /// from the Document History system contract's `priceUpdate` /
    /// `purchase` / `transfer` documents (`byDocument` index), merged
    /// and ordered by block time ascending.
    ///
    /// Works for names that already left the wallet: when the live
    /// domain document can't be resolved, the document id is taken from
    /// the local marketplace rows.
    pub async fn dpns_name_history(
        &self,
        name: &str,
    ) -> Result<Vec<DpnsNameHistoryEvent>, PlatformWalletError> {
        // Resolve the domain document id (live first, local rows for
        // departed names) and the registration timestamp when known.
        let live = self.dpns_name_state(name).await?;
        let (document_id, registered_at_ms) = match &live {
            Some(state) => (state.document_id, state.created_at_ms),
            None => {
                let normalized = convert_to_homograph_safe_chars(dpns_label(name));
                let local = self
                    .local_dpns_name_states(None)
                    .await?
                    .into_iter()
                    .find(|entry| entry.normalized_label == normalized);
                match local {
                    Some(entry) => (entry.document_id, entry.created_at_ms),
                    None => {
                        // Reuse the contested-aware classification for the
                        // typed error. If the name appeared between the two
                        // reads (registration race), just use it.
                        match self.fetch_dpns_domain_state_required(name).await {
                            Ok(state) => (state.document_id, state.created_at_ms),
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        };
        self.dpns_document_history(&document_id, registered_at_ms)
            .await
    }

    /// History timeline for a known domain `document_id`. See
    /// [`Self::dpns_name_history`].
    pub async fn dpns_document_history(
        &self,
        document_id: &Identifier,
        registered_at_ms: Option<u64>,
    ) -> Result<Vec<DpnsNameHistoryEvent>, PlatformWalletError> {
        let dpns_contract_id = dpns_contract_id();
        let mut events: Vec<DpnsNameHistoryEvent> = Vec::new();
        if let Some(at_ms) = registered_at_ms {
            events.push(DpnsNameHistoryEvent {
                kind: DpnsNameHistoryEventKind::Registered,
                at_ms,
                block_height: None,
            });
        }
        for doc_type in [
            HISTORY_TYPE_PRICE_UPDATE,
            HISTORY_TYPE_PURCHASE,
            HISTORY_TYPE_TRANSFER,
        ] {
            let docs = self
                .fetch_history_documents(&dpns_contract_id, document_id, doc_type)
                .await?;
            for doc in docs {
                events.push(history_event_from_document(doc_type, &doc)?);
            }
        }
        events.sort_by_key(|e| e.at_ms);
        Ok(events)
    }

    /// Fetch every history document of one type for a source document via
    /// the `byDocument` (dataContractId, documentId, $createdAt) index,
    /// draining its server pages in ascending creation order.
    async fn fetch_history_documents(
        &self,
        source_contract_id: &Identifier,
        source_document_id: &Identifier,
        history_doc_type: &str,
    ) -> Result<Vec<Document>, PlatformWalletError> {
        let contract = self.document_history_contract().await?;
        let mut all_documents = Vec::new();
        let mut cursor: Option<Identifier> = None;

        loop {
            let query = history_by_source_document_query(
                Arc::clone(&contract),
                history_doc_type,
                source_contract_id,
                source_document_id,
                cursor.map(|id| Start::StartAfter(id.to_vec())),
            );
            let documents = Document::fetch_many(&self.sdk, query).await.map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch {history_doc_type} history documents: {e}"
                ))
            })?;
            let page_len = documents.len();
            let last_id = documents.keys().last().copied();
            all_documents.extend(documents.into_iter().filter_map(|(_, document)| document));

            if page_len < HISTORY_QUERY_LIMIT as usize {
                break;
            }
            let Some(last_id) = last_id else {
                break;
            };
            if cursor == Some(last_id) {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "{history_doc_type} history pagination cursor did not advance"
                )));
            }
            cursor = Some(last_id);
        }

        Ok(all_documents)
    }

    // -----------------------------------------------------------------
    // Sync
    // -----------------------------------------------------------------

    /// One marketplace sync pass over every identity in this wallet:
    /// refreshes owned-name rows (price/sale state), adds newly observed
    /// names to the legacy label list, detects names that LEFT an
    /// identity (sold or transferred away — classified through the
    /// history contract), removes their labels, and refreshes the
    /// balances of identities that sold a name.
    ///
    /// All network reads happen before the wallet-manager write lock is
    /// taken; per-identity failures are logged and skipped, never
    /// aborting the pass.
    pub async fn sync_dpns_marketplace(
        &self,
    ) -> Result<DpnsMarketplaceSyncSummary, PlatformWalletError> {
        let _operation = self.dpns_operation_gate.lock().await;
        // Snapshot identity ids, their label lists, and the current rows.
        let (identity_ids, labels_by_identity, previous_rows) = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let ids = info.identity_manager.wallet_identity_ids(&self.wallet_id);
            let labels: BTreeMap<Identifier, Vec<DpnsNameInfo>> = info
                .identity_manager
                .wallet_managed_identities(&self.wallet_id)
                .map(|managed| (managed.identity.id(), managed.dpns_names.clone()))
                .collect();
            (ids, labels, info.dpns_name_states.clone())
        };

        let mut summary = DpnsMarketplaceSyncSummary::default();
        let mut rows_to_write: BTreeMap<Identifier, DpnsNameStateEntry> = BTreeMap::new();
        let mut rows_to_remove: BTreeSet<Identifier> = BTreeSet::new();
        let mut sellers_to_refresh: Vec<Identifier> = Vec::new();
        let now = now_ms();
        let contract = self.dpns_contract().await?;

        for identity_id in identity_ids {
            let previous_labels = labels_by_identity
                .get(&identity_id)
                .cloned()
                .unwrap_or_default();

            let mut progress = self
                .dpns_sync_progress
                .lock()
                .map_err(|_| {
                    PlatformWalletError::InvalidIdentityData(
                        "DPNS marketplace sync progress lock was poisoned".to_string(),
                    )
                })?
                .get(&identity_id)
                .cloned()
                .unwrap_or_default();

            if progress.pending_departures.is_empty() {
                let (states, next_cursor, complete) = match self
                    .dpns_domain_states_page(
                        Arc::clone(&contract),
                        &identity_id,
                        progress.cursor,
                        SYNC_QUERY_LIMIT,
                    )
                    .await
                {
                    Ok(page) => page,
                    Err(e) => {
                        tracing::warn!(
                            identity = %identity_id,
                            "DPNS marketplace sync: domain-state page failed, retaining cursor: {e}"
                        );
                        continue;
                    }
                };
                // `records.identity` follows ownership on-chain, but filter on
                // `$ownerId` anyway so a protocol edge (or pre-rewrite record)
                // can't count someone else's document as ours.
                let owned: Vec<&DpnsDomainState> = states
                    .iter()
                    .filter(|state| state.owner_id == identity_id)
                    .collect();

                progress
                    .seen_normalized_labels
                    .extend(owned.iter().map(|state| state.normalized_label.clone()));

                // Owned rows: upsert, tracking price changes vs the previous row.
                for state in &owned {
                    if let Some(prev) = previous_rows.get(&state.document_id) {
                        if prev.wallet_identity_id == identity_id && prev.price != state.price {
                            summary.prices_changed.push(DpnsPriceChange {
                                document_id: state.document_id,
                                label: state.label.clone(),
                                previous: prev.price,
                                current: state.price,
                            });
                        }
                    }
                    insert_sync_row(
                        &mut rows_to_write,
                        state.to_entry(identity_id, DpnsNameSaleStatus::Owned, now),
                    );
                    summary.names_tracked += 1;
                }

                // Newly observed labels → legacy list additions.
                for state in &owned {
                    let known = previous_labels.iter().any(|name| {
                        convert_to_homograph_safe_chars(&name.label) == state.normalized_label
                    });
                    if !known {
                        self.add_dpns_label_if_missing(
                            &identity_id,
                            &state.label,
                            state
                                .transferred_at_ms
                                .or(state.created_at_ms)
                                .or(Some(now)),
                        )
                        .await;
                        summary.names_added.push((identity_id, state.label.clone()));
                    }
                }

                if complete {
                    progress.cursor = None;
                    progress.pending_departures = previous_labels
                        .iter()
                        .filter(|name| {
                            !progress
                                .seen_normalized_labels
                                .contains(&convert_to_homograph_safe_chars(&name.label))
                        })
                        .cloned()
                        .collect();
                    progress.seen_normalized_labels.clear();
                } else {
                    progress.cursor = next_cursor;
                }
            }

            // Resolve only a fixed number of departed names per pass. A
            // transient domain fetch error keeps the item queued and its
            // label/row intact, so the next pass retries without data loss.
            let mut departures_processed = 0;
            while departures_processed < SYNC_DEPARTURE_LIMIT {
                let Some(previous_name) = progress.pending_departures.pop_front() else {
                    break;
                };
                let resolved = self
                    .resolve_departed_name(&identity_id, &previous_name.label, &previous_rows, now)
                    .await;
                match resolved.resolution {
                    DepartureResolution::Retry => {
                        progress.pending_departures.push_front(previous_name);
                        break;
                    }
                    // Terminal for this pass, but NOT resolved: the label
                    // stays (so a later scan re-detects the departure and
                    // requeues it once the backend is repaired), no deltas
                    // are applied, and the failure is surfaced on the
                    // summary. Deliberately not requeued in
                    // `pending_departures`: retrying a non-retryable
                    // failure within this process cannot succeed, and the
                    // retained label already guarantees re-detection.
                    DepartureResolution::Failed(error) => {
                        departures_processed += 1;
                        summary.departures_failed.push(FailedDpnsDeparture {
                            identity_id,
                            label: previous_name.label,
                            error: error.to_string(),
                        });
                        continue;
                    }
                    DepartureResolution::Resolved => {}
                }
                departures_processed += 1;
                self.remove_dpns_label(&identity_id, &previous_name.label)
                    .await;
                if let Some(entry) = resolved.entry {
                    insert_sync_row(&mut rows_to_write, entry);
                }
                if let Some(document_id) = resolved.remove_document_id {
                    rows_to_remove.insert(document_id);
                }
                if matches!(
                    resolved.summary.status,
                    Some(DpnsNameSaleStatus::Sold { .. })
                ) {
                    sellers_to_refresh.push(identity_id);
                }
                summary.names_departed.push(resolved.summary);
            }

            let mut sync_progress = self.dpns_sync_progress.lock().map_err(|_| {
                PlatformWalletError::InvalidIdentityData(
                    "DPNS marketplace sync progress lock was poisoned".to_string(),
                )
            })?;
            if progress.cursor.is_none()
                && progress.seen_normalized_labels.is_empty()
                && progress.pending_departures.is_empty()
            {
                sync_progress.remove(&identity_id);
            } else {
                sync_progress.insert(identity_id, progress);
            }
        }

        // A current owned row wins when one document moves between two
        // identities in this wallet during the same pass.
        rows_to_remove.retain(|document_id| !rows_to_write.contains_key(document_id));
        self.record_dpns_name_states(
            rows_to_write.into_values().collect(),
            rows_to_remove.into_iter().collect(),
        )
        .await;
        sellers_to_refresh.sort();
        sellers_to_refresh.dedup();
        for seller in sellers_to_refresh {
            self.refresh_identity_after_trade(&seller, "marketplace sync (sold name)")
                .await;
        }
        summary.sync_unix_ms = now_ms();
        Ok(summary)
    }

    /// Work out what happened to a name that left `identity_id`: fetch
    /// the domain document by label to learn the new owner, then
    /// classify the departure through the history contract.
    ///
    /// A confirmed missing document removes the stale local row. A
    /// transport/query error requests a retry and leaves both the label
    /// and local row untouched.
    ///
    /// A live document whose history never departs this identity removes
    /// the identity's own RECOVERED row, which is the live document only
    /// when the two ids match: DPNS domain documents are deletable, and
    /// a label re-registered under a fresh document id leaves the live
    /// document belonging to the replacement owner, so the removal must
    /// target the recovered prior incarnation and leave the replacement
    /// untouched.
    ///
    /// The removal delta needs the departed name's `document_id`, which
    /// [`previous_document_id_for`] resolves from the in-memory snapshot
    /// and — when that is empty, as it always is on the first pass after
    /// a process start — from the persister, on backends that implement
    /// the lookup (SQLite today; FFI hosts have no read slot yet and
    /// still resolve nothing — see [`previous_document_id_for`]).
    ///
    /// A FAILED persistence read is treated like a failed network read,
    /// not like "no row": a transient error requests a retry and leaves
    /// the label, the pending departure and the durable row untouched.
    /// The alternative — carrying on with no id — is the worst of the
    /// available outcomes, because the very next step can be a confirmed
    /// Platform absence, which removes the label with no removal delta
    /// behind it; the label is what triggers departure detection, so
    /// nothing ever revisits that row and the mirror keeps an
    /// owned/listed row for a name the wallet no longer holds, forever.
    /// Retrying costs one more pass; getting it wrong costs the row.
    ///
    /// A NON-transient persistence error (`Fatal` / `Constraint` /
    /// `LockPoisoned`) cannot be retried into success, so it must not
    /// park the departure queue — but it does not establish that no
    /// durable row exists, either. The error is HELD until the pass
    /// learns whether the id is actually needed: a Sold/Transferred
    /// departure never reads it and resolves normally, while either
    /// branch whose removal delta the id feeds — a confirmed-absent
    /// domain document, or a live one whose history never departs this
    /// identity (where the recovered id decides WHICH row is removed) —
    /// turns the held error into a
    /// terminal per-item failure ([`DepartureResolution::Failed`]) —
    /// the label and the durable row are preserved and the failure is
    /// surfaced on the sync summary, so the still-present label lets a
    /// later pass re-detect the departure and finish the removal once
    /// the backend is repaired, instead of the old degrade-to-`None`
    /// path resolving the departure with no removal delta and orphaning
    /// the persisted row for good.
    async fn resolve_departed_name(
        &self,
        identity_id: &Identifier,
        label: &str,
        previous_rows: &BTreeMap<Identifier, DpnsNameStateEntry>,
        now: u64,
    ) -> ResolvedDepartedName {
        let previous_document_id =
            match previous_document_id_for(&self.persister, identity_id, label, previous_rows) {
                Ok(document_id) => Ok(document_id),
                Err(error) if error.is_transient() => {
                    tracing::warn!(
                        identity = %identity_id,
                        name = label,
                        "persisted DPNS row lookup failed transiently for a departed name; \
                         retaining the departure for the next sync pass rather than \
                         resolving it without a removal delta: {error}"
                    );
                    return ResolvedDepartedName {
                        summary: DepartedDpnsName {
                            identity_id: *identity_id,
                            label: label.to_string(),
                            document_id: None,
                            status: None,
                        },
                        entry: None,
                        remove_document_id: None,
                        resolution: DepartureResolution::Retry,
                    };
                }
                // Non-retryable: HOLD the error instead of acting on it.
                // Whether it matters depends on what Platform says next —
                // the id is load-bearing only where it feeds the removal
                // delta (the confirmed-absent branch, and a live document
                // whose history never departs this identity), and failing
                // a Sold/Transferred departure (which never reads it)
                // over a broken read slot would be gratuitous.
                Err(error) => Err(error),
            };
        let state = match self.dpns_name_state(label).await {
            Ok(Some(state)) => state,
            Ok(None) => {
                let previous_document_id = match previous_document_id {
                    Ok(document_id) => document_id,
                    // Platform confirms the document is gone, but the
                    // persistence read failed non-retryably: whether a
                    // durable row exists — and under which id — is
                    // UNKNOWN. Resolving anyway would drop the label (the
                    // only trigger for future departure detection) while
                    // emitting no removal delta, orphaning any persisted
                    // row for good and reporting a successful sync over
                    // it. Fail this one departure instead: the label and
                    // the durable row survive, so the departure is
                    // re-detected on a later pass and completes once the
                    // backend is repaired.
                    Err(error) => {
                        tracing::warn!(
                            identity = %identity_id,
                            name = label,
                            "persisted DPNS row lookup failed unrecoverably for a departed \
                             name whose domain document is confirmed absent; preserving the \
                             label and any durable row rather than resolving the departure \
                             without a removal delta: {error}"
                        );
                        return ResolvedDepartedName {
                            summary: DepartedDpnsName {
                                identity_id: *identity_id,
                                label: label.to_string(),
                                document_id: None,
                                status: None,
                            },
                            entry: None,
                            remove_document_id: None,
                            resolution: DepartureResolution::Failed(error),
                        };
                    }
                };
                return ResolvedDepartedName {
                    summary: DepartedDpnsName {
                        identity_id: *identity_id,
                        label: label.to_string(),
                        document_id: previous_document_id,
                        status: None,
                    },
                    entry: None,
                    remove_document_id: previous_document_id,
                    resolution: DepartureResolution::Resolved,
                };
            }
            Err(error) => {
                tracing::warn!(
                    identity = %identity_id,
                    name = label,
                    "DPNS departed-name lookup failed; retaining local state for retry: {error}"
                );
                return ResolvedDepartedName {
                    summary: DepartedDpnsName {
                        identity_id: *identity_id,
                        label: label.to_string(),
                        // Informational only; a held non-transient
                        // persistence error reads as "id unknown" here.
                        document_id: previous_document_id.ok().flatten(),
                        status: None,
                    },
                    entry: None,
                    remove_document_id: None,
                    resolution: DepartureResolution::Retry,
                };
            }
        };
        let status = match self
            .classify_departure(&state.document_id, identity_id)
            .await
        {
            Ok(status) => status,
            Err(error) => {
                tracing::warn!(
                    identity = %identity_id,
                    name = label,
                    document = %state.document_id,
                    "DPNS departure-history classification failed; retaining local state for retry: {error}"
                );
                return ResolvedDepartedName {
                    summary: DepartedDpnsName {
                        identity_id: *identity_id,
                        label: label.to_string(),
                        document_id: Some(state.document_id),
                        status: None,
                    },
                    entry: None,
                    remove_document_id: None,
                    resolution: DepartureResolution::Retry,
                };
            }
        };
        let Some(sale_status) = status else {
            // No history event departs THIS identity from the live
            // document. Usually the live document IS the identity's own
            // row with its ownership rewritten out from under it, and
            // removing the live id retires the right row. But DPNS
            // domain documents are deletable, and a label can be
            // re-registered under a fresh document id: when the
            // RECOVERED prior id differs from the live document's, the
            // live document is that replacement — a document this
            // identity never held — while the identity's own durable
            // row still sits under the prior id. Removing the live id
            // would drop the wrong row AND orphan the durable one for
            // good, because the label this removal resolves is the only
            // trigger that would ever revisit it. Resolve the prior
            // incarnation instead: report and remove the recovered id,
            // and leave the replacement untouched.
            let departed_document_id = match previous_document_id {
                Ok(previous_id) => previous_id.unwrap_or(state.document_id),
                // The held non-retryable persistence error turns out to
                // be load-bearing: with a live, history-unrelated
                // document on the label, WHICH row departs depends on
                // the recovered id, so resolving without it would
                // either remove the replacement's document id or orphan
                // the identity's durable row. Fail this one departure
                // exactly like the confirmed-absent branch: the label
                // and the durable row survive, the failure is surfaced
                // on the summary, and a later pass finishes the removal
                // once the backend is repaired.
                Err(error) => {
                    tracing::warn!(
                        identity = %identity_id,
                        name = label,
                        document = %state.document_id,
                        "persisted DPNS row lookup failed unrecoverably for a departed \
                         name whose label carries a live, history-unrelated domain \
                         document; preserving the label and any durable row rather \
                         than guessing which row the removal delta targets: {error}"
                    );
                    return ResolvedDepartedName {
                        summary: DepartedDpnsName {
                            identity_id: *identity_id,
                            label: label.to_string(),
                            document_id: None,
                            status: None,
                        },
                        entry: None,
                        remove_document_id: None,
                        resolution: DepartureResolution::Failed(error),
                    };
                }
            };
            return ResolvedDepartedName {
                summary: DepartedDpnsName {
                    identity_id: *identity_id,
                    label: label.to_string(),
                    document_id: Some(departed_document_id),
                    status: None,
                },
                entry: None,
                remove_document_id: Some(departed_document_id),
                resolution: DepartureResolution::Resolved,
            };
        };
        ResolvedDepartedName {
            summary: DepartedDpnsName {
                identity_id: *identity_id,
                label: label.to_string(),
                document_id: Some(state.document_id),
                status: Some(sale_status),
            },
            entry: Some(state.to_entry(*identity_id, sale_status, now)),
            remove_document_id: None,
            resolution: DepartureResolution::Resolved,
        }
    }

    /// Find the latest history event whose *departing side* is the wallet
    /// identity. This is intentionally independent of the live domain's
    /// current owner: after S→A→B, S's departure must remain S→A.
    async fn classify_departure(
        &self,
        document_id: &Identifier,
        departing_identity: &Identifier,
    ) -> Result<Option<DpnsNameSaleStatus>, PlatformWalletError> {
        let mut candidates: Vec<(u64, DpnsNameSaleStatus)> = Vec::new();
        for document_type in [HISTORY_TYPE_PURCHASE, HISTORY_TYPE_TRANSFER] {
            let documents = self
                .fetch_history_documents(&dpns_contract_id(), document_id, document_type)
                .await?;
            for document in documents {
                match history_event_from_document(document_type, &document) {
                    Ok(event) => {
                        if let Some(candidate) =
                            direct_departure_candidate(event, departing_identity)
                        {
                            candidates.push(candidate);
                        }
                    }
                    Err(error) => tracing::warn!(
                        document = %document.id(),
                        history_type = document_type,
                        "ignoring malformed DPNS departure-history document: {error}"
                    ),
                }
            }
        }
        Ok(candidates
            .into_iter()
            .max_by_key(|(at_ms, _)| *at_ms)
            .map(|(_, status)| status))
    }
}

// ---------------------------------------------------------------------------
// History document decoding
// ---------------------------------------------------------------------------

/// Decode one Document History contract document into a timeline event.
/// Errors on missing/mistyped required fields rather than fabricating
/// values (`priceUpdate`/`purchase` must carry `price`, `transfer` must
/// carry `toIdentityId`, all must carry `$createdAt`).
fn history_event_from_document(
    doc_type: &str,
    doc: &Document,
) -> Result<DpnsNameHistoryEvent, PlatformWalletError> {
    let properties = doc.properties();
    let at_ms = doc.created_at().ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "history document {} ({doc_type}) is missing $createdAt",
            doc.id()
        ))
    })?;
    let price = || -> Result<Credits, PlatformWalletError> {
        properties
            .get_optional_integer::<Credits>("price")
            .ok()
            .flatten()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "history document {} ({doc_type}) is missing its price field",
                    doc.id()
                ))
            })
    };
    let identifier = |key: &str| -> Result<Identifier, PlatformWalletError> {
        properties
            .get(key)
            .and_then(|v| v.to_identifier().ok())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "history document {} ({doc_type}) is missing identifier field {key:?}",
                    doc.id()
                ))
            })
    };
    let kind = match doc_type {
        HISTORY_TYPE_PRICE_UPDATE => DpnsNameHistoryEventKind::PriceSet { price: price()? },
        HISTORY_TYPE_PURCHASE => DpnsNameHistoryEventKind::Purchased {
            price: price()?,
            seller: identifier("sellerId")?,
            buyer: doc.owner_id(),
        },
        HISTORY_TYPE_TRANSFER => DpnsNameHistoryEventKind::Transferred {
            from: doc.owner_id(),
            to: identifier("toIdentityId")?,
        },
        other => {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "unknown history document type {other:?}"
            )))
        }
    };
    Ok(DpnsNameHistoryEvent {
        kind,
        at_ms,
        block_height: doc.created_at_block_height(),
    })
}

/// The listing-side pre-flight of [`IdentityWallet::purchase_dpns_name`],
/// as a pure decision over the freshly fetched domain state.
///
/// Three typed rejections, and the ORDER is part of the contract the
/// method's API documentation promises:
///
/// 1. no `$price` at all → [`PlatformWalletError::DocumentNotForSale`];
/// 2. a `$price` of exactly 0 →
///    [`PlatformWalletError::InvalidParameter`]. Not a listing this
///    wallet will act on — see [`IdentityWallet::set_dpns_name_price`],
///    which refuses to create one. Checked BEFORE the `expected_price`
///    comparison so the caller is told the listing itself is not
///    purchasable, rather than being told the price moved (which would
///    invite a retry at 0 that can never succeed);
/// 3. anything else that differs from `expected_price` →
///    [`PlatformWalletError::DocumentPriceChanged`].
///
/// Only `== 0` is special-cased; every `> 0` listing takes the unchanged
/// price-match path. Extracted from the `async` method so the ordering
/// can be pinned directly, without a live Platform to serve the domain
/// fetch that precedes it.
fn preflight_purchase_price(
    state: &DpnsDomainState,
    name: &str,
    expected_price: Credits,
) -> Result<(), PlatformWalletError> {
    let listed_price = state.price.ok_or(PlatformWalletError::DocumentNotForSale {
        document_id: state.document_id,
    })?;
    if listed_price == 0 {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "DPNS name {name:?} carries a listed price of 0 credits, which is not a \
             valid sale listing and will not be purchased by this wallet"
        )));
    }
    if listed_price != expected_price {
        return Err(PlatformWalletError::DocumentPriceChanged {
            document_id: state.document_id,
            expected: expected_price,
            actual: listed_price,
        });
    }
    Ok(())
}

fn required_purchase_credits(expected_price: Credits) -> Result<Credits, PlatformWalletError> {
    expected_price
        .checked_add(DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS)
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(
                "DPNS purchase price is too large to reserve the document transition fee"
                    .to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name_state_entry(
        document_id: Identifier,
        wallet_identity_id: Identifier,
        status: DpnsNameSaleStatus,
    ) -> DpnsNameStateEntry {
        DpnsNameStateEntry {
            document_id,
            wallet_identity_id,
            label: "alice".to_string(),
            normalized_label: "a11ce".to_string(),
            normalized_parent_domain_name: DPNS_PARENT_DOMAIN.to_string(),
            price: None,
            status,
            created_at_ms: None,
            updated_at_ms: None,
            transferred_at_ms: None,
            last_synced_at_ms: 1,
        }
    }

    #[test]
    fn dpns_label_strips_only_the_dash_suffix() {
        assert_eq!(dpns_label("alice"), "alice");
        assert_eq!(dpns_label("alice.dash"), "alice");
        assert_eq!(dpns_label("alice.dash.dash"), "alice.dash");
    }

    #[test]
    fn fee_reserve_is_one_millidash() {
        // 0.001 DASH = 100_000 duffs? No: 1 DASH = 100_000_000 duffs, so
        // 0.001 DASH = 100_000 duffs = 100_000_000 credits (1 duff =
        // 1000 credits). Pin the constant against unit drift.
        assert_eq!(DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS, 100_000 * 1_000);
    }

    #[test]
    fn purchase_credit_requirement_rejects_overflow() {
        assert_eq!(
            required_purchase_credits(1).expect("small price should fit"),
            DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS + 1
        );

        assert!(matches!(
            required_purchase_credits(u64::MAX),
            Err(PlatformWalletError::InvalidParameter(message))
                if message.contains("too large")
        ));
    }

    #[test]
    fn owned_sync_row_wins_regardless_of_identity_iteration_order() {
        let document_id = Identifier::from([1; 32]);
        let seller_id = Identifier::from([2; 32]);
        let buyer_id = Identifier::from([3; 32]);
        let sold = name_state_entry(
            document_id,
            seller_id,
            DpnsNameSaleStatus::Sold { to: buyer_id },
        );
        let owned = name_state_entry(document_id, buyer_id, DpnsNameSaleStatus::Owned);

        for entries in [[sold.clone(), owned.clone()], [owned.clone(), sold.clone()]] {
            let mut rows = BTreeMap::new();
            for entry in entries {
                insert_sync_row(&mut rows, entry);
            }
            let row = rows.get(&document_id).expect("document row");
            assert_eq!(row.wallet_identity_id, buyer_id);
            assert_eq!(row.status, DpnsNameSaleStatus::Owned);
        }
    }

    #[test]
    fn departure_attribution_ignores_later_multi_hop_owner() {
        let seller = Identifier::from([1; 32]);
        let first_buyer = Identifier::from([2; 32]);
        let later_buyer = Identifier::from([3; 32]);
        let seller_departure = DpnsNameHistoryEvent {
            kind: DpnsNameHistoryEventKind::Purchased {
                price: 10,
                seller,
                buyer: first_buyer,
            },
            at_ms: 100,
            block_height: None,
        };
        let later_transfer = DpnsNameHistoryEvent {
            kind: DpnsNameHistoryEventKind::Transferred {
                from: first_buyer,
                to: later_buyer,
            },
            at_ms: 200,
            block_height: None,
        };

        assert_eq!(
            direct_departure_candidate(seller_departure, &seller),
            Some((100, DpnsNameSaleStatus::Sold { to: first_buyer }))
        );
        assert_eq!(direct_departure_candidate(later_transfer, &seller), None);
    }

    // -----------------------------------------------------------------
    // Departed-name document-id recovery
    //
    // The bug these cover: `info.dpns_name_states` is session-scoped and
    // starts EMPTY on every process start (the load path builds it that
    // way and nothing rehydrates it). A name that departs during the
    // FIRST sync pass after a restart therefore had no in-memory row to
    // resolve its `document_id` from — so the pass emitted no removal
    // delta while still dropping the label, and the host's persisted
    // mirror kept an owned/listed row for a name the wallet no longer
    // holds, with nothing left to ever trigger its removal.
    // -----------------------------------------------------------------

    use crate::changeset::{
        ClientStartState, PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::WalletId;

    const MIRROR_WALLET_ID: WalletId = [0x7A; 32];
    /// Display label whose homograph normalization is visibly different
    /// ("Alice" → "a11ce"), so a reader that forgot to normalize — or
    /// normalized the wrong string — cannot pass by accident.
    const DEPARTED_LABEL: &str = "Alice";

    /// Stand-in for the durable host mirror (Swift `PersistentDPNSName`,
    /// the Android Room `dpns_names` table, the SQLite
    /// `dpns_name_states` table) that survives a process restart.
    ///
    /// Answers `get_dpns_name_state` from a hydrated map keyed exactly
    /// as the trait contract specifies — `(wallet_identity_id,
    /// normalized_label)` — and records every lookup, so a test can
    /// assert both whether the fallback was consulted and what key it
    /// was consulted with.
    struct MirrorPersister {
        rows: BTreeMap<(Identifier, String), DpnsNameStateEntry>,
        lookups: std::sync::Mutex<Vec<(WalletId, Identifier, String)>>,
        /// `Some(kind)` makes every read fail with that retry
        /// classification; [`Self::heal`] clears it so a later pass sees
        /// a working backend, which is how the retry arm is proven to
        /// actually make progress rather than merely defer forever.
        fail: std::sync::Mutex<Option<PersistenceErrorKind>>,
        /// Every DPNS name-state changeset handed to [`Self::store`], in
        /// order — the row deltas a real host would apply to its durable
        /// mirror. Lets a test assert not merely what a sync summary
        /// CLAIMS but what actually reached the persistence boundary.
        stored_dpns: std::sync::Mutex<Vec<DpnsNameStateChangeSet>>,
    }

    impl MirrorPersister {
        fn hydrated(rows: Vec<DpnsNameStateEntry>) -> Self {
            Self {
                rows: rows
                    .into_iter()
                    .map(|entry| {
                        (
                            (entry.wallet_identity_id, entry.normalized_label.clone()),
                            entry,
                        )
                    })
                    .collect(),
                lookups: std::sync::Mutex::new(Vec::new()),
                fail: std::sync::Mutex::new(None),
                stored_dpns: std::sync::Mutex::new(Vec::new()),
            }
        }

        /// Holds `rows`, but every read fails with `kind` until
        /// [`Self::heal`] is called.
        fn hydrated_but_failing(rows: Vec<DpnsNameStateEntry>, kind: PersistenceErrorKind) -> Self {
            let mirror = Self::hydrated(rows);
            *mirror.fail.lock().expect("fail switch") = Some(kind);
            mirror
        }

        fn failing_with(kind: PersistenceErrorKind) -> Self {
            Self::hydrated_but_failing(Vec::new(), kind)
        }

        /// The backend recovers: subsequent reads answer from `rows`.
        fn heal(&self) {
            *self.fail.lock().expect("fail switch") = None;
        }

        fn lookups(&self) -> Vec<(WalletId, Identifier, String)> {
            self.lookups.lock().expect("lookup log").clone()
        }

        /// Every document id a stored DPNS name-state delta removed, in
        /// store order.
        fn stored_dpns_removals(&self) -> Vec<Identifier> {
            self.stored_dpns
                .lock()
                .expect("stored log")
                .iter()
                .flat_map(|cs| cs.removed.iter().copied())
                .collect()
        }
    }

    impl PlatformWalletPersistence for MirrorPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if let Some(cs) = changeset.dpns_name_states {
                self.stored_dpns.lock().expect("stored log").push(cs);
            }
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }

        fn get_dpns_name_state(
            &self,
            wallet_id: WalletId,
            wallet_identity_id: &Identifier,
            normalized_label: &str,
        ) -> Result<Option<DpnsNameStateEntry>, PersistenceError> {
            self.lookups.lock().expect("lookup log").push((
                wallet_id,
                *wallet_identity_id,
                normalized_label.to_string(),
            ));
            if let Some(kind) = *self.fail.lock().expect("fail switch") {
                return Err(PersistenceError::backend_with_kind(
                    kind,
                    "simulated mirror read failure",
                ));
            }
            Ok(self
                .rows
                .get(&(*wallet_identity_id, normalized_label.to_string()))
                .cloned())
        }
    }

    /// A persisted row for `DEPARTED_LABEL` owned by `identity_id`, as the
    /// host mirror would hold it after a previous session's sync pass.
    fn mirrored_row(document_id: Identifier, identity_id: Identifier) -> DpnsNameStateEntry {
        let mut entry = name_state_entry(document_id, identity_id, DpnsNameSaleStatus::Owned);
        entry.label = DEPARTED_LABEL.to_string();
        entry.normalized_label = convert_to_homograph_safe_chars(DEPARTED_LABEL);
        entry
    }

    fn mirror_wallet_persister(mirror: Arc<MirrorPersister>) -> WalletPersister {
        WalletPersister::new(MIRROR_WALLET_ID, mirror)
    }

    /// Sanity-check the fixture itself: if normalization were a no-op for
    /// `DEPARTED_LABEL`, the "looked the row up by its NORMALIZED label"
    /// assertion below would hold vacuously.
    #[test]
    fn departed_label_fixture_actually_normalizes() {
        assert_eq!(convert_to_homograph_safe_chars(DEPARTED_LABEL), "a11ce");
        assert_ne!(
            convert_to_homograph_safe_chars(DEPARTED_LABEL),
            DEPARTED_LABEL
        );
    }

    /// Steady state (any pass after the first): the in-memory snapshot has
    /// the row, so the persister is never touched. Pins that the fallback
    /// is a fallback — not an extra read on the hot path.
    #[test]
    fn departed_document_id_uses_the_in_memory_row_without_reading_the_persister() {
        let document_id = Identifier::from([0x11; 32]);
        let identity_id = Identifier::from([0x22; 32]);
        let row = mirrored_row(document_id, identity_id);

        // The mirror also holds a row — for a DIFFERENT document — so a
        // wrong-source regression would return the wrong id, not None.
        let mirror = Arc::new(MirrorPersister::hydrated(vec![mirrored_row(
            Identifier::from([0xEE; 32]),
            identity_id,
        )]));
        let persister = mirror_wallet_persister(Arc::clone(&mirror));

        let mut previous_rows = BTreeMap::new();
        previous_rows.insert(document_id, row);

        assert_eq!(
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &previous_rows)
                .expect("the in-memory hit must not consult the persister at all"),
            Some(document_id)
        );
        assert!(
            mirror.lookups().is_empty(),
            "a populated in-memory snapshot must not trigger a persistence read"
        );
    }

    /// `previous_rows` is keyed by document id and retains
    /// `Sold`/`Transferred` history, so one identity can hold a
    /// historical row AND the current `Owned` row under the same
    /// normalized label (delete + re-register). The in-memory selection
    /// must prefer the CURRENT row with the same deterministic ordering
    /// the persistence contract demands of backends — `Owned` first —
    /// in BOTH document-id orders: a first-match scan in map order
    /// returns whichever row sorts first, and picking the historical
    /// one makes recovery remove it, drop the identity's label, and
    /// orphan the actual current row for good.
    #[test]
    fn departed_document_id_prefers_the_current_owned_row_in_the_snapshot() {
        let identity_id = Identifier::from([0x22; 32]);
        let buyer = Identifier::from([0x99; 32]);

        for (historical_doc, owned_doc) in [
            // Historical row FIRST in map order: the first-match scan
            // returns it — this is the orphaning bug.
            (Identifier::from([0x01; 32]), Identifier::from([0x02; 32])),
            // And the reverse, so passing by iteration luck is impossible.
            (Identifier::from([0x03; 32]), Identifier::from([0x02; 32])),
        ] {
            let mut historical = mirrored_row(historical_doc, identity_id);
            historical.status = DpnsNameSaleStatus::Sold { to: buyer };
            // The historical row is deliberately FRESHER: `Owned` must
            // outrank recency, exactly as in the backends' ordering.
            historical.last_synced_at_ms = 2_000;
            let mut owned = mirrored_row(owned_doc, identity_id);
            owned.last_synced_at_ms = 1_000;
            // Decoy for another identity, "better" on every tie-break —
            // the identity filter must exclude it outright.
            let mut decoy =
                mirrored_row(Identifier::from([0xFE; 32]), Identifier::from([0xFD; 32]));
            decoy.last_synced_at_ms = 9_000;

            let mut previous_rows = BTreeMap::new();
            for entry in [historical, owned, decoy] {
                previous_rows.insert(entry.document_id, entry);
            }

            // A failing mirror proves the in-memory hit still
            // short-circuits: consulting the persister here would error.
            let persister = mirror_wallet_persister(Arc::new(MirrorPersister::failing_with(
                PersistenceErrorKind::Fatal,
            )));
            assert_eq!(
                previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &previous_rows)
                    .expect("an in-memory hit must not consult the persister"),
                Some(owned_doc),
                "the current Owned row must win over retained history regardless \
                 of document-id order"
            );
        }
    }

    /// With no `Owned` row in the snapshot (both matches are retained
    /// history), the tie-breaks mirror the persistence contract:
    /// greatest `last_synced_at_ms` first, then greatest `document_id`.
    #[test]
    fn departed_document_id_breaks_snapshot_ties_like_the_persistence_contract() {
        let identity_id = Identifier::from([0x23; 32]);
        let buyer = Identifier::from([0x99; 32]);
        let persister = mirror_wallet_persister(Arc::new(MirrorPersister::failing_with(
            PersistenceErrorKind::Fatal,
        )));

        // Freshness decides between two historical rows. The fresher row
        // gets the SMALLER document id, so a map-order or id-order scan
        // cannot pass by accident.
        let mut stale = mirrored_row(Identifier::from([0x0A; 32]), identity_id);
        stale.status = DpnsNameSaleStatus::Sold { to: buyer };
        stale.last_synced_at_ms = 1_000;
        let mut fresh = mirrored_row(Identifier::from([0x09; 32]), identity_id);
        fresh.status = DpnsNameSaleStatus::Transferred { to: buyer };
        fresh.last_synced_at_ms = 2_000;
        let fresh_doc = fresh.document_id;
        let mut rows = BTreeMap::new();
        for entry in [stale.clone(), fresh] {
            rows.insert(entry.document_id, entry);
        }
        assert_eq!(
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &rows)
                .expect("an in-memory hit must not consult the persister"),
            Some(fresh_doc),
            "the fresher retained row must win"
        );

        // An exact freshness tie falls to the greatest document id.
        let mut twin = mirrored_row(Identifier::from([0x0B; 32]), identity_id);
        twin.status = DpnsNameSaleStatus::Sold { to: buyer };
        twin.last_synced_at_ms = 1_000;
        let twin_doc = twin.document_id;
        let mut rows = BTreeMap::new();
        for entry in [stale, twin] {
            rows.insert(entry.document_id, entry);
        }
        assert_eq!(
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &rows)
                .expect("an in-memory hit must not consult the persister"),
            Some(twin_doc),
            "an exact freshness tie must fall to the greatest document id"
        );
    }

    /// THE REGRESSION. First pass after a process restart: the in-memory
    /// snapshot is empty (exactly what the load path produces) but the
    /// durable mirror still holds the row, so the departure recovers the
    /// `document_id` its removal delta needs. Before the fix this
    /// returned `None` and the mirror row was orphaned forever.
    #[test]
    fn departed_document_id_falls_back_to_the_persisted_row_after_a_restart() {
        let document_id = Identifier::from([0x33; 32]);
        let identity_id = Identifier::from([0x44; 32]);
        let mirror = Arc::new(MirrorPersister::hydrated(vec![mirrored_row(
            document_id,
            identity_id,
        )]));
        let persister = mirror_wallet_persister(Arc::clone(&mirror));

        // `BTreeMap::new()` IS the post-restart state: see the wallet load
        // path, which initializes `dpns_name_states` empty.
        let previous_rows = BTreeMap::new();

        assert_eq!(
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &previous_rows)
                .expect("a healthy mirror read must succeed"),
            Some(document_id),
            "an empty in-memory snapshot must fall back to the durable mirror"
        );
        assert_eq!(
            mirror.lookups(),
            vec![(
                MIRROR_WALLET_ID,
                identity_id,
                convert_to_homograph_safe_chars(DEPARTED_LABEL)
            )],
            "the mirror must be queried once, scoped to this wallet and identity, \
             keyed by the NORMALIZED label"
        );
    }

    /// The mirror is asked for this identity's row specifically. A
    /// `Sold`/`Transferred` row is retained after a name leaves, so a
    /// lookup that dropped the identity scope could remove another
    /// identity's document.
    #[test]
    fn departed_document_id_does_not_return_another_identitys_row() {
        let identity_id = Identifier::from([0x55; 32]);
        let other_identity_id = Identifier::from([0x66; 32]);
        let mirror = Arc::new(MirrorPersister::hydrated(vec![mirrored_row(
            Identifier::from([0x77; 32]),
            other_identity_id,
        )]));
        let persister = mirror_wallet_persister(Arc::clone(&mirror));

        assert_eq!(
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &BTreeMap::new())
                .expect("a healthy mirror read must succeed"),
            None
        );
        assert_eq!(
            mirror.lookups().first().map(|(_, id, _)| *id),
            Some(identity_id),
            "the lookup must carry the departing identity, not any row's identity"
        );
    }

    /// A backend that cannot answer (the `Ok(None)` default, e.g.
    /// `NoPlatformPersistence` or an unwired FFI vtable) degrades to the
    /// pre-fix behaviour rather than failing the departure.
    #[test]
    fn departed_document_id_is_none_when_the_backend_does_not_index_dpns_rows() {
        let persister = WalletPersister::new(
            MIRROR_WALLET_ID,
            Arc::new(crate::wallet::persister::NoPlatformPersistence),
        );
        assert_eq!(
            previous_document_id_for(
                &persister,
                &Identifier::from([0x88; 32]),
                DEPARTED_LABEL,
                &BTreeMap::new()
            )
            .expect("the Ok(None) default is not an error"),
            None
        );
    }

    /// A persistence read FAILURE must stay distinguishable from
    /// `Ok(None)`. `Ok(None)` means "the backend answered and has no
    /// better id"; `Err` means "we do not know". Flattening the two here
    /// is what let a failed read reach the departure path as a confirmed
    /// absence — see
    /// [`resolve_departed_name_retains_the_departure_when_the_persister_read_fails`].
    #[test]
    fn departed_document_id_surfaces_the_error_instead_of_flattening_it_to_none() {
        let mirror = Arc::new(MirrorPersister::failing_with(
            PersistenceErrorKind::Transient,
        ));
        let persister = mirror_wallet_persister(Arc::clone(&mirror));

        let error = previous_document_id_for(
            &persister,
            &Identifier::from([0x99; 32]),
            DEPARTED_LABEL,
            &BTreeMap::new(),
        )
        .expect_err("a failed read must not read as an empty mirror");
        assert!(
            error.is_transient(),
            "the backend's retry classification must survive the hop: {error}"
        );
        assert_eq!(
            mirror.lookups().len(),
            1,
            "the failing read must have actually been attempted"
        );
    }

    /// End-to-end through the real `resolve_departed_name`, proving the
    /// fallback is wired into the production path and not just reachable
    /// as a standalone helper.
    ///
    /// The SDK is a mock with no expectations, so the domain-document
    /// lookup fails and resolution takes its retry arm — the one arm
    /// reachable without a live Platform. That arm still reports the
    /// departed name's `document_id`, which is sourced from exactly the
    /// same resolution the removal delta uses, so a regression that
    /// unwired the persister fallback fails here too.
    #[tokio::test]
    async fn resolve_departed_name_recovers_the_document_id_from_the_persister() {
        let document_id = Identifier::from([0xAB; 32]);
        let identity_id = Identifier::from([0xCD; 32]);
        let mirror = Arc::new(MirrorPersister::hydrated(vec![mirrored_row(
            document_id,
            identity_id,
        )]));
        let wallet = mirror_backed_identity_wallet(Arc::clone(&mirror));

        // Post-restart in-memory state: empty.
        let resolved = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;

        assert_eq!(
            resolved.summary.document_id,
            Some(document_id),
            "resolve_departed_name must resolve the departed name's document id \
             through the persister when the in-memory snapshot is empty"
        );
        assert_eq!(
            mirror.lookups(),
            vec![(
                wallet.wallet_id,
                identity_id,
                convert_to_homograph_safe_chars(DEPARTED_LABEL)
            )]
        );
        assert!(
            matches!(resolved.resolution, DepartureResolution::Retry),
            "test precondition: the mock SDK has no expectations, so the domain \
             lookup must fail and request a retry"
        );
    }

    /// THE ROUND-3 REGRESSION for the departure path. Platform CONFIRMS
    /// the name is gone (the mock answers the domain query with an empty
    /// document set), which is the branch that resolves the departure,
    /// drops the identity's label, and emits the removal delta. When the
    /// persistence lookup for the `document_id` FAILED rather than
    /// answering "no row", the old code could not tell the two apart:
    /// resolution carried on with no id, the label — the only trigger
    /// for future departure detection — was removed, and the durable row
    /// was orphaned for good.
    ///
    /// The first assertion block establishes that this mock really does
    /// take the confirmed-absent branch, so the retention assertion that
    /// follows cannot pass by accident.
    #[tokio::test]
    async fn resolve_departed_name_retains_the_departure_when_the_persistence_read_fails() {
        let document_id = Identifier::from([0xA1; 32]);
        let identity_id = Identifier::from([0xA2; 32]);

        // Control: healthy mirror, Platform confirms absence. The
        // departure RESOLVES and carries its removal delta.
        let healthy = Arc::new(MirrorPersister::hydrated(vec![mirrored_row(
            document_id,
            identity_id,
        )]));
        let control = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&healthy),
            sdk_with_absent_dpns_domain(DEPARTED_LABEL).await,
        );
        let resolved = control
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;
        assert!(
            matches!(resolved.resolution, DepartureResolution::Resolved),
            "test precondition: with the mock answering 'no such document', the \
             departure must take the confirmed-absent branch, not a retry arm"
        );
        assert_eq!(
            resolved.remove_document_id,
            Some(document_id),
            "test precondition: the confirmed-absent branch emits the removal delta"
        );

        // Same Platform answer, but the mirror read fails transiently.
        // The departure must be RETAINED instead of resolved: `retry`
        // makes the sync loop push it back on the queue and break before
        // it can call `remove_dpns_label`, so the label, the queue entry
        // and the durable row all survive to the next pass.
        let failing = Arc::new(MirrorPersister::hydrated_but_failing(
            vec![mirrored_row(document_id, identity_id)],
            PersistenceErrorKind::Transient,
        ));
        let wallet = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&failing),
            sdk_with_absent_dpns_domain(DEPARTED_LABEL).await,
        );
        let retained = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;
        assert!(
            matches!(retained.resolution, DepartureResolution::Retry),
            "a transiently failed persistence read must retain the departure for \
             the next pass"
        );
        assert_eq!(
            retained.remove_document_id, None,
            "nothing may be removed while the document id is UNKNOWN"
        );
        assert!(retained.entry.is_none());
        assert_eq!(retained.summary.status, None);
        assert_eq!(
            retained.summary.document_id, None,
            "the summary must not claim an id the lookup never produced"
        );

        // Next pass, backend recovered: the retained departure resolves
        // normally and finally carries its removal delta.
        failing.heal();
        let healed = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;
        assert!(
            matches!(healed.resolution, DepartureResolution::Resolved),
            "a healed backend must let the departure resolve"
        );
        assert_eq!(healed.remove_document_id, Some(document_id));
        assert_eq!(healed.summary.document_id, Some(document_id));
    }

    /// THE ROUND-4 REGRESSION. A NON-retryable persistence error
    /// (`Fatal` / `Constraint` / `LockPoisoned`) cannot be retried into
    /// success, so it must not park this identity's departure queue —
    /// but it does not establish that no durable row exists, either.
    /// The old arm degraded it to "no previous id" and carried on; with
    /// Platform confirming the document absent, that RESOLVED the
    /// departure, dropped the label — the only trigger for future
    /// departure detection — and reported a successful sync, leaving
    /// any persisted row orphaned for good. It must instead be a
    /// terminal per-item FAILURE: no retry, no label drop, no deltas.
    #[tokio::test]
    async fn resolve_departed_name_fails_terminally_when_the_persistence_error_is_not_retryable() {
        let document_id = Identifier::from([0xA4; 32]);
        let identity_id = Identifier::from([0xA3; 32]);
        let mirror = Arc::new(MirrorPersister::hydrated_but_failing(
            vec![mirrored_row(document_id, identity_id)],
            PersistenceErrorKind::Fatal,
        ));
        let wallet = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&mirror),
            sdk_with_absent_dpns_domain(DEPARTED_LABEL).await,
        );

        let resolved = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;

        match &resolved.resolution {
            DepartureResolution::Failed(error) => assert!(
                !error.is_transient(),
                "the terminal failure must carry the non-retryable error: {error}"
            ),
            other => panic!(
                "an unrecoverable read under a confirmed-absent document must be a \
                 terminal per-item failure — not a retry (which would park the queue \
                 for the life of the process) and not a resolution (which would \
                 orphan the durable row), got {other:?}"
            ),
        }
        assert_eq!(
            resolved.remove_document_id, None,
            "nothing may be removed while the document id is unknown"
        );
        assert!(resolved.entry.is_none());
        assert_eq!(
            resolved.summary.document_id, None,
            "the summary must not claim an id the lookup never produced"
        );

        // Once the backend is repaired, the SAME departure — re-detected
        // through the label the failure preserved — resolves normally and
        // finally carries its removal delta.
        mirror.heal();
        let healed = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;
        assert!(
            matches!(healed.resolution, DepartureResolution::Resolved),
            "a repaired backend must let the preserved departure resolve"
        );
        assert_eq!(healed.remove_document_id, Some(document_id));
        assert_eq!(healed.summary.document_id, Some(document_id));
    }

    /// The held non-retryable error must not fail departures that never
    /// need the persisted id: with the domain document still PRESENT,
    /// resolution proceeds past the confirmed-absent branch into history
    /// classification (whose fetch fails on this mock and requests a
    /// plain retry). A regression that failed the departure eagerly —
    /// before knowing whether the id is needed — would turn every
    /// Sold/Transferred departure on a degraded backend into a permanent
    /// failure.
    #[tokio::test]
    async fn resolve_departed_name_defers_a_fatal_persistence_error_until_the_id_is_needed() {
        let document_id = Identifier::from([0xA5; 32]);
        let identity_id = Identifier::from([0xA6; 32]);
        let new_owner = Identifier::from([0xA7; 32]);
        let mirror = Arc::new(MirrorPersister::hydrated_but_failing(
            vec![mirrored_row(document_id, identity_id)],
            PersistenceErrorKind::Fatal,
        ));
        let mut documents = dash_sdk::query_types::Documents::new();
        documents.insert(
            document_id,
            Some(listed_domain_document(document_id, new_owner, None)),
        );
        let wallet = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&mirror),
            sdk_answering_dpns_domain_query(DEPARTED_LABEL, documents).await,
        );

        let resolved = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;

        assert!(
            matches!(resolved.resolution, DepartureResolution::Retry),
            "with the document still present the fatal persistence error is not yet \
             load-bearing; the history-classification fetch failure must yield an \
             ordinary retry, got {:?}",
            resolved.resolution
        );
        assert_eq!(
            resolved.summary.document_id,
            Some(document_id),
            "the id comes from the live document, not the failed persistence read"
        );
    }

    /// THE ROUND-5 REGRESSION. DPNS domain documents are deletable, and
    /// a label can be re-registered under a fresh document id: persisted
    /// document A (the identity's own row) was deleted and an unrelated
    /// identity registered document B under the same normalized label.
    /// The domain query answers with B, whose history never departs the
    /// wallet identity, so classification yields no sale status. The old
    /// code then reported and removed B — the replacement owner's
    /// document, never a row of this departure — while the identity's
    /// durable row A survived with no label left to ever trigger its
    /// reconciliation. The removal delta must target the RECOVERED prior
    /// incarnation A and leave the replacement B untouched.
    #[tokio::test]
    async fn resolve_departed_name_removes_the_prior_incarnation_when_the_label_was_re_registered()
    {
        let prior_document_id = Identifier::from([0xC1; 32]);
        let identity_id = Identifier::from([0xC2; 32]);
        let replacement_document_id = Identifier::from([0xC3; 32]);
        let replacement_owner = Identifier::from([0xC4; 32]);

        let mirror = Arc::new(MirrorPersister::hydrated(vec![mirrored_row(
            prior_document_id,
            identity_id,
        )]));
        let mut documents = dash_sdk::query_types::Documents::new();
        documents.insert(
            replacement_document_id,
            Some(listed_domain_document(
                replacement_document_id,
                replacement_owner,
                None,
            )),
        );
        let wallet = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&mirror),
            sdk_with_history_unrelated_dpns_domain(
                DEPARTED_LABEL,
                documents,
                replacement_document_id,
            )
            .await,
        );

        // Post-restart in-memory state: empty, so the prior incarnation
        // is recovered through the persister — the restart shape in which
        // the orphan was originally reported.
        let resolved = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;

        assert!(
            matches!(resolved.resolution, DepartureResolution::Resolved),
            "test precondition: with the domain and history lookups primed and the \
             mirror healthy, the departure must resolve, got {:?}",
            resolved.resolution
        );
        assert_eq!(
            resolved.remove_document_id,
            Some(prior_document_id),
            "the removal delta must target the identity's recovered prior \
             incarnation, not the re-registered replacement"
        );
        assert_eq!(
            resolved.summary.document_id,
            Some(prior_document_id),
            "the departed document is the prior incarnation, not the replacement"
        );
        assert_eq!(
            resolved.summary.status, None,
            "a deleted-and-re-registered name departs without a sale"
        );
        assert!(
            resolved.entry.is_none(),
            "the replacement belongs to an unrelated identity — no row may be \
             written for it"
        );
    }

    /// The companion failure arm of the round-5 regression: with a live,
    /// history-unrelated document on the label, the recovered prior id
    /// decides WHICH row the removal delta targets, so the held
    /// non-retryable persistence error is load-bearing here exactly as it
    /// is under a confirmed-absent document. Resolving anyway would
    /// either remove the replacement's document id or orphan the
    /// identity's durable row; the departure must fail terminally,
    /// preserving the label, and complete once the backend is repaired.
    #[tokio::test]
    async fn resolve_departed_name_fails_terminally_when_a_re_registered_label_needs_the_failed_lookup(
    ) {
        let prior_document_id = Identifier::from([0xC5; 32]);
        let identity_id = Identifier::from([0xC6; 32]);
        let replacement_document_id = Identifier::from([0xC7; 32]);
        let replacement_owner = Identifier::from([0xC8; 32]);

        let mirror = Arc::new(MirrorPersister::hydrated_but_failing(
            vec![mirrored_row(prior_document_id, identity_id)],
            PersistenceErrorKind::Fatal,
        ));
        let mut documents = dash_sdk::query_types::Documents::new();
        documents.insert(
            replacement_document_id,
            Some(listed_domain_document(
                replacement_document_id,
                replacement_owner,
                None,
            )),
        );
        let wallet = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&mirror),
            sdk_with_history_unrelated_dpns_domain(
                DEPARTED_LABEL,
                documents,
                replacement_document_id,
            )
            .await,
        );

        let resolved = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;

        match &resolved.resolution {
            DepartureResolution::Failed(error) => assert!(
                !error.is_transient(),
                "the terminal failure must carry the non-retryable error: {error}"
            ),
            other => panic!(
                "an unrecoverable read under a live, history-unrelated document \
                 must be a terminal per-item failure — resolving would remove the \
                 wrong row or orphan the durable one, got {other:?}"
            ),
        }
        assert_eq!(
            resolved.remove_document_id, None,
            "nothing may be removed while WHICH row departs is unknown"
        );
        assert!(resolved.entry.is_none());
        assert_eq!(
            resolved.summary.document_id, None,
            "the summary must not claim an id the lookup never produced"
        );

        // Backend repaired: the SAME departure — re-detected through the
        // label the failure preserved — resolves against the prior
        // incarnation and leaves the replacement untouched.
        mirror.heal();
        let healed = wallet
            .resolve_departed_name(&identity_id, DEPARTED_LABEL, &BTreeMap::new(), 1_000)
            .await;
        assert!(
            matches!(healed.resolution, DepartureResolution::Resolved),
            "a repaired backend must let the preserved departure resolve"
        );
        assert_eq!(healed.remove_document_id, Some(prior_document_id));
        assert_eq!(healed.summary.document_id, Some(prior_document_id));
    }

    /// The labels `identity_id` currently carries in the wallet manager —
    /// the departure trigger the Failed arm must preserve.
    async fn dpns_labels(wallet: &IdentityWallet, identity_id: &Identifier) -> Vec<String> {
        let wm = wallet.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet.wallet_id).expect("wallet info");
        info.identity_manager
            .wallet_identity(&wallet.wallet_id, identity_id)
            .expect("managed identity")
            .dpns_names
            .iter()
            .map(|name| name.label.clone())
            .collect()
    }

    /// [`DepartureResolution::Failed`] as the SYNC LOOP consumes it —
    /// the load-bearing caller branch the resolver-level tests above
    /// cannot reach. A managed identity still carries [`DEPARTED_LABEL`],
    /// Platform confirms the domain document absent, and the mirror read
    /// fails fatally. The pass must surface the failure on
    /// `departures_failed` while leaving EVERYTHING else untouched: were
    /// the arm to regress to the old degrade-to-`None` behavior, the pass
    /// would instead report a successful departure with no document id,
    /// drop the label (the only re-detection trigger), and orphan the
    /// mirror's durable row for good — every assertion below fails on
    /// that regression. The healed second pass then proves the retained
    /// label really does let a later pass finish the removal, so the
    /// terminal failure neither parks the queue nor loses the departure.
    #[tokio::test]
    async fn sync_pass_surfaces_a_terminal_departure_failure_and_completes_it_once_healed() {
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::Identity;

        let document_id = Identifier::from([0xB1; 32]);
        let identity_id = Identifier::from([0xB2; 32]);
        let mirror = Arc::new(MirrorPersister::hydrated_but_failing(
            vec![mirrored_row(document_id, identity_id)],
            PersistenceErrorKind::Fatal,
        ));
        let wallet = mirror_backed_identity_wallet_with_sdk(
            Arc::clone(&mirror),
            sdk_for_departed_identity_sync(&identity_id, DEPARTED_LABEL).await,
        );

        // The wallet still holds the identity AND its label; Platform
        // (the mock) no longer shows the identity owning any document.
        {
            let mut wm = wallet.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet.wallet_id)
                .expect("wallet info");
            info.identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id: identity_id,
                        public_keys: BTreeMap::new(),
                        balance: 0,
                        revision: 0,
                    }),
                    0,
                    wallet.wallet_id,
                    &wallet.persister,
                )
                .expect("add identity");
            info.identity_manager
                .wallet_identity_mut(&wallet.wallet_id, &identity_id)
                .expect("managed identity")
                .dpns_names
                .push(DpnsNameInfo {
                    label: DEPARTED_LABEL.to_string(),
                    acquired_at: Some(500),
                });
        }

        let summary = wallet
            .sync_dpns_marketplace()
            .await
            .expect("a terminal PER-ITEM failure must not fail the pass");

        // Surfaced on the summary, not silently swallowed...
        assert_eq!(
            summary.departures_failed.len(),
            1,
            "the fatal mirror read under a confirmed-absent document must land \
             in departures_failed, got {:?}",
            summary.departures_failed
        );
        let failure = &summary.departures_failed[0];
        assert_eq!(failure.identity_id, identity_id);
        assert_eq!(failure.label, DEPARTED_LABEL);
        assert!(
            failure.error.contains("simulated mirror read failure"),
            "the summary must carry the underlying persistence error, got: {}",
            failure.error
        );

        // ...and NOT reported as a successful departure or any other delta.
        assert!(
            summary.names_departed.is_empty(),
            "a failed departure must not appear in names_departed: {:?}",
            summary.names_departed
        );
        assert!(
            summary.is_empty_delta(),
            "the failed pass must apply no adds, departures or price changes"
        );
        assert_eq!(summary.names_tracked, 0);

        // The label survives — it is the only trigger for re-detection.
        assert_eq!(
            dpns_labels(&wallet, &identity_id).await,
            vec![DEPARTED_LABEL.to_string()],
            "the failed departure must leave the identity's label in place"
        );

        // No row delta reached the durable mirror.
        assert_eq!(
            mirror.stored_dpns_removals(),
            Vec::<Identifier>::new(),
            "nothing may be removed from the mirror while the document id is unknown"
        );

        // The queue is not parked: the failed item was consumed, not
        // requeued, so the identity carries no pending sync progress and
        // the next pass starts from a clean scan (which re-detects the
        // departure from the retained label).
        assert!(
            wallet
                .dpns_sync_progress
                .lock()
                .expect("progress lock")
                .get(&identity_id)
                .is_none(),
            "a terminal failure must not park the departure queue"
        );

        // Backend repaired: the SAME departure — re-detected through the
        // preserved label — now resolves, drops the label, and finally
        // emits the removal delta for the mirror's row.
        mirror.heal();
        let healed = wallet
            .sync_dpns_marketplace()
            .await
            .expect("healed pass must succeed");
        assert!(
            healed.departures_failed.is_empty(),
            "no failure may remain once the backend answers: {:?}",
            healed.departures_failed
        );
        assert_eq!(
            healed.names_departed,
            vec![DepartedDpnsName {
                identity_id,
                label: DEPARTED_LABEL.to_string(),
                document_id: Some(document_id),
                status: None,
            }],
            "the healed pass must complete the departure with the document id \
             recovered from the mirror"
        );
        assert_eq!(
            dpns_labels(&wallet, &identity_id).await,
            Vec::<String>::new(),
            "the completed departure finally drops the label"
        );
        assert_eq!(
            mirror.stored_dpns_removals(),
            vec![document_id],
            "the removal delta must finally reach the durable mirror"
        );
    }

    /// A live `IdentityWallet` over a bare mock SDK (no expectations, so
    /// every network read fails) whose persister is `mirror`.
    fn mirror_backed_identity_wallet(mirror: Arc<MirrorPersister>) -> IdentityWallet {
        mirror_backed_identity_wallet_with_sdk(
            mirror,
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
        )
    }

    /// A mock SDK primed so the departed-name domain lookup for `label`
    /// answers "no such document" — Platform CONFIRMING the name is
    /// gone, which is the branch that resolves a departure and emits its
    /// removal delta. Without this the mock has no expectations, every
    /// fetch errors, and resolution can only ever take its retry arm —
    /// which would make a "retained on persistence failure" assertion
    /// vacuous, since the network failure alone already retains.
    async fn sdk_with_absent_dpns_domain(label: &str) -> Arc<dash_sdk::Sdk> {
        sdk_answering_dpns_domain_query(label, dash_sdk::query_types::Documents::new()).await
    }

    /// A mock SDK primed to answer the DPNS contract fetch and the
    /// exact-match domain query for `label` with `documents`.
    async fn sdk_answering_dpns_domain_query(
        label: &str,
        documents: dash_sdk::query_types::Documents,
    ) -> Arc<dash_sdk::Sdk> {
        Arc::new(mock_sdk_answering_dpns_domain_query(label, documents).await)
    }

    /// [`sdk_answering_dpns_domain_query`] before the `Arc` wrap, for
    /// helpers that need to register further expectations.
    async fn mock_sdk_answering_dpns_domain_query(
        label: &str,
        documents: dash_sdk::query_types::Documents,
    ) -> dash_sdk::Sdk {
        // Pin the protocol version. Expectations are keyed by the ENCODED
        // request, and an unpinned SDK seeds at the network minimum and
        // ratchets up on the first response it sees — so the contract
        // fetch would silently re-encode every later query into a
        // different version than the one these expectations were
        // registered against, and none of them would match.
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk");
        let contract = dpp::system_data_contracts::load_system_data_contract(
            dpp::data_contracts::SystemDataContract::DPNS,
            dpp::version::PlatformVersion::latest(),
        )
        .expect("bundled DPNS contract");
        sdk.mock()
            .expect_fetch(dpns_contract_id(), Some(contract.clone()))
            .await
            .expect("DPNS contract expectation");
        let query = domain_by_normalized_label_query(
            Arc::new(contract),
            convert_to_homograph_safe_chars(dpns_label(label)),
        );
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, dash_sdk::query_types::Documents>(
                query,
                Some(documents),
            )
            .await
            .expect("domain-document expectation");
        sdk
    }

    /// A mock SDK primed like [`sdk_answering_dpns_domain_query`] and
    /// additionally answering the Document History contract fetch and the
    /// purchase/transfer history lookups for `history_document_id` with
    /// EMPTY pages — a live domain document whose history never departs
    /// any wallet identity, which is exactly what
    /// [`IdentityWallet::classify_departure`] sees when a label was
    /// deleted and re-registered by an unrelated party. Without these
    /// expectations the history fetch errors and resolution can only take
    /// its retry arm, never reaching the branch under test.
    async fn sdk_with_history_unrelated_dpns_domain(
        label: &str,
        documents: dash_sdk::query_types::Documents,
        history_document_id: Identifier,
    ) -> Arc<dash_sdk::Sdk> {
        let mut sdk = mock_sdk_answering_dpns_domain_query(label, documents).await;
        let history_contract = dpp::system_data_contracts::load_system_data_contract(
            dpp::data_contracts::SystemDataContract::DocumentHistory,
            dpp::version::PlatformVersion::latest(),
        )
        .expect("bundled Document History contract");
        sdk.mock()
            .expect_fetch(
                document_history_contract_id(),
                Some(history_contract.clone()),
            )
            .await
            .expect("Document History contract expectation");
        let history_contract = Arc::new(history_contract);
        for doc_type in [HISTORY_TYPE_PURCHASE, HISTORY_TYPE_TRANSFER] {
            let query = history_by_source_document_query(
                Arc::clone(&history_contract),
                doc_type,
                &dpns_contract_id(),
                &history_document_id,
                None,
            );
            sdk.mock()
                .expect_fetch_many::<Identifier, Document, _, dash_sdk::query_types::Documents>(
                    query,
                    Some(dash_sdk::query_types::Documents::new()),
                )
                .await
                .expect("history-document expectation");
        }
        Arc::new(sdk)
    }

    /// A mock SDK primed for a full [`IdentityWallet::sync_dpns_marketplace`]
    /// pass over one identity that has LOST `label`: the DPNS contract
    /// fetch, the identity-owned domain page query (answered empty — the
    /// identity owns no documents on Platform, so every label it still
    /// carries locally is a departure) and the exact-match domain query
    /// for `label` (also empty — Platform CONFIRMING the name is gone,
    /// the branch whose removal delta needs the persisted document id).
    async fn sdk_for_departed_identity_sync(
        identity_id: &Identifier,
        label: &str,
    ) -> Arc<dash_sdk::Sdk> {
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk");
        let contract = dpp::system_data_contracts::load_system_data_contract(
            dpp::data_contracts::SystemDataContract::DPNS,
            dpp::version::PlatformVersion::latest(),
        )
        .expect("bundled DPNS contract");
        sdk.mock()
            .expect_fetch(dpns_contract_id(), Some(contract.clone()))
            .await
            .expect("DPNS contract expectation");
        let contract = Arc::new(contract);
        // The exact first (cursor-less) page query
        // `dpns_domain_states_page` issues during a sync pass. If the
        // production query drifts from this shape the mock stops
        // matching, the page fetch errors, and the test fails on its
        // `departures_failed` precondition — loudly, not vacuously.
        let page_query = DocumentQuery {
            select: SelectProjection::documents(),
            data_contract: Arc::clone(&contract),
            document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
            where_clauses: vec![WhereClause {
                field: "records.identity".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(identity_id.to_buffer()),
            }],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: SYNC_QUERY_LIMIT,
            offset: None,
            start: None,
        };
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, dash_sdk::query_types::Documents>(
                page_query,
                Some(dash_sdk::query_types::Documents::new()),
            )
            .await
            .expect("identity domain-page expectation");
        let label_query = domain_by_normalized_label_query(
            contract,
            convert_to_homograph_safe_chars(dpns_label(label)),
        );
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, dash_sdk::query_types::Documents>(
                label_query,
                Some(dash_sdk::query_types::Documents::new()),
            )
            .await
            .expect("domain-document expectation");
        Arc::new(sdk)
    }

    /// A live `IdentityWallet` over `sdk` whose persister is `mirror`.
    /// Mirrors `PlatformWallet::new`'s wiring; only the persister and the
    /// SDK are substituted.
    fn mirror_backed_identity_wallet_with_sdk(
        mirror: Arc<MirrorPersister>,
        sdk: Arc<dash_sdk::Sdk>,
    ) -> IdentityWallet {
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::Network;
        use key_wallet_manager::WalletManager;
        use tokio::sync::RwLock;

        let mut wm = WalletManager::<crate::wallet::platform_wallet::PlatformWalletInfo>::new(
            Network::Testnet,
        );
        let wallet_id = wm
            .create_wallet_with_random_mnemonic(WalletAccountCreationOptions::None)
            .expect("create wallet");
        let wallet_manager = Arc::new(RwLock::new(wm));

        let persister = WalletPersister::new(wallet_id, mirror);
        let spv = Arc::new(crate::spv::SpvRuntime::new(
            Arc::clone(&wallet_manager),
            Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
        ));
        let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(crate::wallet::asset_lock::manager::AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(tokio::sync::Notify::new()),
            Arc::clone(&broadcaster),
            persister.clone(),
        ));
        IdentityWallet {
            sdk: Arc::clone(&sdk),
            wallet_manager,
            wallet_id,
            asset_locks,
            persister,
            broadcaster,
            sdk_writer: Arc::new(super::super::sdk_writer::SdkWriter::new(sdk)),
            dpns_operation_gate: Arc::new(tokio::sync::Mutex::new(())),
            dpns_sync_progress: Arc::new(std::sync::Mutex::new(BTreeMap::new())),
        }
    }

    // -----------------------------------------------------------------
    // Zero-price guards (listing side and purchase side)
    // -----------------------------------------------------------------

    /// A DPNS `domain` document for [`DEPARTED_LABEL`] owned by `owner`,
    /// carrying `price` as `$price` when listed. Only the fields
    /// [`DpnsDomainState::from_document`] reads are populated.
    fn listed_domain_document(
        document_id: Identifier,
        owner: Identifier,
        price: Option<Credits>,
    ) -> Document {
        let mut properties = BTreeMap::new();
        properties.insert("label".to_string(), Value::Text(DEPARTED_LABEL.to_string()));
        properties.insert(
            "normalizedLabel".to_string(),
            Value::Text(convert_to_homograph_safe_chars(DEPARTED_LABEL)),
        );
        properties.insert(
            "normalizedParentDomainName".to_string(),
            Value::Text(DPNS_PARENT_DOMAIN.to_string()),
        );
        if let Some(price) = price {
            properties.insert(PRICE.to_string(), Value::U64(price));
        }
        Document::V0(dpp::document::DocumentV0 {
            id: document_id,
            owner_id: owner,
            properties,
            revision: Some(1),
            created_at: Some(1_700_000_000_000),
            ..Default::default()
        })
    }

    /// A wallet whose Platform answers the domain query for
    /// [`DEPARTED_LABEL`] with a single document listed at `price`.
    async fn wallet_seeing_listing(
        document_id: Identifier,
        owner: Identifier,
        price: Option<Credits>,
    ) -> IdentityWallet {
        let mut documents = dash_sdk::query_types::Documents::new();
        documents.insert(
            document_id,
            Some(listed_domain_document(document_id, owner, price)),
        );
        mirror_backed_identity_wallet_with_sdk(
            Arc::new(MirrorPersister::hydrated(Vec::new())),
            sdk_answering_dpns_domain_query(DEPARTED_LABEL, documents).await,
        )
    }

    /// The purchase pre-flight's rejection ORDER, as a pure decision.
    /// `$price` absent outranks everything; a `$price` of 0 is rejected
    /// as an invalid listing BEFORE the `expected_price` comparison, so
    /// the caller is told the listing is not purchasable rather than
    /// that the price moved.
    #[test]
    fn purchase_preflight_rejects_a_zero_price_ahead_of_the_price_comparison() {
        let document_id = Identifier::from([0xB0; 32]);
        let owner = Identifier::from([0xB9; 32]);
        let state = |price: Option<Credits>| {
            DpnsDomainState::from_document(&listed_domain_document(document_id, owner, price))
                .expect("fixture document must decode")
        };

        assert!(matches!(
            preflight_purchase_price(&state(None), DEPARTED_LABEL, 5_000),
            Err(PlatformWalletError::DocumentNotForSale { document_id: got }) if got == document_id
        ));

        match preflight_purchase_price(&state(Some(0)), DEPARTED_LABEL, 5_000) {
            Err(PlatformWalletError::InvalidParameter(message)) => assert!(
                message.contains("0 credits"),
                "the rejection must name the zero price: {message}"
            ),
            other => panic!(
                "a zero listing must be InvalidParameter, never DocumentPriceChanged: {other:?}"
            ),
        }

        assert!(matches!(
            preflight_purchase_price(&state(Some(7_000)), DEPARTED_LABEL, 5_000),
            Err(PlatformWalletError::DocumentPriceChanged {
                expected: 5_000,
                actual: 7_000,
                ..
            })
        ));
        assert!(preflight_purchase_price(&state(Some(5_000)), DEPARTED_LABEL, 5_000).is_ok());
    }

    /// End-to-end through the real `purchase_dpns_name` against a mock
    /// Platform that serves a domain document listed at `$price = 0`.
    ///
    /// The purchaser is deliberately NOT one of this wallet's identities,
    /// so every step after the price pre-flight — the credit check, the
    /// signing-key selection, the broadcast — fails with a DIFFERENT,
    /// clearly identifiable error. A typed `InvalidParameter` back from
    /// the call is therefore proof that the guard fired and that nothing
    /// downstream of it ran.
    #[tokio::test]
    async fn purchase_dpns_name_rejects_a_zero_listed_price_before_signing() {
        let document_id = Identifier::from([0xB1; 32]);
        let seller = Identifier::from([0xB2; 32]);
        let purchaser = Identifier::from([0xB3; 32]);
        let wallet = wallet_seeing_listing(document_id, seller, Some(0)).await;
        let signer = simple_signer::signer::SimpleSigner::default();

        // Non-zero expectation: without the zero guard this is a plain
        // 0-vs-5000 mismatch and would surface as DocumentPriceChanged,
        // inviting a "refresh the price and retry" loop that can never
        // succeed.
        match wallet
            .purchase_dpns_name(&purchaser, DEPARTED_LABEL, 5_000, &signer)
            .await
            .expect_err("a zero-credit listing must not be purchasable")
        {
            PlatformWalletError::InvalidParameter(message) => assert!(
                message.contains("0 credits"),
                "the rejection must name the zero price: {message}"
            ),
            other => panic!(
                "expected the zero-price guard to reject ahead of the price \
                 comparison and ahead of signing, got {other:?}"
            ),
        }

        // Zero expectation: the prices MATCH, so without the guard the
        // pre-flight would pass and the call would run on into signing
        // and broadcast. Reaching an identity/signing error here instead
        // of InvalidParameter is exactly the regression.
        match wallet
            .purchase_dpns_name(&purchaser, DEPARTED_LABEL, 0, &signer)
            .await
            .expect_err("a zero-credit listing must not be purchasable at any price")
        {
            PlatformWalletError::InvalidParameter(message) => assert!(
                message.contains("0 credits"),
                "the rejection must name the zero price: {message}"
            ),
            other => {
                panic!("a matching zero price must still be refused BEFORE signing, got {other:?}")
            }
        }
    }

    /// The positive control: a non-zero listing that matches
    /// `expected_price` passes the price pre-flight and fails at the NEXT
    /// step (the buyer is not a wallet identity, so the credit check
    /// cannot find its balance). Pins that the guard rejects only zero,
    /// and that the pre-flight really does sit ahead of the credit /
    /// signing stages rather than replacing them.
    #[tokio::test]
    async fn purchase_dpns_name_lets_a_matching_non_zero_price_past_the_guard() {
        let document_id = Identifier::from([0xB4; 32]);
        let seller = Identifier::from([0xB5; 32]);
        let purchaser = Identifier::from([0xB6; 32]);
        let wallet = wallet_seeing_listing(document_id, seller, Some(5_000)).await;
        let signer = simple_signer::signer::SimpleSigner::default();

        let error = wallet
            .purchase_dpns_name(&purchaser, DEPARTED_LABEL, 5_000, &signer)
            .await
            .expect_err("the buyer is not a wallet identity, so the credit check must fail");

        assert!(
            matches!(error, PlatformWalletError::IdentityNotFound(id) if id == purchaser),
            "a matching non-zero price must pass the price pre-flight and fail at the \
             credit check: {error:?}"
        );
    }

    /// A zero-credit listing is refused BEFORE any network work, so the
    /// mock SDK (which has no expectations and would fail every fetch)
    /// never gets a chance to speak. A regression that moved the guard
    /// below the domain fetch would surface as the fetch's
    /// `InvalidIdentityData` error instead.
    #[tokio::test]
    async fn set_dpns_name_price_rejects_a_zero_price_before_any_network_work() {
        let wallet = mirror_backed_identity_wallet(Arc::new(MirrorPersister::hydrated(Vec::new())));
        // Empty: the guard rejects long before a transition is signed, so
        // this signer must never be asked for a key.
        let signer = simple_signer::signer::SimpleSigner::default();

        let error = wallet
            .set_dpns_name_price(&Identifier::from([0x01; 32]), DEPARTED_LABEL, 0, &signer)
            .await
            .expect_err("a zero-credit listing must be refused");

        match error {
            PlatformWalletError::InvalidParameter(message) => {
                assert!(
                    message.contains("0 credits"),
                    "the rejection must name the zero price: {message}"
                );
            }
            other => panic!(
                "expected a typed InvalidParameter rejection ahead of any network \
                 work, got {other:?}"
            ),
        }
    }

    /// A non-zero price passes the guard and proceeds to the (mocked-out,
    /// therefore failing) domain fetch. Pins that the guard rejects ONLY
    /// zero — a regression that rejected every price would fail here.
    #[tokio::test]
    async fn set_dpns_name_price_lets_a_non_zero_price_reach_the_network() {
        let wallet = mirror_backed_identity_wallet(Arc::new(MirrorPersister::hydrated(Vec::new())));
        let signer = simple_signer::signer::SimpleSigner::default();

        let error = wallet
            .set_dpns_name_price(&Identifier::from([0x01; 32]), DEPARTED_LABEL, 1, &signer)
            .await
            .expect_err("the mock SDK has no expectations, so the fetch must fail");

        assert!(
            !matches!(error, PlatformWalletError::InvalidParameter(_)),
            "a price of 1 credit must pass the zero-price guard and fail later, \
             at the network: {error:?}"
        );
    }
}
