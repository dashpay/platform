mod contested_queries;
mod queries;

pub use contested_queries::ContestedDpnsUsername;
pub use dash_platform_queries::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
};
pub use queries::DpnsUsername;

use crate::platform::transition::put_document::PutDocument;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::{DataContract, Document, Fetch, FetchMany};
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::{DocumentV0, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use std::collections::BTreeMap;
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
/// (e.g. `"a11ce"`).
fn normalize_dpns_label(input: &str) -> String {
    convert_to_homograph_safe_chars(extract_dpns_label(input))
}

/// Hash a buffer twice using SHA256 (double SHA256)
fn hash_double(data: Vec<u8>) -> [u8; 32] {
    use dpp::dashcore::hashes::{sha256d, Hash};
    // sha256d already does double SHA256
    let hash = sha256d::Hash::hash(&data);
    hash.to_byte_array()
}

/// The `saltedDomainHash` a preorder by `owner_id` commits to, for a domain
/// carrying `domain_properties` (its `preorderSalt`, labels and parent), as the
/// DPNS contract the network runs declares it. From DPNS v3 the domain's
/// `preorderSalt` reveals the hash of the writer's id, the salt, the normalized
/// label, `"."` and the parent, and the declaration computes it, as the platform
/// does; before it the salt carries no reference and the hash is of the salt and
/// `<normalizedLabel>.dash`. A reference without a `findBy` function is an error,
/// never the older hash, which such a contract could not reveal.
fn salted_domain_hash(
    domain_document_type: DocumentTypeRef,
    owner_id: Identifier,
    salt: [u8; 32],
    normalized_label: &str,
    domain_properties: &BTreeMap<String, Value>,
) -> Result<[u8; 32], Error> {
    let Some(reference) = domain_document_type
        .flattened_properties()
        .get("preorderSalt")
        .and_then(|property| property.revealed_reference.as_ref())
    else {
        let mut salted_domain_buffer = salt.to_vec();
        salted_domain_buffer.extend(format!("{normalized_label}.dash").as_bytes());
        return Ok(hash_double(salted_domain_buffer));
    };
    let key = reference
        .as_any_document_reference()
        .and_then(|declaration| declaration.lookup)
        .and_then(|lookup| lookup.hash_key())
        .map(|(_, key)| key)
        .ok_or_else(|| {
            Error::Generic(
                "the DPNS domain's preorderSalt reference declares no findBy function".to_string(),
            )
        })?;
    key.key_value(domain_document_type, None, owner_id, domain_properties)
        .map(|(hash, _)| hash)
        .map_err(|error| {
            Error::Generic(format!(
                "cannot compute the DPNS preorder hash: {} {}",
                error.param, error.reason
            ))
        })
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
    /// The preorder document that was created. From protocol version 14 the domain
    /// create deletes it, so it is no longer in state once the name is registered
    pub preorder_document: Document,
    /// The domain document that was created
    pub domain_document: Document,
    /// The full domain name (e.g., "alice.dash")
    pub full_domain_name: String,
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
    /// refresh), so the contract is parsed under it; when the refresh failed and the fetch's
    /// own response teaches the SDK a newer version, that contract was parsed under the older
    /// one and is fetched again.
    async fn fetch_current_dpns_contract(&self) -> Result<DataContract, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;
        let protocol_version = self.refresh_protocol_version().await?;
        let mut dpns_contract = DataContract::fetch(self, dpns_contract_id).await?;
        if self.protocol_version_number() != protocol_version {
            dpns_contract = DataContract::fetch(self, dpns_contract_id).await?;
        }
        dpns_contract.ok_or_else(|| Error::Generic("DPNS contract not found".to_string()))
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
        let dpns_contract = self.fetch_current_dpns_contract().await?;

        // Get document types
        let preorder_document_type = dpns_contract
            .document_type_for_name("preorder")
            .map_err(|_| Error::Generic("DPNS preorder document type not found".to_string()))?;

        let domain_document_type = dpns_contract
            .document_type_for_name("domain")
            .map_err(|_| Error::Generic("DPNS domain document type not found".to_string()))?;

        // Generate the preorder salt
        let mut rng = StdRng::from_entropy();
        let salt: [u8; 32] = rng.gen();

        // The id of a new document commits to the identity contract nonce of
        // its create transition, so it only exists once `put_to_platform` has
        // fetched that nonce. The documents are built with a placeholder id;
        // the confirmed documents carry the real one. Nothing here needs the
        // ids up front: the domain is tied to its preorder by the salt.
        let identity_id = input.identity.id().to_owned();
        let preorder_id = Identifier::default();
        let domain_id = Identifier::default();

        let normalized_label = convert_to_homograph_safe_chars(&input.label);
        let domain_properties = BTreeMap::from([
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("label".to_string(), Value::Text(input.label.clone())),
            (
                "normalizedLabel".to_string(),
                Value::Text(normalized_label.clone()),
            ),
            ("preorderSalt".to_string(), Value::Bytes32(salt)),
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

        // The salted domain hash the preorder commits to
        let salted_domain_hash = salted_domain_hash(
            domain_document_type,
            identity_id,
            salt,
            &normalized_label,
            &domain_properties,
        )?;

        // Create preorder document
        let preorder_document = Document::V0(DocumentV0 {
            contract_version: None,
            id: preorder_id,
            owner_id: identity_id,
            properties: BTreeMap::from([(
                "saltedDomainHash".to_string(),
                Value::Bytes32(salted_domain_hash),
            )]),
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

        // Submit preorder document first
        debug!(%identity_id, stage = "preorder", "DPNS registration: submitting document");
        let platform_preorder_document = preorder_document
            .put_to_platform_and_wait_for_response(
                self,
                preorder_document_type.to_owned_document_type(),
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
        let platform_domain_document = domain_document
            .put_to_platform_and_wait_for_response(
                self,
                domain_document_type.to_owned_document_type(),
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

        Ok(RegisterDpnsNameResult {
            preorder_document: platform_preorder_document,
            domain_document: platform_domain_document,
            full_domain_name: format!("{}.dash", normalized_label),
        })
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
    /// Returns `true` if the name is available, `false` if it's taken
    pub async fn is_dpns_name_available(&self, name: &str) -> Result<bool, Error> {
        use crate::platform::documents::document_query::DocumentQuery;
        use drive::query::WhereClause;
        use drive::query::WhereOperator;

        let normalized_label = normalize_dpns_label(name);

        // An empty normalized label (e.g. `""`, `".dash"`, `".DASH"`) is not
        // a registrable DPNS name, so report it as unavailable rather than
        // doing a network round-trip that would query for
        // `normalizedLabel == ""`. This mirrors the early-return guard in
        // `resolve_dpns_name` so the two APIs agree on malformed input.
        if normalized_label.is_empty() {
            return Ok(false);
        }

        let dpns_contract = self.fetch_dpns_contract().await?;

        // Query for existing domain with this label
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

        // `Document::fetch_many` returns `BTreeMap<Identifier, Option<Document>>`
        // — a non-existence proof comes back as a non-empty map whose values are
        // all `None`. Checking `documents.is_empty()` would treat a proven
        // non-existence as "taken". The name is available iff no entry in the
        // map carries an actual document.
        Ok(documents.values().all(|d| d.is_none()))
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
    use crate::sdk::test::expect_epoch_refresh;
    use crate::SdkBuilder;
    use dash_context_provider::{ContextProvider, ContextProviderError};
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::CoreBlockHeight;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::{PlatformVersion, LATEST_VERSION};

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

    /// The properties of a domain `Bob.dash` the preorder hash may read.
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

    /// The preorder hash the SDK computes for `owner_id` and `properties` against the DPNS
    /// contract of `platform_version`.
    fn preorder_hash(
        platform_version: &PlatformVersion,
        owner_id: Identifier,
        properties: &BTreeMap<String, Value>,
    ) -> Result<[u8; 32], Error> {
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("the DPNS contract loads");
        let domain = contract
            .document_type_for_name("domain")
            .expect("the DPNS contract has a domain type");
        salted_domain_hash(domain, owner_id, SALT, "b0b", properties)
    }

    fn sha256d(parts: &[&[u8]]) -> [u8; 32] {
        use dpp::dashcore::hashes::{sha256d, Hash};
        sha256d::Hash::hash(&parts.concat()).to_byte_array()
    }

    #[test]
    fn should_hash_the_salt_and_name_whoever_the_owner_for_dpns_v2() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let expected = sha256d(&[&SALT, b"b0b.dash"]);
        for owner_id in [Identifier::new([1u8; 32]), Identifier::new([2u8; 32])] {
            assert_eq!(
                preorder_hash(platform_version, owner_id, &bob_domain_properties())
                    .expect("DPNS v2 hashes the salt and the name"),
                expected
            );
        }
    }

    #[test]
    fn should_hash_the_owner_salt_and_name_the_dpns_v3_contract_declares() {
        let platform_version = PlatformVersion::latest();
        for owner_id in [Identifier::new([1u8; 32]), Identifier::new([2u8; 32])] {
            assert_eq!(
                preorder_hash(platform_version, owner_id, &bob_domain_properties())
                    .expect("DPNS v3 hashes the declared params"),
                sha256d(&[owner_id.as_slice(), &SALT, b"b0b", b".", b"dash"]),
            );
        }
    }

    #[test]
    fn should_refuse_a_dpns_v3_domain_missing_a_hashed_property() {
        let mut properties = bob_domain_properties();
        properties.remove("normalizedLabel");
        assert!(preorder_hash(
            PlatformVersion::latest(),
            Identifier::new([1u8; 32]),
            &properties
        )
        .is_err());
    }

    /// A context provider seeded before the upgrade to DPNS v3: it holds DPNS v2 whatever
    /// protocol version it is asked for, as a provider's known contracts do.
    struct DpnsV2ContextProvider;

    impl ContextProvider for DpnsV2ContextProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
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
    }

    #[tokio::test]
    async fn should_register_against_the_dpns_contract_the_network_stores() {
        // An SDK seeded at protocol version 13, whose context provider holds DPNS v2, on a
        // network that runs the latest version and stores DPNS v3
        let mut sdk = SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(13).expect("protocol version 13"))
            .with_context_provider(DpnsV2ContextProvider)
            .build()
            .expect("the mock SDK builds");
        expect_epoch_refresh(&mut sdk).await;
        let stored = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("the DPNS contract loads");
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DPNS fetch is expected");

        let dpns_contract = sdk
            .fetch_current_dpns_contract()
            .await
            .expect("the DPNS contract is fetched");

        assert_eq!(sdk.protocol_version_number(), LATEST_VERSION);
        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("the DPNS contract has a domain type");
        let owner_id = Identifier::new([1u8; 32]);
        assert_eq!(
            salted_domain_hash(domain, owner_id, SALT, "b0b", &bob_domain_properties())
                .expect("the stored contract declares the hash"),
            sha256d(&[owner_id.as_slice(), &SALT, b"b0b", b".", b"dash"]),
        );
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
