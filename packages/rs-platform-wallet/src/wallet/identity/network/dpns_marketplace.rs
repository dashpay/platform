//! DPNS username marketplace: wallet-level search / sell / delist /
//! purchase / transfer orchestration, per-name trade history, and the
//! local name-state bookkeeping behind them.
//!
//! Design record: `rs-platform-wallet/docs/DPNS_MARKETPLACE.md`.
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
//! Consensus facts this module relies on (verified against rs-drive; see
//! the design doc §2): purchase and transfer both REMOVE `$price`
//! (transfer-to-self is therefore the delist primitive); purchase
//! requires the transition price to equal the listed price;
//! `records.identity` is rewritten to the new owner by the protocol on
//! purchase/transfer; a name inside an active contested-name vote is not
//! in the documents tree at all.

use std::collections::BTreeMap;
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
    /// `Some(Sold { to })` when the departure is attributable to a
    /// purchase, `Some(Transferred { to })` when the new owner is known
    /// but no purchase matches, and `None` when the domain document could
    /// not be resolved at all (deleted name / fetch failure) — unknown is
    /// reported as unknown, never as a fabricated counterparty.
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

// ---------------------------------------------------------------------------
// Contract caches
// ---------------------------------------------------------------------------

/// The DPNS system contract id (fixed across contract versions).
fn dpns_contract_id() -> Identifier {
    dpp::data_contracts::SystemDataContract::DPNS.id()
}

/// The Document History system contract id.
fn document_history_contract_id() -> Identifier {
    dpp::data_contracts::SystemDataContract::DocumentHistory.id()
}

/// Cached system contracts, keyed by `(network, contract id)`.
///
/// **Keyed by network, deliberately.** The cache is process-wide (a
/// `static`), but a host can hold several `PlatformWalletManager`s at
/// once and each is bound to one network. These contracts are FETCHED
/// ON-CHAIN, so a bare process-global cell would let the first caller's
/// network decide the contract definition every later caller — including
/// a wallet on a different network — uses for document queries and
/// post-broadcast proof verification. Networks routinely run different
/// protocol versions, and the DPNS / Document History system contracts
/// can differ in schema and version between them, so a cross-network hit
/// would verify against the wrong definition.
type SystemContractCache =
    std::sync::RwLock<BTreeMap<(dashcore::Network, Identifier), Arc<DataContract>>>;

static SYSTEM_CONTRACT_CACHE: std::sync::OnceLock<SystemContractCache> = std::sync::OnceLock::new();

fn system_contract_cache() -> &'static SystemContractCache {
    SYSTEM_CONTRACT_CACHE.get_or_init(|| std::sync::RwLock::new(BTreeMap::new()))
}

impl IdentityWallet {
    /// Fetch a system contract for this wallet's network, memoized in
    /// [`SYSTEM_CONTRACT_CACHE`].
    ///
    /// Goes through `fetch_contract_arc_for_document_op`, which also
    /// registers the contract with the SDK's context provider so
    /// document-query and post-broadcast proof verification can resolve
    /// it. Fetching (rather than loading the bundled system contract)
    /// guarantees the schema matches the network's ACTIVE contract
    /// version, and makes the marketplace self-sufficient on hosts that
    /// never seed the trusted provider's known-contracts list.
    ///
    /// A lock is never held across the `.await`: a miss drops the read
    /// guard, fetches, then takes the write guard. Two racing misses both
    /// fetch and the later one wins the insert — same contract, so the
    /// only cost is a duplicate round-trip.
    async fn cached_system_contract(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        let network = self.sdk.network;
        let key = (network, contract_id);
        if let Some(contract) = system_contract_cache()
            .read()
            .ok()
            .and_then(|cache| cache.get(&key).cloned())
        {
            // Re-register on cache hits too: the context provider can be
            // swapped/reset across SDK reconnects while this static
            // outlives it. Registration is an idempotent map insert.
            self.register_contract_for_proof_verification(&contract);
            return Ok(contract);
        }
        let contract = self
            .fetch_contract_arc_for_document_op(&contract_id, document_type_name)
            .await?;
        if let Ok(mut cache) = system_contract_cache().write() {
            cache.insert(key, Arc::clone(&contract));
        }
        Ok(contract)
    }

    /// The DPNS data contract for this wallet's network. See
    /// [`Self::cached_system_contract`].
    pub(crate) async fn dpns_contract(&self) -> Result<Arc<DataContract>, PlatformWalletError> {
        self.cached_system_contract(dpns_contract_id(), DPNS_DOCUMENT_TYPE)
            .await
    }

    /// The Document History system contract for this wallet's network —
    /// the event log DPNS v2's `keeps*History` flags write `transfer` /
    /// `purchase` / `priceUpdate` documents into. NOT the GroveDB
    /// `documentsKeepHistory` mechanism (`getDocumentHistory` returns
    /// empty for DPNS). See [`Self::cached_system_contract`].
    pub(crate) async fn document_history_contract(
        &self,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        self.cached_system_contract(document_history_contract_id(), HISTORY_TYPE_TRANSFER)
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
    /// server-side price filter or ordering — `$price` is not indexable
    /// (design doc §7); the marketplace is search-driven.
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
    pub async fn dpns_domain_states_for_identity(
        &self,
        identity_id: &Identifier,
        limit: Option<u32>,
    ) -> Result<Vec<DpnsDomainState>, PlatformWalletError> {
        let contract = self.dpns_contract().await?;
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
            limit: limit.unwrap_or(SYNC_QUERY_LIMIT),
            offset: None,
            start: None,
        };
        self.fetch_domain_states(query).await
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
            .identity(identity_id)
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
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
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
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
            return;
        };
        managed.remove_dpns_name(label, &self.persister);
    }

    /// Whether `identity_id` is one of this wallet's identities.
    async fn is_wallet_identity(&self, identity_id: &Identifier) -> bool {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.identity_manager.identity(identity_id).is_some())
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
    /// Pre-flight: the name must resolve to a domain document owned by
    /// `owner_identity_id` (typed contested/not-found errors otherwise).
    /// The signing key is auto-selected on the owner. On success the
    /// local sale state is persisted from the confirmed document and the
    /// updated state returned.
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
        let state = self.fetch_dpns_domain_state_required(name).await?;
        if state.owner_id == *purchaser_identity_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "identity {purchaser_identity_id} already owns DPNS name {name:?}"
            )));
        }
        let listed_price = state.price.ok_or(PlatformWalletError::DocumentNotForSale {
            document_id: state.document_id,
        })?;
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
                .managed_identity(purchaser_identity_id)
                .map(|m| m.balance())
                .ok_or(PlatformWalletError::IdentityNotFound(
                    *purchaser_identity_id,
                ))?
        };
        let required = expected_price.saturating_add(DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS);
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

    /// Fetch one history document type's rows for a source document via
    /// the `byDocument` (dataContractId, documentId, $createdAt) index.
    async fn fetch_history_documents(
        &self,
        source_contract_id: &Identifier,
        source_document_id: &Identifier,
        history_doc_type: &str,
    ) -> Result<Vec<Document>, PlatformWalletError> {
        let contract = self.document_history_contract().await?;
        let query = DocumentQuery {
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
            start: None,
        };
        let documents = Document::fetch_many(&self.sdk, query).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to fetch {history_doc_type} history documents: {e}"
            ))
        })?;
        Ok(documents.into_iter().filter_map(|(_, doc)| doc).collect())
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
        // Snapshot identity ids, their label lists, and the current rows.
        let (identity_ids, labels_by_identity, previous_rows) = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let ids = info.identity_manager.identity_ids();
            let labels: BTreeMap<Identifier, Vec<DpnsNameInfo>> = ids
                .iter()
                .filter_map(|id| {
                    info.identity_manager
                        .managed_identity(id)
                        .map(|m| (*id, m.dpns_names.clone()))
                })
                .collect();
            (ids, labels, info.dpns_name_states.clone())
        };

        let mut summary = DpnsMarketplaceSyncSummary::default();
        let mut rows_to_write: Vec<DpnsNameStateEntry> = Vec::new();
        let mut sellers_to_refresh: Vec<Identifier> = Vec::new();
        let now = now_ms();

        for identity_id in identity_ids {
            let states = match self
                .dpns_domain_states_for_identity(&identity_id, None)
                .await
            {
                Ok(states) => states,
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        "DPNS marketplace sync: domain-state fetch failed, skipping identity: {e}"
                    );
                    continue;
                }
            };
            // `records.identity` follows ownership on-chain, but filter on
            // `$ownerId` anyway so a protocol edge (or pre-rewrite record)
            // can't count someone else's document as ours.
            let owned: Vec<&DpnsDomainState> = states
                .iter()
                .filter(|s| s.owner_id == identity_id)
                .collect();
            let owned_normalized: Vec<String> =
                owned.iter().map(|s| s.normalized_label.clone()).collect();

            let previous_labels = labels_by_identity
                .get(&identity_id)
                .cloned()
                .unwrap_or_default();

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
                rows_to_write.push(state.to_entry(identity_id, DpnsNameSaleStatus::Owned, now));
                summary.names_tracked += 1;
            }

            // Newly observed labels → legacy list additions.
            for state in &owned {
                let known = previous_labels
                    .iter()
                    .any(|n| convert_to_homograph_safe_chars(&n.label) == state.normalized_label);
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

            // Departed labels: previously listed on the identity, no longer
            // among its owned documents.
            for prev_name in &previous_labels {
                let normalized = convert_to_homograph_safe_chars(&prev_name.label);
                if owned_normalized.contains(&normalized) {
                    continue;
                }
                let departed = self
                    .resolve_departed_name(&identity_id, &prev_name.label, &previous_rows, now)
                    .await;
                self.remove_dpns_label(&identity_id, &prev_name.label).await;
                if let Some(entry) = departed.1 {
                    rows_to_write.push(entry);
                }
                if matches!(departed.0.status, Some(DpnsNameSaleStatus::Sold { .. })) {
                    sellers_to_refresh.push(identity_id);
                }
                summary.names_departed.push(departed.0);
            }
        }

        self.record_dpns_name_states(rows_to_write, vec![]).await;
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
    /// Returns the summary record plus the updated row (when the
    /// document could be resolved).
    async fn resolve_departed_name(
        &self,
        identity_id: &Identifier,
        label: &str,
        previous_rows: &BTreeMap<Identifier, DpnsNameStateEntry>,
        now: u64,
    ) -> (DepartedDpnsName, Option<DpnsNameStateEntry>) {
        let state = match self.dpns_name_state(label).await {
            Ok(Some(state)) => state,
            Ok(None) | Err(_) => {
                // Document gone (deleted name) or unreadable: report the
                // departure without a new owner; keep any old row as-is.
                let document_id = previous_rows
                    .values()
                    .find(|e| {
                        e.wallet_identity_id == *identity_id
                            && e.normalized_label == convert_to_homograph_safe_chars(label)
                    })
                    .map(|e| e.document_id);
                return (
                    DepartedDpnsName {
                        identity_id: *identity_id,
                        label: label.to_string(),
                        document_id,
                        status: None,
                    },
                    None,
                );
            }
        };
        let status = self
            .classify_departure(&state.document_id, &state.owner_id, state.transferred_at_ms)
            .await;
        let entry = state.to_entry(*identity_id, status, now);
        (
            DepartedDpnsName {
                identity_id: *identity_id,
                label: label.to_string(),
                document_id: Some(state.document_id),
                status: Some(status),
            },
            Some(entry),
        )
    }

    /// Sold vs transferred: the protocol stamps the history `purchase`
    /// document and the domain's `$transferredAt` from the same block
    /// time, so a purchase event whose buyer is the new owner at exactly
    /// the domain's transfer timestamp means the departure was a sale.
    /// Anything else — including an unavailable history query — reports
    /// as `Transferred` (the enum's documented fallback), never a
    /// fabricated `Sold`.
    async fn classify_departure(
        &self,
        document_id: &Identifier,
        new_owner: &Identifier,
        domain_transferred_at_ms: Option<u64>,
    ) -> DpnsNameSaleStatus {
        let purchases = match self
            .fetch_history_documents(&dpns_contract_id(), document_id, HISTORY_TYPE_PURCHASE)
            .await
        {
            Ok(docs) => docs,
            Err(e) => {
                tracing::warn!(
                    document = %document_id,
                    "purchase-history lookup failed; reporting departure as transfer: {e}"
                );
                return DpnsNameSaleStatus::Transferred { to: *new_owner };
            }
        };
        let sold = purchases.iter().any(|doc| {
            doc.owner_id() == *new_owner
                && domain_transferred_at_ms.is_some()
                && doc.created_at() == domain_transferred_at_ms
        });
        if sold {
            DpnsNameSaleStatus::Sold { to: *new_owner }
        } else {
            DpnsNameSaleStatus::Transferred { to: *new_owner }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
