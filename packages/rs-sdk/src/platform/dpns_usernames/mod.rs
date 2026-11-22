mod contender_request;
mod contested_queries;
mod give_up;
mod queries;

pub use contender_request::{ContenderRequest, ContestRequests};
pub use contested_queries::ContestedDpnsUsername;
pub use dash_platform_queries::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
};
pub use give_up::DpnsNameGiveUpOptions;
pub use queries::DpnsUsername;

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::transitions::DocumentDeleteTransitionBuilder;
use crate::platform::transition::put_document::PutDocument;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::{DataContract, Document, Fetch, FetchMany};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::ResponseMetadata;
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef, LookupHashKey};
use dpp::document::{DocumentV0, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{Rng, SeedableRng};
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use drive::query::{SelectProjection, WhereClause, WhereOperator};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use tracing::debug;
use tracing::warn;

fn extract_dpns_label(name: &str) -> &str {
    if let Some(dot_pos) = name.rfind('.') {
        let (label_part, suffix) = name.split_at(dot_pos);
        if suffix.eq_ignore_ascii_case(".dash") {
            return label_part;
        }
    }
    name
}

/// Strip an optional case-insensitive `.dash` suffix and apply DPNS
/// homograph-safe normalization, producing a value suitable for matching
/// against the `normalizedLabel` field of `domain` documents.
///
/// Accepts either a bare label (e.g. `"alice"`) or a full DPNS name
/// (e.g. `"alice.dash"`, `"Alice.DASH"`) and returns the normalized label
/// (e.g. `"a11ce"`). A normalized label is returned unchanged.
fn normalize_dpns_label(input: &str) -> String {
    convert_to_homograph_safe_chars(extract_dpns_label(input))
}

/// The property a DPNS v3 preorder holds the hash of the name alone under, as the contract
/// names it, which the `ownerAndNameHash` index keys with its owner. Spelled here rather than
/// read from the bundled DPNS contract, which the SDK builds only with that contract's feature.
const NAME_HASH: &str = "nameHash";

/// How long before a pending preorder expires the registration stops revealing it: a domain
/// create broadcast now lands in a later block, and a preorder expired at that block's time
/// is not found (40120). One minute covers the blocks a broadcast waits for.
const DPNS_PREORDER_REVEAL_MARGIN_MS: u64 = 60_000;

/// A preorder of the name its owner already holds ([`Sdk::pending_dpns_preorder`]).
#[derive(Debug, Clone, PartialEq)]
enum PendingDpnsPreorder {
    /// It lives long enough for a domain create broadcast now to reveal it.
    Revealable(Document),
    /// It has expired, or expires before a domain create broadcast now would land, but is
    /// still stored: the cleanup of expired documents has not reached it. A reveal of it is
    /// refused (40120), and so is another preorder of the name while it holds the owner's
    /// `ownerAndNameHash` entry (40105), so its owner deletes it and preorders anew.
    Expiring(Document),
}

/// Whether `preorder`, of a type whose documents live `ttl_seconds`, can still be revealed by
/// a domain create broadcast at `now_ms`, the time of the newest block a proof reports: the
/// network counts it expired from `$createdAt` plus the time to live, so it must not reach that
/// within [`DPNS_PREORDER_REVEAL_MARGIN_MS`]. A preorder of a type with no time to live, or
/// with no `$createdAt`, never expires.
fn preorder_revealable(preorder: &Document, ttl_seconds: Option<u32>, now_ms: u64) -> bool {
    let (Some(ttl_seconds), Some(created_at)) = (ttl_seconds, preorder.created_at()) else {
        return true;
    };
    let expires_at = created_at.saturating_add(u64::from(ttl_seconds).saturating_mul(1_000));
    now_ms.saturating_add(DPNS_PREORDER_REVEAL_MARGIN_MS) < expires_at
}

/// Hash a buffer twice using SHA256 (double SHA256)
fn hash_double(data: Vec<u8>) -> [u8; 32] {
    use dpp::dashcore::hashes::sha256d;
    // sha256d already does double SHA256
    let hash = sha256d::Hash::hash(&data);
    hash.to_byte_array()
}

/// The hash properties a preorder by `owner_id` holds for a domain carrying
/// `domain_properties`, as the DPNS contract the network runs declares them. From DPNS v3
/// the domain's `creatorRefersTo` finds the creator's own preorder through a `findBy`
/// function over the normalized label, `"."` and the normalized parent, which the preorder
/// holds under the property the function fills (`nameHash`), and chooses the first valid
/// preorder of the name, one holding under its `validWhen` property (`ownerNameHash`) the
/// hash of its owner's id and the same values: both computed by the declaration as the
/// platform computes them. Before it the domain declares no such reference, and the
/// preorder holds `saltedDomainHash`, the hash of the domain's `preorderSalt` and
/// `<normalizedLabel>.dash`. A creator reference without a `findBy` function or a
/// `chooseFirst` is an error, never the older hash, which such a contract could not reveal.
fn preorder_hash_properties(
    domain_document_type: DocumentTypeRef,
    owner_id: Identifier,
    domain_properties: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, Error> {
    let Some(reference) = domain_document_type.creator_reference() else {
        let (Some(Value::Bytes32(salt)), Some(Value::Text(normalized_label))) = (
            domain_properties.get("preorderSalt"),
            domain_properties.get("normalizedLabel"),
        ) else {
            return Err(Error::Generic(
                "a DPNS domain carries a 32-byte preorderSalt and a normalizedLabel".to_string(),
            ));
        };
        let mut salted_domain_buffer = salt.to_vec();
        salted_domain_buffer.extend(format!("{normalized_label}.dash").as_bytes());
        return Ok(BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes32(hash_double(salted_domain_buffer)),
        )]));
    };
    let lookup = reference
        .as_any_document_reference()
        .and_then(|declaration| declaration.lookup)
        .ok_or_else(|| {
            Error::Generic("the DPNS domain's creatorRefersTo declares no findBy".to_string())
        })?;
    let (name_hash_property, name_key) = lookup.hash_key().ok_or_else(|| {
        Error::Generic("the DPNS domain's creatorRefersTo declares no findBy function".to_string())
    })?;
    let choose_first = lookup.choose_first.as_ref().ok_or_else(|| {
        Error::Generic("the DPNS domain's creatorRefersTo declares no chooseFirst".to_string())
    })?;
    let hash = |key: &LookupHashKey| {
        key.key_value(domain_document_type, None, owner_id, domain_properties)
            .map(|(hash, _)| Value::Bytes32(hash))
            .map_err(|error| {
                Error::Generic(format!(
                    "cannot compute the DPNS preorder hash: {} {}",
                    error.param, error.reason
                ))
            })
    };
    Ok(BTreeMap::from([
        (name_hash_property.to_string(), hash(name_key)?),
        (
            choose_first.valid_when_property.clone(),
            hash(&choose_first.valid_when)?,
        ),
    ]))
}

/// The query for the preorder `owner_id` holds with `name_hash`, through the DPNS v3
/// `ownerAndNameHash` index: at most one exists.
fn pending_preorder_query(
    dpns_contract: Arc<DataContract>,
    owner_id: Identifier,
    name_hash: Value,
) -> DocumentQuery {
    DocumentQuery {
        select: SelectProjection::documents(),
        data_contract: dpns_contract,
        document_type_name: "preorder".to_string(),
        where_clauses: vec![
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(owner_id.to_buffer()),
            },
            WhereClause {
                field: NAME_HASH.to_string(),
                operator: WhereOperator::Equal,
                value: name_hash,
            },
        ],
        time_range_clauses: vec![],
        integer_range_clauses: vec![],
        sub_queries: vec![],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![],
        limit: 1,
        offset: None,
        start: None,
    }
}

/// Callback type for preorder document
pub type PreorderCallback = Box<dyn FnOnce(&Document) + Send>;

/// Input for registering a DPNS name
pub struct RegisterDpnsNameInput<S: Signer<IdentityPublicKey>> {
    /// The label for the domain (e.g., "alice" for "alice.dash")
    pub label: String,
    /// The identity that will own the domain
    pub identity: Identity,
    /// The identity public key to use for signing
    pub identity_public_key: IdentityPublicKey,
    /// The signer for the identity
    pub signer: S,
    /// Optional callback to be called with the preorder document result
    pub preorder_callback: Option<PreorderCallback>,
    /// The most the registration is willing to pay into the contest a contested name joins.
    /// From protocol version 14 the fund to join doubles once a contest holds 250 contenders
    /// and again for every 50 more, and a registration is charged it. The identity must hold
    /// what it states. `None` states the fund to join read just before the domain is submitted.
    pub contest_fund: Option<Credits>,
}

/// Result of a DPNS name registration
#[derive(Debug)]
pub struct RegisterDpnsNameResult {
    /// The preorder document that was created. From protocol version 15 the domain
    /// create deletes it, so it is no longer in state once the name is registered
    pub preorder_document: Document,
    /// The domain document that was created
    pub domain_document: Document,
    /// The full domain name (e.g., "alice.dash")
    pub full_domain_name: String,
}

/// The documents a DPNS registration submits, built against one DPNS contract.
struct DpnsRegistrationDocuments {
    /// The DPNS contract the network stores now, which the documents were built against.
    dpns_contract: DataContract,
    preorder_document_type: DocumentType,
    preorder_document: Document,
    domain_document_type: DocumentType,
    domain_document: Document,
    normalized_label: String,
}

/// How many DPNS contract responses a registration reads for one proved at the protocol
/// version the SDK parsed it under.
const DPNS_CONTRACT_FETCH_ATTEMPTS: usize = 3;

/// The DPNS contract of the first response proved at the protocol version the SDK parsed it
/// under, reading the SDK's version through `sdk_version` and each response through `fetch`,
/// at most [`DPNS_CONTRACT_FETCH_ATTEMPTS`] times. The SDK's version only rises, and SDK clones
/// share it, so a version unchanged across the fetch is the one the response was parsed under;
/// one that rose meanwhile (this response, or another request, taught the SDK a newer version)
/// leaves the response unused. A response proved at a newer version was parsed under an older
/// one, which can drop the declaration the preorder hash follows. One proved at an older
/// version comes from a node behind the network, which can still hold the contract an upgrade
/// replaced. Either would commit the preorder to a hash the domain cannot reveal.
async fn dpns_contract_proved_at_parsed_version<V, F, R>(
    sdk_version: V,
    mut fetch: F,
) -> Result<DataContract, Error>
where
    V: Fn() -> u32,
    F: FnMut() -> R,
    R: Future<Output = Result<(Option<DataContract>, ResponseMetadata), Error>>,
{
    let mut last_mismatch = None;
    for _ in 0..DPNS_CONTRACT_FETCH_ATTEMPTS {
        let parsed_under = sdk_version();
        let (dpns_contract, metadata) = fetch().await?;
        if metadata.protocol_version == parsed_under && sdk_version() == parsed_under {
            return dpns_contract
                .ok_or_else(|| Error::Generic("DPNS contract not found".to_string()));
        }
        last_mismatch = Some((metadata.protocol_version, parsed_under));
    }
    let (proved_at, parsed_under) = last_mismatch.unwrap_or_default();
    Err(Error::Generic(format!(
        "no DPNS contract response was proved at the protocol version the SDK parsed it under \
         (last proved at {proved_at}, parsed under {parsed_under})"
    )))
}

impl Sdk {
    /// Helper method to get the DPNS contract ID
    fn get_dpns_contract_id(&self) -> Result<Identifier, Error> {
        // Get DPNS contract ID from system contract if available
        #[cfg(feature = "dpns-contract")]
        let dpns_contract_id = {
            use dpp::system_data_contracts::SystemDataContract;
            SystemDataContract::DPNS.id()
        };

        #[cfg(not(feature = "dpns-contract"))]
        let dpns_contract_id = {
            const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
            Identifier::from_string(
                DPNS_CONTRACT_ID,
                dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| Error::Generic(format!("Invalid DPNS contract ID: {}", e)))?
        };

        Ok(dpns_contract_id)
    }

    /// Helper method to fetch the DPNS contract, checking context provider first
    async fn fetch_dpns_contract(&self) -> Result<Arc<dpp::data_contract::DataContract>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;
        self.fetch_system_data_contract(dpns_contract_id)
            .await?
            .ok_or_else(|| Error::Generic("DPNS contract not found".to_string()))
    }

    /// The DPNS contract the network stores now, fetched and proved, for a registration.
    /// The preorder commits to the hash that contract declares, so neither a contract the
    /// context provider holds for the SDK's protocol version nor one cached before an upgrade
    /// changed it will do: either can be older than the network's, and its hash one the
    /// domain cannot reveal. The SDK first learns the network's protocol version (a proven
    /// refresh), then takes a response proved at the version it parsed it under
    /// ([`dpns_contract_proved_at_parsed_version`]).
    async fn fetch_current_dpns_contract(&self) -> Result<DataContract, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;
        self.refresh_protocol_version().await?;
        dpns_contract_proved_at_parsed_version(
            || self.protocol_version_number(),
            || DataContract::fetch_with_metadata(self, dpns_contract_id, None),
        )
        .await
    }

    /// The documents a registration of `label` by `identity_id` submits, built against the
    /// DPNS contract the network stores now: the preorder, committing to the hash that
    /// contract declares, and the domain revealing it.
    async fn dpns_registration_documents(
        &self,
        label: &str,
        identity_id: Identifier,
    ) -> Result<DpnsRegistrationDocuments, Error> {
        let dpns_contract = self.fetch_current_dpns_contract().await?;

        // The confirmations of both documents are proofs the SDK decodes under the DPNS
        // contract its context provider resolves, which may be an older one cached for the
        // id (a known-contracts cache warmed before the upgrade): give it the contract the
        // documents are built against, so they decode under it
        if let Some(provider) = self.context_provider() {
            provider.register_data_contract(Arc::new(dpns_contract.clone()));
        }

        // Get document types
        let preorder_document_type = dpns_contract
            .document_type_for_name("preorder")
            .map_err(|_| Error::Generic("DPNS preorder document type not found".to_string()))?;

        let domain_document_type = dpns_contract
            .document_type_for_name("domain")
            .map_err(|_| Error::Generic("DPNS domain document type not found".to_string()))?;

        // The id of a new document commits to the identity contract nonce of
        // its create transition, so it only exists once `put_to_platform` has
        // fetched that nonce. The documents are built with a placeholder id;
        // the confirmed documents carry the real one. Nothing here needs the
        // ids up front: the domain finds its preorder by the hash of its creator
        // and its name (by the salt before DPNS v3).
        let preorder_id = Identifier::default();
        let domain_id = Identifier::default();

        let normalized_label = convert_to_homograph_safe_chars(label);
        let mut domain_properties = BTreeMap::from([
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("label".to_string(), Value::Text(label.to_string())),
            (
                "normalizedLabel".to_string(),
                Value::Text(normalized_label.clone()),
            ),
            (
                "records".to_string(),
                Value::Map(vec![(
                    Value::Text("identity".to_string()),
                    Value::Identifier(identity_id.to_buffer()),
                )]),
            ),
            (
                "subdomainRules".to_string(),
                Value::Map(vec![(
                    Value::Text("allowSubdomains".to_string()),
                    Value::Bool(false),
                )]),
            ),
        ]);

        // Before DPNS v3 the domain reveals the salt its preorder hashed
        if domain_document_type.creator_reference().is_none() {
            let salt: [u8; 32] = StdRng::from_os_rng().random();
            domain_properties.insert("preorderSalt".to_string(), Value::Bytes32(salt));
        }

        // The hashes the preorder holds
        let preorder_properties =
            preorder_hash_properties(domain_document_type, identity_id, &domain_properties)?;

        // Create preorder document
        let preorder_document = Document::V0(DocumentV0 {
            contract_version: None,
            id: preorder_id,
            owner_id: identity_id,
            properties: preorder_properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        });

        // Create domain document
        let domain_document = Document::V0(DocumentV0 {
            contract_version: None,
            id: domain_id,
            owner_id: identity_id,
            properties: domain_properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        });

        Ok(DpnsRegistrationDocuments {
            preorder_document_type: preorder_document_type.to_owned_document_type(),
            dpns_contract: dpns_contract.clone(),
            preorder_document,
            domain_document_type: domain_document_type.to_owned_document_type(),
            domain_document,
            normalized_label,
        })
    }

    /// Register a DPNS username in a single operation
    ///
    /// This method handles both the preorder and domain registration steps automatically.
    /// It generates the necessary entropy, creates both documents, and submits them in order.
    ///
    /// # Arguments
    ///
    /// * `input` - The registration input containing label, identity, public key, and signer
    ///
    /// # Returns
    ///
    /// Returns a `RegisterDpnsNameResult` containing both created documents and the full domain name
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The DPNS contract cannot be fetched
    /// - Document types are not found in the contract
    /// - Document creation or submission fails
    pub async fn register_dpns_name<S: Signer<IdentityPublicKey>>(
        &self,
        input: RegisterDpnsNameInput<S>,
    ) -> Result<RegisterDpnsNameResult, Error> {
        self.register_dpns_name_with_owner_balance_and_metadata(input)
            .await
            .map(|(result, _owner_balance, _metadata)| result)
    }

    /// Register a DPNS username like [`Sdk::register_dpns_name`], and also return
    /// what the domain document's proof carries next to the document: the credit
    /// balance of the registering identity after the registration (`None` for a
    /// proof made before protocol version 14) and the verified metadata of that
    /// response, whose height is the block the balance is a snapshot at.
    ///
    /// The domain is the registration's last write, so its balance already
    /// includes what the preorder cost.
    pub async fn register_dpns_name_with_owner_balance_and_metadata<
        S: Signer<IdentityPublicKey>,
    >(
        &self,
        input: RegisterDpnsNameInput<S>,
    ) -> Result<(RegisterDpnsNameResult, Option<Credits>, ResponseMetadata), Error> {
        let identity_id = input.identity.id().to_owned();
        let DpnsRegistrationDocuments {
            dpns_contract,
            preorder_document_type,
            preorder_document,
            domain_document_type,
            domain_document,
            normalized_label,
        } = self
            .dpns_registration_documents(&input.label, identity_id)
            .await?;

        // From DPNS v3 an identity holds one pending preorder of a name: one an earlier
        // registration left behind, whose domain create failed, is revealed again rather
        // than preordered anew, which the platform would refuse as a duplicate. One that
        // expires before a reveal could land is deleted, then the name is preordered anew
        let pending_preorder = self
            .pending_dpns_preorder(&dpns_contract, identity_id, &preorder_document)
            .await?;
        let platform_preorder_document = if let Some(PendingDpnsPreorder::Revealable(
            pending_preorder,
        )) = pending_preorder
        {
            debug!(%identity_id, document_id = %pending_preorder.id(), stage = "preorder", "DPNS registration: revealing a pending preorder");
            pending_preorder
        } else {
            if let Some(PendingDpnsPreorder::Expiring(expiring)) = pending_preorder {
                debug!(%identity_id, document_id = %expiring.id(), stage = "preorder", "DPNS registration: deleting an expiring preorder");
                self.document_delete(
                    DocumentDeleteTransitionBuilder::from_document(
                        Arc::new(dpns_contract),
                        "preorder".to_string(),
                        &expiring,
                    ),
                    &input.identity_public_key,
                    &input.signer,
                )
                .await
                .inspect_err(|error| {
                    warn!(%identity_id, stage = "preorder", %error, "DPNS registration: deleting the expiring preorder failed");
                })?;
            }
            // Submit preorder document first
            debug!(%identity_id, stage = "preorder", "DPNS registration: submitting document");
            let platform_preorder_document = preorder_document
                .put_to_platform_and_wait_for_response(
                    self,
                    preorder_document_type,
                    None, // entropy: generated together with the id once the nonce is known
                    input.identity_public_key.clone(),
                    None, // token payment info
                    &input.signer,
                    None, // settings
                )
                .await
                .inspect_err(|error| {
                    warn!(%identity_id, stage = "preorder", %error, "DPNS registration: document failed");
                })?;
            debug!(%identity_id, document_id = %platform_preorder_document.id(), stage = "preorder", "DPNS registration: document confirmed");
            platform_preorder_document
        };

        // Call the preorder callback if provided
        if let Some(callback) = input.preorder_callback {
            callback(&platform_preorder_document);
        }

        // Submit domain document after preorder
        debug!(%identity_id, stage = "domain", "DPNS registration: submitting document");
        let domain_settings = input.contest_fund.map(|contest_fund| PutSettings {
            state_transition_creation_options: Some(StateTransitionCreationOptions {
                contest_fund: Some(contest_fund),
                ..Default::default()
            }),
            ..Default::default()
        });
        let (platform_domain_document, owner_balance, metadata) = domain_document
            .put_to_platform_and_wait_for_response_with_owner_balance_and_metadata(
                self,
                domain_document_type,
                None, // entropy: generated together with the id once the nonce is known
                input.identity_public_key,
                None, // token payment info
                &input.signer,
                domain_settings,
            )
            .await
            .inspect_err(|error| {
                warn!(%identity_id, stage = "domain", %error, "DPNS registration: document failed");
            })?;
        debug!(%identity_id, document_id = %platform_domain_document.id(), stage = "domain", "DPNS registration: document confirmed");

        Ok((
            RegisterDpnsNameResult {
                preorder_document: platform_preorder_document,
                domain_document: platform_domain_document,
                full_domain_name: format!("{}.dash", normalized_label),
            },
            owner_balance,
            metadata,
        ))
    }

    /// The preorder `owner_id` already holds of the name `preorder_document` commits to,
    /// found by its owner and its `nameHash` (the `ownerAndNameHash` index), when
    /// `dpns_contract` is DPNS v3 or later, revealable or expiring as the time of the block
    /// the proof reports says ([`preorder_revealable`]); `None` when there is none, and
    /// before v3, whose preorders hash a fresh salt and so never repeat. A failed query is an
    /// error.
    async fn pending_dpns_preorder(
        &self,
        dpns_contract: &DataContract,
        owner_id: Identifier,
        preorder_document: &Document,
    ) -> Result<Option<PendingDpnsPreorder>, Error> {
        let Some(name_hash) = preorder_document.properties().get(NAME_HASH).cloned() else {
            return Ok(None);
        };
        let ttl_seconds = dpns_contract
            .document_type_for_name("preorder")
            .map_err(|_| Error::Generic("DPNS preorder document type not found".to_string()))?
            .documents_ttl_seconds();
        let query = pending_preorder_query(Arc::new(dpns_contract.clone()), owner_id, name_hash);
        let (preorders, metadata) = Document::fetch_many_with_metadata(self, query, None).await?;
        // A proven absence comes back as entries holding `None`
        let Some(preorder) = preorders.into_values().flatten().next() else {
            return Ok(None);
        };
        Ok(Some(
            if preorder_revealable(&preorder, ttl_seconds, metadata.time_ms) {
                PendingDpnsPreorder::Revealable(preorder)
            } else {
                PendingDpnsPreorder::Expiring(preorder)
            },
        ))
    }

    /// Check if a DPNS name is available
    ///
    /// # Arguments
    ///
    /// * `name` - The username label (e.g., "alice") or full DPNS name
    ///   (e.g., "alice.dash"). The `.dash` suffix is matched
    ///   case-insensitively and stripped before normalization, mirroring
    ///   [`Sdk::resolve_dpns_name`].
    ///
    /// # Returns
    ///
    /// Returns `true` if the name is available, `false` if it's taken or, from
    /// DPNS v3, tombstoned ([`Sdk::is_dpns_name_tombstoned`]): nobody registers a
    /// tombstoned name.
    pub async fn is_dpns_name_available(&self, name: &str) -> Result<bool, Error> {
        let normalized_label = normalize_dpns_label(name);

        // An empty normalized label (e.g. `""`, `".dash"`, `".DASH"`) is not
        // a registrable DPNS name, so report it as unavailable rather than
        // doing a network round-trip that would query for
        // `normalizedLabel == ""`. This mirrors the early-return guard in
        // `resolve_dpns_name` so the two APIs agree on malformed input.
        if normalized_label.is_empty() {
            return Ok(false);
        }

        Ok(self.dpns_name_state(normalized_label).await? == give_up::DpnsNameState::Free)
    }

    /// Resolve a DPNS name to an identity ID
    ///
    /// # Arguments
    ///
    /// * `name` - The full domain name (e.g., "alice.dash") or just the label (e.g., "alice")
    ///
    /// # Returns
    ///
    /// Returns the identity ID associated with the domain, or None if not found
    pub async fn resolve_dpns_name(&self, name: &str) -> Result<Option<Identifier>, Error> {
        use crate::platform::documents::document_query::DocumentQuery;
        use drive::query::WhereClause;
        use drive::query::WhereOperator;

        let normalized_label = normalize_dpns_label(name);

        // Empty normalized label (e.g. `""`, `".dash"`) can't resolve to an
        // identity; bail before the contract fetch. Mirrors `is_dpns_name_available`.
        if normalized_label.is_empty() {
            return Ok(None);
        }

        let dpns_contract = self.fetch_dpns_contract().await?;

        // Query for domain with this label
        let query = DocumentQuery {
            select: drive::query::SelectProjection::documents(),
            data_contract: dpns_contract,
            document_type_name: "domain".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
                WhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(normalized_label),
                },
            ],
            time_range_clauses: vec![],
            integer_range_clauses: vec![],
            sub_queries: vec![],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: 1,
            offset: None,
            start: None,
        };

        let documents = Document::fetch_many(self, query).await?;

        match documents.into_iter().next() {
            Some((_, Some(doc))) => identity_from_domain_records(doc.properties()),
            _ => Ok(None),
        }
    }
}

/// Read the identity a `domain` document points at from its `records` map.
///
/// A document decoded from a proof carries the identifier as
/// `Value::Identifier`, but the same document after a serde round trip
/// (CBOR, JSON, mock fixtures) comes back as `Value::Bytes` or
/// `Value::Bytes32`, because serde has no identifier type. Accept every
/// representation `to_identifier` understands, and report a malformed value
/// as an error rather than as an unresolved name.
fn identity_from_domain_records(
    properties: &BTreeMap<String, Value>,
) -> Result<Option<Identifier>, Error> {
    let Some(Value::Map(records)) = properties.get("records") else {
        return Ok(None);
    };

    records
        .iter()
        .find(|(key, _)| key.as_text() == Some("identity"))
        .map(|(_, value)| {
            value
                .to_identifier()
                .map_err(|e| Error::Generic(format!("Invalid identifier: {e}")))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StaleNodeError;
    use crate::query_types::Documents;
    use crate::sdk::test::expect_epoch_refresh;
    use crate::SdkBuilder;
    use dash_context_provider::{ContextProvider, ContextProviderError};
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::CoreBlockHeight;
    use dpp::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
        PlatformSerializableWithPlatformVersion,
    };
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::{PlatformVersion, LATEST_VERSION};
    use rs_dapi_client::CanRetry;
    use std::cell::{Cell, RefCell};
    use std::future::ready;

    #[test]
    fn test_normalize_dpns_label_strips_dash_suffix_case_insensitively() {
        // Bare label and full name normalize to the same value, regardless
        // of the case of the .dash suffix. This is the contract that
        // `is_dpns_name_available` and `resolve_dpns_name` share so that
        // queries against `normalizedLabel` agree.
        let expected = "a11ce";
        assert_eq!(normalize_dpns_label("alice"), expected);
        assert_eq!(normalize_dpns_label("alice.dash"), expected);
        assert_eq!(normalize_dpns_label("alice.DASH"), expected);
        assert_eq!(normalize_dpns_label("Alice.DaSh"), expected);
        assert_eq!(normalize_dpns_label("ALICE.DASH"), expected);

        // Non-.dash suffixes are not stripped (they are treated as part of
        // the label and normalized whole).
        assert_eq!(normalize_dpns_label("alice.eth"), "a11ce.eth");

        // Empty / suffix-only inputs normalize to an empty label.
        assert_eq!(normalize_dpns_label(""), "");
        assert_eq!(normalize_dpns_label(".dash"), "");
        assert_eq!(normalize_dpns_label(".DASH"), "");
    }

    fn domain_properties(identity: Value) -> BTreeMap<String, Value> {
        BTreeMap::from([(
            "records".to_string(),
            Value::Map(vec![(Value::Text("identity".to_string()), identity)]),
        )])
    }

    #[test]
    fn identity_from_domain_records_accepts_every_identifier_representation() {
        // A proof-decoded document carries `Value::Identifier`; the same
        // document after a serde round trip carries `Bytes` / `Bytes32`
        // (serde has no identifier type), and JSON carries base58 text.
        // Every one of them names the same identity.
        let id = Identifier::new([7u8; 32]);
        for value in [
            Value::Identifier(id.to_buffer()),
            Value::Bytes32(id.to_buffer()),
            Value::Bytes(id.to_vec()),
            Value::Text(id.to_string(dpp::platform_value::string_encoding::Encoding::Base58)),
        ] {
            let resolved = identity_from_domain_records(&domain_properties(value.clone()))
                .unwrap_or_else(|e| panic!("{value:?} should resolve: {e}"));
            assert_eq!(resolved, Some(id), "{value:?}");
        }
    }

    #[test]
    fn identity_from_domain_records_distinguishes_missing_from_malformed() {
        // No records map, or a records map without an identity entry, is an
        // unresolved name.
        assert_eq!(
            identity_from_domain_records(&BTreeMap::new()).unwrap(),
            None
        );
        let no_identity = BTreeMap::from([("records".to_string(), Value::Map(vec![]))]);
        assert_eq!(identity_from_domain_records(&no_identity).unwrap(), None);

        // A present but malformed identity is an error, never "not found".
        for malformed in [
            Value::Bytes(vec![1u8; 31]),
            Value::Text("not base58!".to_string()),
            Value::U64(7),
        ] {
            assert!(
                identity_from_domain_records(&domain_properties(malformed.clone())).is_err(),
                "{malformed:?} should be rejected"
            );
        }
    }

    const SALT: [u8; 32] = [9u8; 32];

    /// The properties of a domain `Bob.dash` the preorder hashes may read (the salt, read
    /// under DPNS v2 only).
    fn bob_domain_properties() -> BTreeMap<String, Value> {
        BTreeMap::from([
            ("label".to_string(), Value::Text("Bob".to_string())),
            (
                "normalizedLabel".to_string(),
                Value::Text("b0b".to_string()),
            ),
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("preorderSalt".to_string(), Value::Bytes32(SALT)),
        ])
    }

    /// The preorder hash properties the SDK computes for `owner_id` and `properties`
    /// against the DPNS contract of `platform_version`.
    fn preorder_hashes(
        platform_version: &PlatformVersion,
        owner_id: Identifier,
        properties: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, Error> {
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("the DPNS contract loads");
        let domain = contract
            .document_type_for_name("domain")
            .expect("the DPNS contract has a domain type");
        preorder_hash_properties(domain, owner_id, properties)
    }

    fn sha256d(parts: &[&[u8]]) -> [u8; 32] {
        use dpp::dashcore::hashes::sha256d;
        sha256d::Hash::hash(&parts.concat()).to_byte_array()
    }

    #[test]
    fn should_hash_the_salt_and_name_whoever_the_owner_for_dpns_v2() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let expected = BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes32(sha256d(&[&SALT, b"b0b.dash"])),
        )]);
        for owner_id in [Identifier::new([1u8; 32]), Identifier::new([2u8; 32])] {
            assert_eq!(
                preorder_hashes(platform_version, owner_id, &bob_domain_properties())
                    .expect("DPNS v2 hashes the salt and the name"),
                expected
            );
        }
    }

    /// DPNS v3 hashes the owner and the name its creator reference declares, and the same
    /// name without the owner, which is alike for every owner; the salt is not read
    #[test]
    fn should_hash_the_owner_and_name_the_dpns_v3_contract_declares() {
        let platform_version = PlatformVersion::latest();
        let mut without_salt = bob_domain_properties();
        without_salt.remove("preorderSalt");
        for owner_id in [Identifier::new([1u8; 32]), Identifier::new([2u8; 32])] {
            let expected = BTreeMap::from([
                (
                    "ownerNameHash".to_string(),
                    Value::Bytes32(sha256d(&[owner_id.as_slice(), b"b0b", b".", b"dash"])),
                ),
                (
                    "nameHash".to_string(),
                    Value::Bytes32(sha256d(&[b"b0b", b".", b"dash"])),
                ),
            ]);
            for properties in [&bob_domain_properties(), &without_salt] {
                assert_eq!(
                    preorder_hashes(platform_version, owner_id, properties)
                        .expect("DPNS v3 hashes the declared params"),
                    expected
                );
            }
        }
    }

    #[test]
    fn should_refuse_a_dpns_v3_domain_missing_a_hashed_property() {
        let mut properties = bob_domain_properties();
        properties.remove("normalizedLabel");
        assert!(preorder_hashes(
            PlatformVersion::latest(),
            Identifier::new([1u8; 32]),
            &properties
        )
        .is_err());
    }

    /// A context provider seeded before the upgrade to DPNS v3: it holds DPNS v2 whatever
    /// protocol version it is asked for, as a provider's known contracts do, until a contract
    /// is registered with it, which then takes its place (a writable known-contracts cache).
    /// Its clones share what was registered.
    #[derive(Clone, Default)]
    struct DpnsV2ContextProvider {
        registered: Arc<std::sync::Mutex<Option<Arc<DataContract>>>>,
    }

    impl ContextProvider for DpnsV2ContextProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            if let Some(contract) = self.registered.lock().expect("the lock").clone() {
                return Ok(Some(contract));
            }
            let platform_version = PlatformVersion::get(13).expect("protocol version 13");
            load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|error| ContextProviderError::Generic(error.to_string()))
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Err(ContextProviderError::InvalidQuorum(
                "no quorum in this test".to_string(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }

        fn register_data_contract(&self, contract: Arc<DataContract>) {
            *self.registered.lock().expect("the lock") = Some(contract);
        }
    }

    #[tokio::test]
    async fn should_build_the_registration_against_the_dpns_contract_the_network_stores() {
        // An SDK seeded at protocol version 13, whose context provider holds DPNS v2, on a
        // network that runs the latest version and stores DPNS v3
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(13).expect("protocol version 13"))
            .with_context_provider(provider.clone())
            .build()
            .expect("the mock SDK builds");
        expect_epoch_refresh(&mut sdk).await;
        let stored = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("the DPNS contract loads");
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DPNS fetch is expected");

        let owner_id = Identifier::new([1u8; 32]);
        let documents = sdk
            .dpns_registration_documents("Bob", owner_id)
            .await
            .expect("the registration documents are built");

        assert_eq!(sdk.protocol_version_number(), LATEST_VERSION);
        assert!(
            documents
                .domain_document
                .properties()
                .get("preorderSalt")
                .is_none(),
            "a DPNS v3 domain reveals no salt"
        );
        assert_eq!(documents.preorder_document.owner_id(), owner_id);
        assert_eq!(
            documents.preorder_document.properties(),
            &BTreeMap::from([
                (
                    "ownerNameHash".to_string(),
                    Value::Bytes32(sha256d(&[owner_id.as_slice(), b"b0b", b".", b"dash"])),
                ),
                (
                    "nameHash".to_string(),
                    Value::Bytes32(sha256d(&[b"b0b", b".", b"dash"])),
                ),
            ]),
        );
        // The documents' confirmations decode under the contract the provider resolves,
        // which now is the one they were built against
        let resolved = provider
            .get_data_contract(&stored.id(), PlatformVersion::latest())
            .expect("the provider answers")
            .expect("the provider holds DPNS");
        assert!(declares_the_preorder_hash(&resolved));
    }

    /// A tombstone document of `b0b.dash`, as a proof returns it.
    fn bob_tombstone() -> Document {
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::new([8u8; 32]),
            owner_id: Identifier::new([1u8; 32]),
            properties: BTreeMap::from([
                (
                    "normalizedParentDomainName".to_string(),
                    Value::Text("dash".to_string()),
                ),
                (
                    "normalizedLabel".to_string(),
                    Value::Text("b0b".to_string()),
                ),
            ]),
            revision: None,
            created_at: Some(1_700_000_000_000),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        })
    }

    /// DPNS v2, as [`DpnsV2ContextProvider`] holds it.
    fn dpns_v2() -> DataContract {
        load_system_data_contract(
            SystemDataContract::DPNS,
            PlatformVersion::get(13).expect("protocol version 13"),
        )
        .expect("the DPNS contract loads")
    }

    /// Answers the fetch of the DPNS contract with `stored`, and the refresh a registration's
    /// fetch makes first.
    async fn expect_network_dpns(sdk: &mut Sdk, stored: DataContract) {
        expect_epoch_refresh(sdk).await;
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored))
            .await
            .expect("the DPNS fetch is expected");
    }

    /// Answers the query for the tombstone of `b0b.dash` with `found`. A mock query is
    /// matched by its contract as a fetch returns it, so another SDK behind the same network
    /// fetches the network's here: `sdk`, the one under test, reads nothing before the read
    /// the test makes, and keeps the protocol version it was built with until then. The
    /// other SDK is built by `configure` as `sdk` was, since the mock encodes a contract
    /// under the version an SDK holds when the fetch is scripted.
    async fn expect_bobs_tombstone(
        sdk: &mut Sdk,
        configure: fn(SdkBuilder) -> SdkBuilder,
        found: Option<Document>,
    ) {
        let mut twin = sdk_with_a_dpns_v2_provider(&DpnsV2ContextProvider::default(), configure);
        expect_network_dpns(&mut twin, dpns_v3()).await;
        let network_contract = twin
            .fetch_network_dpns_contract_declaring_tombstones()
            .await
            .expect("the DPNS contract is fetched")
            .expect("the network's DPNS contract declares tombstones");
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, Documents>(
                give_up::dpns_name_query(
                    Arc::new(network_contract),
                    "tombstone",
                    "b0b".to_string(),
                ),
                Some([(Identifier::new([8u8; 32]), found)].into_iter().collect()),
            )
            .await
            .expect("the tombstone query is expected");
    }

    /// Answers the query for the domain of `b0b.dash`, read under DPNS v2, with none.
    async fn expect_no_domain_of_bob(sdk: &mut Sdk) {
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, Documents>(
                give_up::dpns_name_query(Arc::new(dpns_v2()), "domain", "b0b".to_string()),
                Some([(Identifier::new([9u8; 32]), None)].into_iter().collect()),
            )
            .await
            .expect("the domain query is expected");
    }

    /// A mock SDK whose context provider holds DPNS v2, built by `configure`.
    fn sdk_with_a_dpns_v2_provider(
        provider: &DpnsV2ContextProvider,
        configure: impl FnOnce(SdkBuilder) -> SdkBuilder,
    ) -> Sdk {
        configure(SdkBuilder::new_mock())
            .with_context_provider(provider.clone())
            .build()
            .expect("the mock SDK builds")
    }

    fn seeded_at_13(builder: SdkBuilder) -> SdkBuilder {
        builder.with_initial_version(PlatformVersion::get(13).expect("protocol version 13"))
    }

    fn pinned_at_latest(builder: SdkBuilder) -> SdkBuilder {
        builder.with_version(PlatformVersion::latest())
    }

    fn resolves_dpns_with_tombstones(provider: &DpnsV2ContextProvider) -> bool {
        provider
            .get_data_contract(&dpns_v3().id(), PlatformVersion::latest())
            .expect("the provider answers")
            .expect("the provider holds DPNS")
            .has_document_type_for_name("tombstone")
    }

    /// A context provider that still holds DPNS v2 after the network upgraded resolves a
    /// contract declaring no tombstone. The tombstone is found all the same: the read asks
    /// the network for its contract, reads under it and hands it to the provider.
    #[tokio::test]
    async fn should_find_a_tombstone_when_the_provider_holds_dpns_v2_after_the_upgrade() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_network_dpns(&mut sdk, dpns_v3()).await;
        expect_bobs_tombstone(&mut sdk, seeded_at_13, Some(bob_tombstone())).await;
        expect_no_domain_of_bob(&mut sdk).await;
        assert!(!resolves_dpns_with_tombstones(&provider));
        assert_eq!(sdk.protocol_version_number(), 13);

        assert!(sdk
            .is_dpns_name_tombstoned("Bob.dash")
            .await
            .expect("the tombstone is read"));
        assert!(resolves_dpns_with_tombstones(&provider));
    }

    /// And asked whether the name is available: no domain of the name exists, read under
    /// the DPNS v2 the provider holds, and the name is still not available, since its
    /// tombstone is found under the contract the network stores.
    #[tokio::test]
    async fn should_not_report_a_tombstoned_name_available_when_the_provider_holds_dpns_v2() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_network_dpns(&mut sdk, dpns_v3()).await;
        expect_bobs_tombstone(&mut sdk, seeded_at_13, Some(bob_tombstone())).await;
        expect_no_domain_of_bob(&mut sdk).await;
        assert_eq!(sdk.protocol_version_number(), 13);

        assert!(!sdk
            .is_dpns_name_available("Bob")
            .await
            .expect("the name is judged"));
    }

    /// An SDK with a pinned version never raises it from a response, so it does not learn
    /// what the network runs from its own version. Pinned to the network's version by its
    /// host, it still reads the tombstone under the contract the network stores when its
    /// provider holds DPNS v2: the domain read's verified response shows the network past the
    /// upgrade. (Pinned below the network's version it can not parse the network's contract,
    /// and the reads fail.)
    #[tokio::test]
    async fn should_find_a_tombstone_with_a_pinned_version() {
        // Each read starts from a provider holding DPNS v2, which the read replaces
        for read in ["available", "tombstoned"] {
            let provider = DpnsV2ContextProvider::default();
            let mut sdk = sdk_with_a_dpns_v2_provider(&provider, pinned_at_latest);
            expect_network_dpns(&mut sdk, dpns_v3()).await;
            expect_bobs_tombstone(&mut sdk, pinned_at_latest, Some(bob_tombstone())).await;
            expect_no_domain_of_bob(&mut sdk).await;

            if read == "available" {
                assert!(!sdk
                    .is_dpns_name_available("Bob")
                    .await
                    .expect("the name is judged"));
            } else {
                assert!(sdk
                    .is_dpns_name_tombstoned("Bob")
                    .await
                    .expect("the tombstone is read"));
            }
            assert!(resolves_dpns_with_tombstones(&provider), "{read}");
        }
    }

    /// A registered name is not tombstoned, and its tombstone is not read: the domain found
    /// answers, as a tombstone create deletes the domain. The mock holds no other
    /// expectation, so neither the network's contract nor a tombstone is asked for.
    #[tokio::test]
    async fn should_not_read_a_tombstone_of_a_registered_name() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_domain_of(&mut sdk, "Bob", Identifier::new([1u8; 32])).await;

        assert!(!sdk
            .is_dpns_name_tombstoned("Bob.dash")
            .await
            .expect("the domain answers"));
        assert!(!sdk
            .is_dpns_name_available("Bob.dash")
            .await
            .expect("the domain answers"));
    }

    /// When the network's DPNS contract can not be read (here no fetch is answered, as when
    /// the network can not be reached or the version refresh fails), a contract declaring no
    /// tombstone proves nothing: the reads fail instead of answering "not tombstoned" or
    /// "available".
    #[tokio::test]
    async fn should_not_take_an_unreadable_network_contract_for_no_tombstone() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_no_domain_of_bob(&mut sdk).await;

        assert!(sdk.is_dpns_name_tombstoned("Bob").await.is_err());
        assert!(sdk.is_dpns_name_available("Bob").await.is_err());
    }

    /// An SDK with proofs off can not fetch a contract at all (the SDK fetches contracts
    /// with proofs only), so it can not tell what the network stores: asked about a
    /// tombstone under a provider's DPNS v2, it fails instead of answering "not tombstoned".
    #[tokio::test]
    async fn should_not_answer_about_a_tombstone_with_proofs_off() {
        let provider = DpnsV2ContextProvider::default();
        let sdk = sdk_with_a_dpns_v2_provider(&provider, |builder| {
            builder
                .with_initial_version(PlatformVersion::latest())
                .with_proofs(false)
        });

        assert!(sdk.is_dpns_name_tombstoned("Bob").await.is_err());
    }

    /// The network's own contract answers when it declares no tombstone, while nothing
    /// verified shows the network past the upgrade to DPNS v3: no name is tombstoned. Once a
    /// verified response does, a contract declaring none is not the network's, and the read
    /// fails. A contract declaring tombstones is read under either way.
    #[test]
    fn should_judge_a_contract_declaring_no_tombstone_by_what_is_verified_of_the_network() {
        for before_dpns_v3_or_unknown in [Some(false), None] {
            assert!(!give_up::network_contract_declares_tombstones(
                false,
                before_dpns_v3_or_unknown
            )
            .expect("the network's contract answers"));
        }
        assert!(give_up::network_contract_declares_tombstones(false, Some(true)).is_err());
        for verified in [Some(true), Some(false), None] {
            assert!(
                give_up::network_contract_declares_tombstones(true, verified)
                    .expect("the contract declares tombstones")
            );
        }
    }

    /// A freshly built SDK pinned to protocol version 14, which has verified nothing yet,
    /// with a provider holding DPNS v2, on a network at the latest version. Its pinned
    /// version says nothing of the network. The domain read of the same call does: its
    /// verified response shows the network past the upgrade, so the provider's DPNS v2 is not
    /// taken as the network's and the network is asked for its contract. Answered with DPNS
    /// v2 again (a node behind the network), the reads fail instead of answering "not
    /// tombstoned" or "available".
    ///
    /// The mock SDK does not count a scripted answer as verified, so what the domain read's
    /// verified response shows on a network is recorded here by hand.
    #[tokio::test]
    async fn should_fail_a_fresh_pinned_sdk_read_answered_with_the_contract_the_upgrade_replaced() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, |builder| {
            builder.with_version(PlatformVersion::get(14).expect("protocol version 14"))
        });
        assert_eq!(sdk.protocol_version_info().verified_version, None);
        sdk.accept_verified_metadata("getDocuments", &proved_at(LATEST_VERSION), None)
            .expect("the metadata is accepted");
        expect_network_dpns(&mut sdk, dpns_v2()).await;
        expect_no_domain_of_bob(&mut sdk).await;

        assert!(sdk.is_dpns_name_tombstoned("Bob").await.is_err());
        assert!(sdk.is_dpns_name_available("Bob").await.is_err());
        assert_eq!(sdk.protocol_version_number(), 14);
        assert!(!resolves_dpns_with_tombstones(&provider));
    }

    /// A verified response that shows the network at protocol version 14, as the domain read
    /// of a call on such a network is, settles it without another read: at a block of that
    /// version no name was ever tombstoned. The mock SDK holds no expectation.
    #[tokio::test]
    async fn should_read_nothing_more_once_a_verified_response_proves_a_network_before_dpns_v3() {
        let provider = DpnsV2ContextProvider::default();
        let sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        sdk.accept_verified_metadata(
            "getDocuments",
            &ResponseMetadata {
                protocol_version: 14,
                ..Default::default()
            },
            None,
        )
        .expect("the metadata is accepted");

        assert!(!sdk
            .dpns_name_tombstoned_under(Arc::new(dpns_v2()), "b0b".to_string(), 0)
            .await
            .expect("nothing is read"));
    }

    /// The domain of `label.dash` held by `owner`, as a proof returns it under DPNS v2.
    fn stored_domain(label: &str, owner: Identifier) -> Document {
        let mut properties = bob_domain_properties();
        properties.insert("label".to_string(), Value::Text(label.to_string()));
        properties.insert(
            "normalizedLabel".to_string(),
            Value::Text(convert_to_homograph_safe_chars(label)),
        );
        properties.insert(
            "records".to_string(),
            Value::Map(vec![(
                Value::Text("identity".to_string()),
                Value::Identifier(owner.to_buffer()),
            )]),
        );
        properties.insert(
            "subdomainRules".to_string(),
            Value::Map(vec![(
                Value::Text("allowSubdomains".to_string()),
                Value::Bool(false),
            )]),
        );
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::new([9u8; 32]),
            owner_id: owner,
            properties,
            revision: Some(1),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_000_000),
            transferred_at: Some(1_700_000_000_000),
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        })
    }

    /// Answers the query for the domain of `label.dash`, read under DPNS v2, with it.
    async fn expect_domain_of(sdk: &mut Sdk, label: &str, owner: Identifier) {
        let domain = stored_domain(label, owner);
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, Documents>(
                give_up::dpns_name_query(
                    Arc::new(dpns_v2()),
                    "domain",
                    convert_to_homograph_safe_chars(label),
                ),
                Some([(domain.id(), Some(domain))].into_iter().collect()),
            )
            .await
            .expect("the domain query is expected");
    }

    /// A key the writes below are asked to sign with, and never do.
    fn signing_key() -> IdentityPublicKey {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{KeyType, Purpose, SecurityLevel};

        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![1; 33]),
            disabled_at: None,
        })
    }

    /// An empty name (`""`, `".dash"`) is given up by nobody: both writes refuse it before
    /// they read anything, where the network would refuse it only once broadcast and paid for.
    /// The mock SDK holds no expectation.
    #[tokio::test]
    async fn should_refuse_to_give_up_an_empty_name_before_reading_anything() {
        let provider = DpnsV2ContextProvider::default();
        let sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        let owner = Identifier::new([1u8; 32]);

        for name in ["", ".dash", ".DASH"] {
            let deleted = sdk
                .delete_dpns_name(name, &signing_key(), &UnusedSigner)
                .await;
            assert!(
                matches!(deleted, Err(Error::InvalidRequest(_))),
                "{name:?}: {deleted:?}"
            );
            let tombstoned = sdk
                .tombstone_dpns_name(name, owner, signing_key(), &UnusedSigner)
                .await;
            assert!(
                matches!(tombstoned, Err(Error::InvalidRequest(_))),
                "{name:?}: {tombstoned:?}"
            );
        }
    }

    /// A name nobody holds is not deleted: the delete stops at the proven absence of its
    /// domain, before anything is signed.
    #[tokio::test]
    async fn should_refuse_to_delete_a_name_that_is_not_registered() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_no_domain_of_bob(&mut sdk).await;

        let error = sdk
            .delete_dpns_name("Bob.dash", &signing_key(), &UnusedSigner)
            .await
            .expect_err("no domain to delete");
        assert!(error.to_string().contains("is not registered"), "{error}");
    }

    /// A contested name (`b0b` matches the contest's pattern) can only be tombstoned. The
    /// network would refuse its delete once broadcast and paid for, so the delete refuses it
    /// from the stored name: nothing else is read and nothing is signed.
    #[tokio::test]
    async fn should_refuse_to_delete_a_contested_name_before_anything_is_broadcast() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_domain_of(&mut sdk, "Bob", Identifier::new([1u8; 32])).await;

        let error = sdk
            .delete_dpns_name("Bob", &signing_key(), &UnusedSigner)
            .await
            .expect_err("a contested name is not deleted");
        assert!(error.to_string().contains("only tombstoned"), "{error}");
    }

    /// A name that is not contested may be given up either way, and the options carry its
    /// domain, whose owner is the only identity that can
    #[tokio::test]
    async fn should_offer_a_delete_and_a_tombstone_for_a_name_that_is_not_contested() {
        let owner = Identifier::new([1u8; 32]);
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_domain_of(&mut sdk, "Bob7", owner).await;

        let options = sdk
            .dpns_name_give_up_options("Bob7.dash")
            .await
            .expect("the options are read")
            .expect("the name is registered");
        assert!(options.can_delete);
        assert!(options.can_tombstone);
        assert_eq!(options.domain.owner_id(), owner);
    }

    /// The network refuses the delete of a contested name, so only a tombstone is offered
    #[tokio::test]
    async fn should_offer_only_a_tombstone_for_a_contested_name() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_domain_of(&mut sdk, "Bob", Identifier::new([1u8; 32])).await;

        let options = sdk
            .dpns_name_give_up_options("Bob")
            .await
            .expect("the options are read")
            .expect("the name is registered");
        assert!(!options.can_delete);
        assert!(options.can_tombstone);
    }

    #[tokio::test]
    async fn should_offer_nothing_for_a_name_that_is_not_registered() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_no_domain_of_bob(&mut sdk).await;

        assert_eq!(
            sdk.dpns_name_give_up_options("Bob.dash")
                .await
                .expect("the options are read"),
            None
        );
    }

    /// An uncontested name (`b0b7` holds a digit the contest's pattern does not) passes both
    /// checks, and the delete starts: it stops at the first request the mock leaves
    /// unanswered, the nonce of the transition it is about to sign.
    #[tokio::test]
    async fn should_start_the_delete_of_an_uncontested_name() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_domain_of(&mut sdk, "Bob7", Identifier::new([1u8; 32])).await;

        let error = sdk
            .delete_dpns_name("Bob7.dash", &signing_key(), &UnusedSigner)
            .await
            .expect_err("the unanswered nonce fetch stops the delete");
        let message = error.to_string();
        assert!(
            message.contains("GetIdentityContractNonceRequest"),
            "expected the delete to stop at its nonce fetch, got {message}"
        );
        assert!(!message.contains("only tombstoned"), "{message}");
        assert!(!message.contains("is not registered"), "{message}");
    }

    /// A domain document a caller already read is deleted as it is: a contested one is
    /// refused before anything is read or signed, and the mock SDK holds no expectation
    #[tokio::test]
    async fn should_refuse_to_delete_a_given_contested_domain_before_anything_is_read() {
        let provider = DpnsV2ContextProvider::default();
        let sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);

        let error = sdk
            .delete_dpns_domain(
                stored_domain("Bob", Identifier::new([1u8; 32])),
                &signing_key(),
                &UnusedSigner,
            )
            .await
            .expect_err("a contested name is not deleted");
        assert!(error.to_string().contains("only tombstoned"), "{error}");
    }

    /// The delete of a given domain that is not contested is built without reading the
    /// name again (no domain query is expected): it stops at the nonce of the transition
    #[tokio::test]
    async fn should_delete_a_given_domain_without_reading_the_name_again() {
        let provider = DpnsV2ContextProvider::default();
        let sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);

        let error = sdk
            .delete_dpns_domain(
                stored_domain("Bob7", Identifier::new([1u8; 32])),
                &signing_key(),
                &UnusedSigner,
            )
            .await
            .expect_err("the unanswered nonce fetch stops the delete");
        let message = error.to_string();
        assert!(
            message.contains("GetIdentityContractNonceRequest"),
            "expected the delete to stop at its nonce fetch, got {message}"
        );
    }

    /// An SDK pinned below protocol version 15 can not build a give-up whatever the network
    /// runs: both the read and the delete of a given domain are refused as pinned, before
    /// anything is read (this SDK expects no request)
    #[tokio::test]
    async fn should_refuse_give_up_on_an_sdk_pinned_below_protocol_version_15() {
        let provider = DpnsV2ContextProvider::default();
        let sdk = sdk_with_a_dpns_v2_provider(&provider, |builder| {
            builder.with_version(PlatformVersion::get(14).expect("protocol version 14"))
        });
        let pinned = |error: &Error| {
            matches!(
                error,
                Error::ProtocolVersionNotActive {
                    required: 15,
                    pinned: true,
                    ..
                }
            )
        };

        let read = sdk
            .dpns_name_give_up_options("Bob7")
            .await
            .expect_err("the read is refused");
        assert!(pinned(&read), "{read:?}");
        let deleted = sdk
            .delete_dpns_domain(
                stored_domain("Bob7", Identifier::new([1u8; 32])),
                &signing_key(),
                &UnusedSigner,
            )
            .await
            .expect_err("the delete is refused");
        assert!(pinned(&deleted), "{deleted:?}");
    }

    /// On a network whose DPNS contract declares no tombstone (before protocol version 15) a
    /// name can not be tombstoned: the tombstone stops with that reason once it has fetched
    /// the contract the network stores, before anything is signed.
    #[tokio::test]
    async fn should_refuse_to_tombstone_on_a_network_whose_contract_declares_no_tombstone() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_network_dpns(&mut sdk, dpns_v2()).await;

        let error = sdk
            .tombstone_dpns_name(
                "Bob",
                Identifier::new([1u8; 32]),
                signing_key(),
                &UnusedSigner,
            )
            .await
            .expect_err("DPNS v2 has no tombstone");
        assert!(
            error.to_string().contains("declares no tombstone"),
            "{error}"
        );
    }

    /// A tombstone is built against the DPNS v3 the network stores, whatever the provider
    /// held, and that contract is handed to the provider so the confirmation decodes under
    /// it. The write then starts: it stops at the first request the mock leaves unanswered,
    /// the nonce of the transition it is about to sign.
    #[tokio::test]
    async fn should_build_a_tombstone_against_the_network_contract_and_start_the_write() {
        let provider = DpnsV2ContextProvider::default();
        let mut sdk = sdk_with_a_dpns_v2_provider(&provider, seeded_at_13);
        expect_network_dpns(&mut sdk, dpns_v3()).await;
        assert!(!resolves_dpns_with_tombstones(&provider));

        let error = sdk
            .tombstone_dpns_name(
                "Bob.dash",
                Identifier::new([1u8; 32]),
                signing_key(),
                &UnusedSigner,
            )
            .await
            .expect_err("the unanswered nonce fetch stops the tombstone");
        let message = error.to_string();
        assert!(
            message.contains("GetIdentityContractNonceRequest"),
            "expected the tombstone to stop at its nonce fetch, got {message}"
        );
        assert!(!message.contains("declares no tombstone"), "{message}");
        assert_eq!(sdk.protocol_version_number(), LATEST_VERSION);
        assert!(resolves_dpns_with_tombstones(&provider));
    }

    /// A signer the tests never reach: each registration below stops before it signs.
    #[derive(Debug)]
    struct UnusedSigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for UnusedSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            Err(dpp::ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            Err(dpp::ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// The public registration, from an SDK seeded at protocol version 13 whose context
    /// provider holds DPNS v2: it learns the network's version, builds against the DPNS v3
    /// the network stores, hands that contract to the provider, and looks for a pending
    /// preorder of the name before it pays for a new one. That query is left unanswered, so
    /// the registration stops with its error, before any nonce is fetched or anything signed.
    #[tokio::test]
    async fn should_look_for_a_pending_preorder_before_paying_for_one() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{KeyType, Purpose, SecurityLevel};

        let provider = DpnsV2ContextProvider::default();
        let mut sdk = SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(13).expect("protocol version 13"))
            .with_context_provider(provider.clone())
            .build()
            .expect("the mock SDK builds");
        expect_epoch_refresh(&mut sdk).await;
        let stored = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("the DPNS contract loads");
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DPNS fetch is expected");

        let owner_id = Identifier::new([1u8; 32]);
        let error = sdk
            .register_dpns_name(RegisterDpnsNameInput {
                label: "Bob".to_string(),
                identity: Identity::create_basic_identity(owner_id, PlatformVersion::latest())
                    .expect("the identity builds"),
                identity_public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: 1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: dpp::platform_value::BinaryData::new(vec![1; 33]),
                    disabled_at: None,
                }),
                signer: UnusedSigner,
                preorder_callback: None,
                contest_fund: None,
            })
            .await
            .expect_err("the unanswered pending-preorder query stops the registration");

        // The request it stopped at is the documents query for the pending preorder, not the
        // nonce fetch a new preorder would start with
        let message = error.to_string();
        assert!(
            message.contains("GetDocumentsRequest"),
            "expected the pending-preorder query to fail, got {message}"
        );
        assert_eq!(sdk.protocol_version_number(), LATEST_VERSION);
        let resolved = provider
            .get_data_contract(&stored.id(), PlatformVersion::latest())
            .expect("the provider answers")
            .expect("the provider holds DPNS");
        assert!(declares_the_preorder_hash(&resolved));
    }

    /// A DPNS v3 preorder of `owner` for `Bob.dash`, with the hashes the contract declares.
    fn bob_v3_preorder(owner: Identifier) -> Document {
        let properties =
            preorder_hashes(PlatformVersion::latest(), owner, &bob_domain_properties())
                .expect("DPNS v3 hashes the declared params");
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::new([7u8; 32]),
            owner_id: owner,
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: Some(10),
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        })
    }

    fn dpns_v3() -> DataContract {
        load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("the DPNS contract loads")
    }

    /// A mock SDK at the latest version; a request without an expectation fails.
    fn mock_sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(DpnsV2ContextProvider::default())
            .build()
            .expect("the mock SDK builds")
    }

    /// The pending preorder is looked up through `ownerAndNameHash`: the owner and the
    /// preorder's own `nameHash`, at most one
    #[test]
    fn should_query_a_pending_preorder_by_its_owner_and_name_hash() {
        let owner = Identifier::new([1u8; 32]);
        let hash = Value::Bytes32([4u8; 32]);
        let query = pending_preorder_query(Arc::new(dpns_v3()), owner, hash.clone());
        assert_eq!(query.document_type_name, "preorder");
        assert_eq!(
            query.where_clauses,
            vec![
                WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(owner.to_buffer()),
                },
                WhereClause {
                    field: "nameHash".to_string(),
                    operator: WhereOperator::Equal,
                    value: hash,
                },
            ]
        );
        assert_eq!(query.limit, 1);
    }

    #[tokio::test]
    async fn should_return_the_pending_preorder_the_network_holds() {
        let owner = Identifier::new([1u8; 32]);
        let pending = bob_v3_preorder(owner);
        let mut sdk = mock_sdk();
        let query = pending_preorder_query(
            Arc::new(dpns_v3()),
            owner,
            pending.properties()["nameHash"].clone(),
        );
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, Documents>(
                query,
                Some(
                    [(pending.id(), Some(pending.clone()))]
                        .into_iter()
                        .collect(),
                ),
            )
            .await
            .expect("the expectation is set");

        // It records no `$createdAt`, so it never expires
        let Some(PendingDpnsPreorder::Revealable(found)) = sdk
            .pending_dpns_preorder(&dpns_v3(), owner, &bob_v3_preorder(owner))
            .await
            .expect("the query answers")
        else {
            panic!("the pending preorder is found, revealable");
        };
        // The response is decoded as a proof is, so compare what identifies the preorder
        assert_eq!(found.id(), pending.id());
        assert_eq!(found.owner_id(), owner);
        assert_eq!(
            found
                .properties()
                .get("nameHash")
                .and_then(|hash| hash.as_bytes_slice().ok()),
            pending.properties()["nameHash"].as_bytes_slice().ok()
        );
    }

    #[tokio::test]
    async fn should_find_no_pending_preorder_in_a_proven_absence() {
        let owner = Identifier::new([1u8; 32]);
        let preorder = bob_v3_preorder(owner);
        let mut sdk = mock_sdk();
        let query = pending_preorder_query(
            Arc::new(dpns_v3()),
            owner,
            preorder.properties()["nameHash"].clone(),
        );
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, Documents>(
                query,
                Some([(preorder.id(), None)].into_iter().collect()),
            )
            .await
            .expect("the expectation is set");

        assert_eq!(
            sdk.pending_dpns_preorder(&dpns_v3(), owner, &preorder)
                .await
                .expect("the query answers"),
            None
        );
    }

    /// The pending preorder is judged at the block time its response reports, against the
    /// fetched preorder type's time to live and the minute's margin: one created an hour and a
    /// minute and a millisecond before a reveal broadcast then could reach is revealable, one
    /// created a millisecond earlier is expiring, and either way the found document is the one
    /// returned.
    #[tokio::test]
    async fn should_judge_a_pending_preorder_at_the_time_its_response_reports() {
        let owner = Identifier::new([1u8; 32]);
        let now = 1_700_000_000_000u64;
        let ttl_ms = 3_600_000u64;
        for (created_at, expiring) in [
            (now + DPNS_PREORDER_REVEAL_MARGIN_MS - ttl_ms + 1, false),
            (now + DPNS_PREORDER_REVEAL_MARGIN_MS - ttl_ms, true),
        ] {
            let mut pending = bob_v3_preorder(owner);
            let Document::V0(ref mut v0) = pending;
            v0.created_at = Some(created_at);
            let mut sdk = mock_sdk();
            let query = pending_preorder_query(
                Arc::new(dpns_v3()),
                owner,
                pending.properties()["nameHash"].clone(),
            );
            sdk.mock()
                .set_response_time_ms(now)
                .expect_fetch_many::<Identifier, Document, _, Documents>(
                    query,
                    Some(
                        [(pending.id(), Some(pending.clone()))]
                            .into_iter()
                            .collect(),
                    ),
                )
                .await
                .expect("the expectation is set");

            let found = sdk
                .pending_dpns_preorder(&dpns_v3(), owner, &bob_v3_preorder(owner))
                .await
                .expect("the query answers")
                .expect("the pending preorder is found");
            let document = match (found, expiring) {
                (PendingDpnsPreorder::Revealable(document), false)
                | (PendingDpnsPreorder::Expiring(document), true) => document,
                (found, _) => panic!(
                    "created at {created_at}, judged at {now}: expected expiring {expiring}, \
                     got {found:?}"
                ),
            };
            assert_eq!(document.id(), pending.id());
        }
    }

    /// A query that fails is an error, never taken for no pending preorder
    #[tokio::test]
    async fn should_propagate_a_failed_pending_preorder_query() {
        let owner = Identifier::new([1u8; 32]);
        // No expectation: the mock fails the request
        let sdk = mock_sdk();
        assert!(sdk
            .pending_dpns_preorder(&dpns_v3(), owner, &bob_v3_preorder(owner))
            .await
            .is_err());
    }

    /// A pending preorder is revealed only while a domain create broadcast now would land before
    /// it expires: the network counts it expired from `$createdAt` plus the type's time to live,
    /// and the registration keeps a minute's margin. Without a time to live, or a `$createdAt`,
    /// it never expires.
    #[test]
    fn should_reveal_a_pending_preorder_only_while_a_reveal_lands_before_it_expires() {
        let created_at = 1_700_000_000_000u64;
        let mut preorder = bob_v3_preorder(Identifier::new([1u8; 32]));
        let Document::V0(ref mut v0) = preorder;
        v0.created_at = Some(created_at);
        let ttl = dpns_v3()
            .document_type_for_name("preorder")
            .expect("the preorder type")
            .documents_ttl_seconds();
        assert_eq!(ttl, Some(3600));
        let expires_at = created_at + 3_600_000;

        assert!(preorder_revealable(&preorder, ttl, created_at));
        assert!(preorder_revealable(
            &preorder,
            ttl,
            expires_at - DPNS_PREORDER_REVEAL_MARGIN_MS - 1
        ));
        assert!(!preorder_revealable(
            &preorder,
            ttl,
            expires_at - DPNS_PREORDER_REVEAL_MARGIN_MS
        ));
        assert!(!preorder_revealable(&preorder, ttl, expires_at));
        assert!(!preorder_revealable(&preorder, ttl, expires_at + 1));

        assert!(preorder_revealable(&preorder, None, expires_at + 1));
        assert!(preorder_revealable(
            &bob_v3_preorder(Identifier::new([1u8; 32])),
            ttl,
            expires_at + 1
        ));
    }

    /// Before DPNS v3 a preorder hashes a fresh salt, so none repeats: nothing is queried
    #[tokio::test]
    async fn should_not_look_for_a_pending_salted_preorder() {
        let owner = Identifier::new([1u8; 32]);
        let mut salted = bob_v3_preorder(owner);
        let Document::V0(ref mut v0) = salted;
        v0.properties =
            BTreeMap::from([("saltedDomainHash".to_string(), Value::Bytes32([5u8; 32]))]);
        // No expectation: a query would fail the call
        let sdk = mock_sdk();
        assert_eq!(
            sdk.pending_dpns_preorder(&dpns_v3(), owner, &salted)
                .await
                .expect("nothing is queried"),
            None
        );
    }

    /// The DPNS contract of `platform_version`, as a proof decodes it under `parsed_under`:
    /// serialized, then rebuilt without validation.
    fn dpns_contract_decoded_under(
        platform_version: &PlatformVersion,
        parsed_under: u32,
    ) -> DataContract {
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("the DPNS contract loads");
        let bytes = contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("the DPNS contract serializes");
        DataContract::versioned_deserialize_untrusted(
            &bytes,
            false,
            PlatformVersion::get(parsed_under).expect("a known protocol version"),
        )
        .expect("the DPNS contract decodes")
    }

    fn proved_at(protocol_version: u32) -> ResponseMetadata {
        ResponseMetadata {
            protocol_version,
            ..Default::default()
        }
    }

    fn declares_the_preorder_hash(dpns_contract: &DataContract) -> bool {
        dpns_contract
            .document_type_for_name("domain")
            .expect("the DPNS contract has a domain type")
            .creator_reference()
            .and_then(|reference| reference.lookup_hash_key())
            .is_some()
    }

    #[tokio::test]
    async fn should_fetch_again_a_dpns_contract_parsed_under_an_older_protocol_version() {
        // The refresh failed: the SDK is still at 13 when the first response, proved at the
        // latest version, is parsed, so what it holds is read under the older version (here
        // DPNS v2, which declares no preorder hash). The response teaches the SDK the latest
        // version, so the next one is parsed under it.
        let sdk_version = Cell::new(13);
        let fetches = Cell::new(0);
        let dpns_contract = dpns_contract_proved_at_parsed_version(
            || sdk_version.get(),
            || {
                fetches.set(fetches.get() + 1);
                let stored_at = if sdk_version.get() == 13 {
                    PlatformVersion::get(13).expect("protocol version 13")
                } else {
                    PlatformVersion::latest()
                };
                let dpns_contract = dpns_contract_decoded_under(stored_at, sdk_version.get());
                sdk_version.set(LATEST_VERSION);
                ready(Ok((Some(dpns_contract), proved_at(LATEST_VERSION))))
            },
        )
        .await
        .expect("the second response is taken");

        assert_eq!(fetches.get(), 2);
        assert!(declares_the_preorder_hash(&dpns_contract));
    }

    #[tokio::test]
    async fn should_skip_a_dpns_contract_proved_by_a_node_behind_the_network() {
        // The SDK learned the latest version; the first node is still at 13 and proves DPNS
        // v2, the next one proves DPNS v3
        let responses = RefCell::new(vec![
            (PlatformVersion::latest(), LATEST_VERSION),
            (PlatformVersion::get(13).expect("protocol version 13"), 13),
        ]);
        let dpns_contract = dpns_contract_proved_at_parsed_version(
            || LATEST_VERSION,
            || {
                let (stored_at, proved) = responses
                    .borrow_mut()
                    .pop()
                    .expect("a response is scripted");
                let dpns_contract = dpns_contract_decoded_under(stored_at, LATEST_VERSION);
                ready(Ok((Some(dpns_contract), proved_at(proved))))
            },
        )
        .await
        .expect("the current node's response is taken");

        assert!(responses.borrow().is_empty());
        assert!(declares_the_preorder_hash(&dpns_contract));
    }

    /// A response holding the DPNS contract of `stored_at`, proved at `proved`, as a fetch
    /// under protocol version `parsed_under` returns it.
    fn dpns_response(
        stored_at: u32,
        proved: u32,
        parsed_under: u32,
    ) -> Result<(Option<DataContract>, ResponseMetadata), Error> {
        let stored_at = PlatformVersion::get(stored_at).expect("a known protocol version");
        Ok((
            Some(dpns_contract_decoded_under(stored_at, parsed_under)),
            proved_at(proved),
        ))
    }

    /// The network's contract, read to judge whether it declares tombstones, by an SDK pinned
    /// to protocol version 14 that a verified response has shown a network at the latest
    /// version. A node still at 14 answers first with the DPNS v2 the upgrade replaced, which
    /// is not taken as the network's; the fetch is made again, and the next node's response
    /// at the latest version is taken, although it is not at the version the SDK is pinned
    /// to. Its DPNS v3 declares tombstones. The registration's fetch, which takes only a
    /// response at the pinned version, is not what answers.
    #[tokio::test]
    async fn should_fetch_again_past_a_node_behind_the_network() {
        let responses = RefCell::new(vec![
            dpns_response(LATEST_VERSION, LATEST_VERSION, LATEST_VERSION),
            dpns_response(14, 14, 14),
        ]);
        let network_contract = give_up::network_dpns_contract_declaring_tombstones(
            || Some(LATEST_VERSION),
            || {
                ready(
                    responses
                        .borrow_mut()
                        .pop()
                        .expect("a response is scripted"),
                )
            },
            || async { panic!("the registration's fetch is not needed") },
        )
        .await
        .expect("the network's contract is fetched again")
        .expect("the network's contract declares tombstones");

        assert!(responses.borrow().is_empty());
        assert!(network_contract.has_document_type_for_name("tombstone"));
    }

    /// And when only nodes behind the network answer, three times over, the read fails: the
    /// DPNS v2 of a node behind is never the answer.
    #[tokio::test]
    async fn should_fail_when_only_nodes_behind_the_network_answer() {
        let fetches = Cell::new(0);
        let result = give_up::network_dpns_contract_declaring_tombstones(
            || Some(LATEST_VERSION),
            || {
                fetches.set(fetches.get() + 1);
                ready(dpns_response(14, 14, 14))
            },
            || async { panic!("the registration's fetch is not needed") },
        )
        .await;

        assert_eq!(fetches.get(), 3);
        // A stale node error, which a caller may retry, naming the versions
        let error = result.expect_err("no response at the network's version came");
        assert!(error.can_retry(), "{error}");
        assert!(
            matches!(
                error,
                Error::StaleNode(StaleNodeError::BehindProtocolUpgrade {
                    required: LATEST_VERSION,
                    signed: 14,
                })
            ),
            "{error}"
        );
    }

    /// A fetch that fails is made as a registration makes it. For an SDK pinned to protocol
    /// version 14, that fetch takes a response proved at 14, the version it is pinned to and
    /// parses under: the DPNS v2 of a node behind the network. A contract declaring no
    /// tombstone is still not the network's once a verified response has shown the network at
    /// the latest version: the read fails.
    #[tokio::test]
    async fn should_fail_when_the_registration_fetch_of_a_pinned_sdk_takes_the_replaced_contract() {
        let registration_fetches = Cell::new(0);
        let result = give_up::network_dpns_contract_declaring_tombstones(
            || Some(LATEST_VERSION),
            || ready(Err(Error::Generic("the fetch fails".to_string()))),
            || {
                dpns_contract_proved_at_parsed_version(
                    || 14,
                    || {
                        registration_fetches.set(registration_fetches.get() + 1);
                        ready(dpns_response(14, 14, 14))
                    },
                )
            },
        )
        .await;

        assert_eq!(
            registration_fetches.get(),
            1,
            "the pinned SDK takes the response proved at its own version"
        );
        assert!(result.is_err());
    }

    /// A response at the highest version a verified response reported is the network's, and
    /// so is one when nothing was verified yet: on a network at protocol version 14 the
    /// contract is taken as fetched, DPNS v2, and the network stores no tombstone.
    #[tokio::test]
    async fn should_report_no_tombstones_on_a_network_at_protocol_version_14() {
        for verified_version in [Some(14), None] {
            let network_contract = give_up::network_dpns_contract_declaring_tombstones(
                || verified_version,
                || ready(dpns_response(14, 14, 14)),
                || async { panic!("the registration's fetch is not needed") },
            )
            .await
            .expect("the network's contract answers");

            assert!(network_contract.is_none());
        }
    }

    /// A tombstone's absence, with the block its response proved it at.
    fn no_tombstone_at(height: u64) -> Result<(Option<Document>, ResponseMetadata), Error> {
        Ok((
            None,
            ResponseMetadata {
                height,
                ..Default::default()
            },
        ))
    }

    /// A name's domain is proved absent at block 101, where a tombstone create deleted it.
    /// A node one block behind then proves the name has no tombstone, at block 100, where the
    /// domain still stood: the two absences are of different blocks and do not make the name
    /// free. The tombstone is read again, and the next node, at block 101, shows it.
    #[tokio::test]
    async fn should_read_a_tombstone_again_when_its_absence_is_older_than_the_domains() {
        let responses = RefCell::new(vec![
            Ok((
                Some(bob_tombstone()),
                ResponseMetadata {
                    height: 101,
                    ..Default::default()
                },
            )),
            no_tombstone_at(100),
        ]);
        let tombstoned = give_up::name_tombstoned_not_before(101, || {
            ready(
                responses
                    .borrow_mut()
                    .pop()
                    .expect("a response is scripted"),
            )
        })
        .await
        .expect("the tombstone is read again");

        assert!(responses.borrow().is_empty());
        assert!(tombstoned);
    }

    /// And when every absence comes from a block before the domain's, three times over, the
    /// read fails instead of answering "not tombstoned".
    #[tokio::test]
    async fn should_fail_when_every_tombstone_absence_is_older_than_the_domains() {
        let reads = Cell::new(0);
        let result = give_up::name_tombstoned_not_before(101, || {
            reads.set(reads.get() + 1);
            ready(no_tombstone_at(100))
        })
        .await;

        assert_eq!(reads.get(), 3);
        // A stale node error, which a caller may retry, naming the blocks
        let error = result.expect_err("no absence at the domain's block came");
        assert!(error.can_retry(), "{error}");
        assert!(
            matches!(
                error,
                Error::StaleNode(StaleNodeError::Height {
                    expected_height: 101,
                    received_height: 100,
                    tolerance_blocks: 0,
                })
            ),
            "{error}"
        );
    }

    /// An absence proved at the domain's block or a later one answers at once. A tombstone
    /// found answers whatever block it was read at, as a tombstone never leaves.
    #[tokio::test]
    async fn should_take_a_tombstone_absence_no_older_than_the_domains() {
        for proved_at_height in [101, 102] {
            let reads = Cell::new(0);
            let tombstoned = give_up::name_tombstoned_not_before(101, || {
                reads.set(reads.get() + 1);
                ready(no_tombstone_at(proved_at_height))
            })
            .await
            .expect("the absence answers");

            assert_eq!(reads.get(), 1);
            assert!(!tombstoned);
        }

        let tombstoned = give_up::name_tombstoned_not_before(101, || {
            ready(Ok((
                Some(bob_tombstone()),
                ResponseMetadata {
                    height: 100,
                    ..Default::default()
                },
            )))
        })
        .await
        .expect("the tombstone answers");
        assert!(tombstoned);
    }

    #[tokio::test]
    async fn should_skip_a_dpns_contract_proved_while_another_request_raised_the_version() {
        // The SDK is at 13 when the fetch starts. While it waits, another request on a clone
        // teaches the shared SDK the latest version, and the response comes from a node still
        // at 13 proving DPNS v2: it matches the version the fetch started at, not the one the
        // SDK has learned since.
        let sdk_version = Cell::new(13);
        let fetches = Cell::new(0);
        let dpns_contract = dpns_contract_proved_at_parsed_version(
            || sdk_version.get(),
            || {
                fetches.set(fetches.get() + 1);
                let response = if fetches.get() == 1 {
                    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
                    let dpns_v2 = dpns_contract_decoded_under(platform_version, 13);
                    sdk_version.set(LATEST_VERSION);
                    (Some(dpns_v2), proved_at(13))
                } else {
                    let dpns_v3 =
                        dpns_contract_decoded_under(PlatformVersion::latest(), sdk_version.get());
                    (Some(dpns_v3), proved_at(LATEST_VERSION))
                };
                ready(Ok(response))
            },
        )
        .await
        .expect("the response proved at the learned version is taken");

        assert_eq!(fetches.get(), 2);
        assert!(declares_the_preorder_hash(&dpns_contract));
    }

    #[tokio::test]
    async fn should_refuse_when_every_dpns_contract_response_is_behind_the_network() {
        let fetches = Cell::new(0);
        let result = dpns_contract_proved_at_parsed_version(
            || LATEST_VERSION,
            || {
                fetches.set(fetches.get() + 1);
                let platform_version = PlatformVersion::get(13).expect("protocol version 13");
                let dpns_contract = dpns_contract_decoded_under(platform_version, LATEST_VERSION);
                ready(Ok((Some(dpns_contract), proved_at(13))))
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(fetches.get(), DPNS_CONTRACT_FETCH_ATTEMPTS);
    }

    #[test]
    fn test_extract_dpns_label() {
        assert_eq!(extract_dpns_label("alice.dash"), "alice");
        assert_eq!(extract_dpns_label("alice.DASH"), "alice");
        assert_eq!(extract_dpns_label("alice.DaSh"), "alice");
        assert_eq!(extract_dpns_label("Alice.DASH"), "Alice");
        assert_eq!(extract_dpns_label("alice"), "alice");
        assert_eq!(extract_dpns_label("alice.eth"), "alice.eth");
        assert_eq!(extract_dpns_label(".dash"), "");
    }
}
