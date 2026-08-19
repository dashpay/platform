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

use crate::changeset::{DpnsNameSaleStatus, DpnsNameStateChangeSet, DpnsNameStateEntry};
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

/// Summary of one [`IdentityWallet::sync_dpns_marketplace`] pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DpnsMarketplaceSyncSummary {
    /// Name-state rows written this pass (owned names refreshed).
    pub names_tracked: u32,
    /// Labels newly observed on a wallet identity: `(identity, label)`.
    pub names_added: Vec<(Identifier, String)>,
    /// Names that left a wallet identity since the local snapshot.
    pub names_departed: Vec<DepartedDpnsName>,
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
///    free.
/// 2. The persister — the durable host mirror — when the snapshot has
///    nothing.
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
/// A persister failure degrades to `None` (logged): a marketplace
/// departure must still be classified and reported when the local
/// mirror cannot be read. That reproduces today's behaviour for this
/// one name — the row is orphaned rather than the sync pass aborting —
/// and the next departure is unaffected.
fn previous_document_id_for(
    persister: &crate::wallet::persister::WalletPersister,
    identity_id: &Identifier,
    label: &str,
    previous_rows: &BTreeMap<Identifier, DpnsNameStateEntry>,
) -> Option<Identifier> {
    let normalized_label = convert_to_homograph_safe_chars(label);
    let in_memory = previous_rows
        .values()
        .find(|entry| {
            entry.wallet_identity_id == *identity_id && entry.normalized_label == normalized_label
        })
        .map(|entry| entry.document_id);
    if in_memory.is_some() {
        return in_memory;
    }
    match persister.get_dpns_name_state(identity_id, &normalized_label) {
        Ok(row) => row.map(|entry| entry.document_id),
        Err(error) => {
            tracing::warn!(
                identity = %identity_id,
                name = label,
                "persisted DPNS row lookup failed for a departed name; the host mirror may \
                 keep a stale row for it until the name is re-acquired: {error}"
            );
            None
        }
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

struct ResolvedDepartedName {
    summary: DepartedDpnsName,
    entry: Option<DpnsNameStateEntry>,
    remove_document_id: Option<Identifier>,
    retry: bool,
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
        let query = DocumentQuery {
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
                    value: Value::Text(normalized),
                },
            ],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: 1,
            offset: None,
            start: None,
        };
        Ok(self.fetch_domain_states(query).await?.into_iter().next())
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
        let listed_price = state.price.ok_or(PlatformWalletError::DocumentNotForSale {
            document_id: state.document_id,
        })?;
        // A `$price` of 0 is not a listing this wallet will act on — see
        // `set_dpns_name_price`, which refuses to create one. Rejected
        // ahead of the `expected_price` comparison so the caller is told
        // the listing itself is not purchasable rather than that the
        // price moved. Only `== 0` is affected; every `> 0` listing
        // follows the unchanged price-match path below.
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
            let query = DocumentQuery {
                select: SelectProjection::documents(),
                data_contract: Arc::clone(&contract),
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
                start: cursor.map(|id| Start::StartAfter(id.to_vec())),
            };
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
                if resolved.retry {
                    progress.pending_departures.push_front(previous_name);
                    break;
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
    /// The removal delta needs the departed name's `document_id`, which
    /// [`previous_document_id_for`] resolves from the in-memory snapshot
    /// and — when that is empty, as it always is on the first pass after
    /// a process start — from the durable persister mirror.
    async fn resolve_departed_name(
        &self,
        identity_id: &Identifier,
        label: &str,
        previous_rows: &BTreeMap<Identifier, DpnsNameStateEntry>,
        now: u64,
    ) -> ResolvedDepartedName {
        let previous_document_id =
            previous_document_id_for(&self.persister, identity_id, label, previous_rows);
        let state = match self.dpns_name_state(label).await {
            Ok(Some(state)) => state,
            Ok(None) => {
                return ResolvedDepartedName {
                    summary: DepartedDpnsName {
                        identity_id: *identity_id,
                        label: label.to_string(),
                        document_id: previous_document_id,
                        status: None,
                    },
                    entry: None,
                    remove_document_id: previous_document_id,
                    retry: false,
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
                        document_id: previous_document_id,
                        status: None,
                    },
                    entry: None,
                    remove_document_id: None,
                    retry: true,
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
                    retry: true,
                };
            }
        };
        ResolvedDepartedName {
            summary: DepartedDpnsName {
                identity_id: *identity_id,
                label: label.to_string(),
                document_id: Some(state.document_id),
                status,
            },
            entry: status.map(|sale_status| state.to_entry(*identity_id, sale_status, now)),
            remove_document_id: status.is_none().then_some(state.document_id),
            retry: false,
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
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
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
        fail: bool,
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
                fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                rows: BTreeMap::new(),
                lookups: std::sync::Mutex::new(Vec::new()),
                fail: true,
            }
        }

        fn lookups(&self) -> Vec<(WalletId, Identifier, String)> {
            self.lookups.lock().expect("lookup log").clone()
        }
    }

    impl PlatformWalletPersistence for MirrorPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
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
            if self.fail {
                return Err(PersistenceError::backend("simulated mirror read failure"));
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
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &previous_rows),
            Some(document_id)
        );
        assert!(
            mirror.lookups().is_empty(),
            "a populated in-memory snapshot must not trigger a persistence read"
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
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &previous_rows),
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
            previous_document_id_for(&persister, &identity_id, DEPARTED_LABEL, &BTreeMap::new()),
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
            ),
            None
        );
    }

    /// A persistence read FAILURE must not abort departure resolution:
    /// the name still departs and is still classified, we simply lose the
    /// removal delta for it (today's behaviour) instead of panicking or
    /// propagating.
    #[test]
    fn departed_document_id_degrades_to_none_when_the_persister_errors() {
        let mirror = Arc::new(MirrorPersister::failing());
        let persister = mirror_wallet_persister(Arc::clone(&mirror));

        assert_eq!(
            previous_document_id_for(
                &persister,
                &Identifier::from([0x99; 32]),
                DEPARTED_LABEL,
                &BTreeMap::new()
            ),
            None
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
            resolved.retry,
            "test precondition: the mock SDK has no expectations, so the domain \
             lookup must fail and request a retry"
        );
    }

    /// A live `IdentityWallet` over a mock SDK whose persister is
    /// `mirror`. Mirrors `PlatformWallet::new`'s wiring; only the
    /// persister and the SDK are substituted.
    fn mirror_backed_identity_wallet(mirror: Arc<MirrorPersister>) -> IdentityWallet {
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::Network;
        use key_wallet_manager::WalletManager;
        use tokio::sync::RwLock;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
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
    // Zero-price listing guard
    // -----------------------------------------------------------------

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
