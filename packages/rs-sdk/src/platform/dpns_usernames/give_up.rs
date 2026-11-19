//! Giving up a DPNS name, from DPNS v3 (protocol version 15). The identity holding a name
//! throws it away one of two ways:
//!
//! - a delete ([`Sdk::delete_dpns_name`]) removes the `domain` document, and anybody may
//!   register the name again. A contested name, one a masternode vote awarded, can not be
//!   deleted.
//! - a tombstone ([`Sdk::tombstone_dpns_name`]) creates a `tombstone` document of the name,
//!   which deletes the `domain` document with it and keeps the name from being registered
//!   while the tombstone exists. Nobody can delete a tombstone; the contract says that a
//!   tombstoned name may later be brought back and redistributed through a direct or indirect
//!   will of the DAO.

use super::normalize_dpns_label;
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::transitions::{
    DocumentDeleteResult, DocumentDeleteTransitionBuilder,
};
use crate::platform::transition::put_document::PutDocument;
use crate::platform::{DataContract, Document, Fetch, FetchMany};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::ResponseMetadata;
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::version::{FeatureVersion, PlatformVersion};
use drive::query::{SelectProjection, WhereClause, WhereOperator};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

/// The DPNS document type of a registered name.
const DOMAIN: &str = "domain";

/// The DPNS document type of a name given up for good, from DPNS v3.
const TOMBSTONE: &str = "tombstone";

/// The first DPNS contract version, as a platform version's `system_data_contracts.dpns`
/// numbers them, that declares the `tombstone` type.
const FIRST_DPNS_VERSION_WITH_TOMBSTONES: FeatureVersion = 3;

/// How many times a read is made when a node behind what the SDK has already verified
/// answers it: another node usually answers the next one.
const READ_ATTEMPTS: usize = 3;

/// The query for the `document_type_name` document (a `domain` or a `tombstone`) of the name
/// `normalized_label` under `dash`: at most one exists.
pub(super) fn dpns_name_query(
    dpns_contract: Arc<DataContract>,
    document_type_name: &str,
    normalized_label: String,
) -> DocumentQuery {
    DocumentQuery {
        select: SelectProjection::documents(),
        data_contract: dpns_contract,
        document_type_name: document_type_name.to_string(),
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
    }
}

/// The network's DPNS contract from `fetch`, asked again, [`READ_ATTEMPTS`] times at most,
/// while a node behind the network answers, and an error when only such nodes do. When
/// `fetch` fails, the contract comes from `fetch_as_a_registration`, whose error is returned
/// when it fails too.
///
/// A node that has not yet reached an upgrade still proves the contract the upgrade replaced,
/// at its own, older protocol version. Its response is behind the network when that version
/// is below `verified_version`, the highest a verified response has reported, read once the
/// fetch has answered. Taken as the network's, a DPNS v2 from such a node would say the
/// network stores no tombstone. Every response is judged by that version alone, never by the
/// version the SDK is pinned to: a response at the network's version is taken by an SDK
/// pinned below it too.
///
/// A fetch fails when the contract can not be parsed under the SDK's version, among other
/// causes. A registration's fetch first learns the network's version and takes only a
/// response proved at the version the SDK parsed it under
/// ([`super::dpns_contract_proved_at_parsed_version`]).
pub(super) async fn network_dpns_contract<V, F, R, C, RC>(
    verified_version: V,
    mut fetch: F,
    fetch_as_a_registration: C,
) -> Result<DataContract, Error>
where
    V: Fn() -> Option<u32>,
    F: FnMut() -> R,
    R: Future<Output = Result<(Option<DataContract>, ResponseMetadata), Error>>,
    C: FnOnce() -> RC,
    RC: Future<Output = Result<DataContract, Error>>,
{
    for _ in 0..READ_ATTEMPTS {
        let Ok((dpns_contract, metadata)) = fetch().await else {
            return fetch_as_a_registration().await;
        };
        if verified_version().is_some_and(|verified| metadata.protocol_version < verified) {
            continue;
        }
        return dpns_contract.ok_or_else(|| Error::Generic("DPNS contract not found".to_string()));
    }
    Err(Error::Generic(
        "every response holding the DPNS contract was proved by a node behind the network"
            .to_string(),
    ))
}

/// Whether a name is tombstoned, from `read`: the query for its tombstone, answered with the
/// tombstone, if any, and the metadata of the response.
///
/// A tombstone found is the answer whatever block it was read at, as a tombstone never
/// leaves. Its absence is the answer only when proved at `not_before_height` or later, the
/// block the name's domain was proved absent at: a tombstone create deletes the domain in
/// its own block, so a domain absent at one block and a tombstone absent at an earlier one
/// can be a name that was registered then and is tombstoned now, never free. An absence
/// proved earlier is read again, [`READ_ATTEMPTS`] times at most, and is then an error:
/// never "not tombstoned". Without a `not_before_height` the first answer is taken.
pub(super) async fn name_tombstoned_not_before<F, R>(
    not_before_height: Option<u64>,
    mut read: F,
) -> Result<bool, Error>
where
    F: FnMut() -> R,
    R: Future<Output = Result<(Option<Document>, ResponseMetadata), Error>>,
{
    for _ in 0..READ_ATTEMPTS {
        let (tombstone, metadata) = read().await?;
        if tombstone.is_some() {
            return Ok(true);
        }
        if !not_before_height.is_some_and(|height| metadata.height < height) {
            return Ok(false);
        }
    }
    Err(Error::Generic(
        "every response proving the name has no tombstone was older than the one proving it \
         has no domain"
            .to_string(),
    ))
}

/// Whether the DPNS contract of `protocol_version` declares tombstones. A version this build
/// does not know is newer than every one it knows, whose contract does.
fn protocol_version_declares_tombstones(protocol_version: u32) -> bool {
    PlatformVersion::get(protocol_version).map_or(true, |platform_version| {
        platform_version.system_data_contracts.dpns >= FIRST_DPNS_VERSION_WITH_TOMBSTONES
    })
}

/// Whether tombstones are read under the contract fetched as the network's, which
/// `declares_tombstones` or not, given what a verified response showed of the network
/// (`verified_declares_tombstones`: whether the DPNS contract of the highest protocol version
/// one reported declares them, `None` when none was verified).
///
/// A contract declaring none is the network's only while nothing verified shows the network
/// past the upgrade to DPNS v3. Once a verified response does, such a contract was answered
/// by a node behind the network, however it was fetched (an SDK pinned to the older version
/// takes that node's response as proved at its own version), and is an error: never "the
/// network stores no tombstone".
pub(super) fn network_contract_declares_tombstones(
    declares_tombstones: bool,
    verified_declares_tombstones: Option<bool>,
) -> Result<bool, Error> {
    if !declares_tombstones && verified_declares_tombstones == Some(true) {
        return Err(Error::Generic(
            "the DPNS contract read as the network's declares no tombstone, while a verified \
             response shows the network runs a protocol version whose DPNS contract does: it \
             was answered by a node behind the network"
                .to_string(),
        ));
    }
    Ok(declares_tombstones)
}

/// The network's DPNS contract when it declares tombstones, `None` when the network stores
/// none: fetched by [`network_dpns_contract`], then judged by
/// [`network_contract_declares_tombstones`] against `verified_version` as it stands once the
/// fetch has answered.
pub(super) async fn network_dpns_contract_declaring_tombstones<V, F, R, C, RC>(
    verified_version: V,
    fetch: F,
    fetch_as_a_registration: C,
) -> Result<Option<DataContract>, Error>
where
    V: Fn() -> Option<u32>,
    F: FnMut() -> R,
    R: Future<Output = Result<(Option<DataContract>, ResponseMetadata), Error>>,
    C: FnOnce() -> RC,
    RC: Future<Output = Result<DataContract, Error>>,
{
    let network_contract =
        network_dpns_contract(&verified_version, fetch, fetch_as_a_registration).await?;
    let declares_tombstones = network_contract_declares_tombstones(
        network_contract.has_document_type_for_name(TOMBSTONE),
        verified_version().map(protocol_version_declares_tombstones),
    )?;
    Ok(declares_tombstones.then_some(network_contract))
}

impl Sdk {
    /// The `document_type_name` document of the name `normalized_label` under `dash`, if the
    /// network holds one. A proven absence comes back as entries holding `None`.
    pub(super) async fn dpns_name_document(
        &self,
        dpns_contract: Arc<DataContract>,
        document_type_name: &str,
        normalized_label: String,
    ) -> Result<Option<Document>, Error> {
        self.dpns_name_document_with_metadata(dpns_contract, document_type_name, normalized_label)
            .await
            .map(|(document, _metadata)| document)
    }

    /// [`Sdk::dpns_name_document`], with the metadata of the response: the block the
    /// document, or its absence, was proved at.
    pub(super) async fn dpns_name_document_with_metadata(
        &self,
        dpns_contract: Arc<DataContract>,
        document_type_name: &str,
        normalized_label: String,
    ) -> Result<(Option<Document>, ResponseMetadata), Error> {
        let query = dpns_name_query(dpns_contract, document_type_name, normalized_label);
        let (documents, metadata) = Document::fetch_many_with_metadata(self, query, None).await?;
        Ok((documents.into_values().flatten().next(), metadata))
    }

    /// Whether the DPNS contract of the highest protocol version a verified response ever
    /// reported declares tombstones, whatever the SDK's own version is pinned to. `None`
    /// when no response was verified (an SDK with proofs off verifies none), so not knowing
    /// never reads as "before DPNS v3".
    fn verified_network_declares_tombstones(&self) -> Option<bool> {
        self.protocol_version_info()
            .verified_version
            .map(protocol_version_declares_tombstones)
    }

    /// The DPNS contract the network stores now when it declares tombstones, `None` when the
    /// network stores none.
    ///
    /// The SDK first learns the network's protocol version where it can (a proven refresh,
    /// which does nothing on an SDK with a pinned version or with proofs off), so a contract
    /// stored at a newer version is not parsed under an older one. The contract is then
    /// fetched as any contract is, again while a node behind the network answers; when the
    /// fetch fails it is fetched as a registration fetches it
    /// ([`Sdk::fetch_current_dpns_contract`]), whose error is returned when that fails too.
    /// Whichever fetch answered, a contract declaring no tombstone is an error once a
    /// verified response shows the network past the upgrade
    /// ([`network_dpns_contract_declaring_tombstones`]).
    pub(super) async fn fetch_network_dpns_contract_declaring_tombstones(
        &self,
    ) -> Result<Option<DataContract>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;
        self.refresh_protocol_version().await?;
        network_dpns_contract_declaring_tombstones(
            || self.protocol_version_info().verified_version,
            || DataContract::fetch_with_metadata(self, dpns_contract_id, None),
            || self.fetch_current_dpns_contract(),
        )
        .await
    }

    /// The DPNS contract to read a name's tombstone under, or `None` when the network's
    /// contract declares no tombstone (before DPNS v3).
    ///
    /// `dpns_contract` is the one the SDK resolves for reads, which a context provider may
    /// still hold from before the upgrade to DPNS v3 (a known-contracts cache warmed earlier).
    /// Such a contract declares no tombstone while the network stores them, so a tombstoned
    /// name would read as free. A contract without the type is therefore taken as the
    /// network's only when a verified response of this call
    /// (`verified_response_in_this_call`) proved that the network runs a protocol version
    /// before DPNS v3. In every other case what the network runs is not known from the
    /// contract in hand (the response may be older than an upgrade, the SDK's version may be
    /// pinned, or proofs may be off), so the network is asked for its contract, which is
    /// handed to the provider when it declares tombstones, as a registration does. A failed
    /// fetch is an error, never "no tombstone", and so is a fetched contract declaring none
    /// once a verified response shows the network past the upgrade
    /// ([`Sdk::fetch_network_dpns_contract_declaring_tombstones`]).
    async fn dpns_contract_declaring_tombstones(
        &self,
        dpns_contract: Arc<DataContract>,
        verified_response_in_this_call: bool,
    ) -> Result<Option<Arc<DataContract>>, Error> {
        if dpns_contract.has_document_type_for_name(TOMBSTONE) {
            return Ok(Some(dpns_contract));
        }
        if verified_response_in_this_call
            && self.verified_network_declares_tombstones() == Some(false)
        {
            return Ok(None);
        }
        let Some(network_contract) = self
            .fetch_network_dpns_contract_declaring_tombstones()
            .await?
            .map(Arc::new)
        else {
            return Ok(None);
        };
        if let Some(provider) = self.context_provider() {
            provider.register_data_contract(network_contract.clone());
        }
        Ok(Some(network_contract))
    }

    /// Whether the name `normalized_label` under `dash` is tombstoned, read under
    /// `dpns_contract` or, when that one declares no tombstone, under the contract the
    /// network stores ([`Sdk::dpns_contract_declaring_tombstones`]). Never before DPNS v3,
    /// whose contract declares no tombstone.
    ///
    /// `domain_absent_at_height` is the block a verified response of this call proved the
    /// name's domain absent at, `None` when the call read no domain. It says a response of
    /// this call showed what the network runs, and holds the tombstone's absence to a block
    /// no earlier ([`name_tombstoned_not_before`]).
    pub(super) async fn dpns_name_tombstoned_under(
        &self,
        dpns_contract: Arc<DataContract>,
        normalized_label: String,
        domain_absent_at_height: Option<u64>,
    ) -> Result<bool, Error> {
        let Some(dpns_contract) = self
            .dpns_contract_declaring_tombstones(dpns_contract, domain_absent_at_height.is_some())
            .await?
        else {
            return Ok(false);
        };
        name_tombstoned_not_before(domain_absent_at_height, || {
            self.dpns_name_document_with_metadata(
                dpns_contract.clone(),
                TOMBSTONE,
                normalized_label.clone(),
            )
        })
        .await
    }

    /// Whether a DPNS name is tombstoned: its owner gave it up for good, and nobody can
    /// register it while the tombstone exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The username label (e.g., "alice") or full DPNS name (e.g., "alice.dash")
    ///
    /// # Errors
    ///
    /// Returns an error, never `false`, when the SDK resolves a DPNS contract declaring no
    /// tombstone and the contract the network stores can not be read.
    pub async fn is_dpns_name_tombstoned(&self, name: &str) -> Result<bool, Error> {
        let normalized_label = normalize_dpns_label(name);
        if normalized_label.is_empty() {
            return Ok(false);
        }
        let dpns_contract = self.fetch_dpns_contract().await?;
        // Nothing this call has read says what the network runs
        self.dpns_name_tombstoned_under(dpns_contract, normalized_label, None)
            .await
    }

    /// Deletes a DPNS name its owner no longer wants. The name is free again: anybody may
    /// register it. The owner is refunded what is left of the name's storage fee.
    ///
    /// A contested name, one a masternode vote awarded, can not be deleted, only tombstoned
    /// ([`Sdk::tombstone_dpns_name`]): the network refuses the delete, so this refuses it
    /// before anything is broadcast.
    ///
    /// # Arguments
    ///
    /// * `name` - The username label (e.g., "alice") or full DPNS name (e.g., "alice.dash")
    /// * `identity_public_key` - The key of the name's owner that signs the delete
    /// * `signer` - The signer holding that key
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not registered, is contested, or the delete is refused
    /// (the signing identity does not hold the name, or the network runs a protocol version
    /// before 15, where a name can not be deleted).
    pub async fn delete_dpns_name<S: Signer<IdentityPublicKey>>(
        &self,
        name: &str,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentDeleteResult, Error> {
        let normalized_label = normalize_dpns_label(name);
        let dpns_contract = self.fetch_dpns_contract().await?;
        let Some(domain) = self
            .dpns_name_document(dpns_contract.clone(), DOMAIN, normalized_label.clone())
            .await?
        else {
            return Err(Error::Generic(format!(
                "the DPNS name {normalized_label}.dash is not registered"
            )));
        };
        let is_contested = dpns_contract
            .document_type_for_name(DOMAIN)
            .map_err(|_| Error::Generic("DPNS domain document type not found".to_string()))?
            .contested_vote_poll_for_document(&domain, self.version())?
            .is_some();
        if is_contested {
            return Err(Error::Generic(format!(
                "the DPNS name {normalized_label}.dash is contested: it can not be deleted, \
                 only tombstoned"
            )));
        }
        self.document_delete(
            DocumentDeleteTransitionBuilder::from_document(
                dpns_contract,
                DOMAIN.to_string(),
                &domain,
            ),
            identity_public_key,
            signer,
        )
        .await
    }

    /// Tombstones a DPNS name its owner no longer wants: the name is deleted and nobody can
    /// register it again while the tombstone exists. This can not be undone by its owner or
    /// by anybody else: the DPNS contract says only that a tombstoned name may later be
    /// brought back and redistributed through a direct or indirect will of the DAO. The owner
    /// is refunded what is left of the name's storage fee and pays for the tombstone's.
    ///
    /// Any name can be tombstoned, a contested one included.
    ///
    /// # Arguments
    ///
    /// * `name` - The username label (e.g., "alice") or full DPNS name (e.g., "alice.dash")
    /// * `owner_id` - The identity holding the name
    /// * `identity_public_key` - The key of that identity that signs the tombstone
    /// * `signer` - The signer holding that key
    ///
    /// # Returns
    ///
    /// The tombstone document the network stored.
    ///
    /// # Errors
    ///
    /// Returns an error if the network's DPNS contract declares no tombstone (before protocol
    /// version 15), or the tombstone is refused (the name is not registered, or `owner_id`
    /// does not hold it).
    pub async fn tombstone_dpns_name<S: Signer<IdentityPublicKey>>(
        &self,
        name: &str,
        owner_id: Identifier,
        identity_public_key: IdentityPublicKey,
        signer: &S,
    ) -> Result<Document, Error> {
        let normalized_label = normalize_dpns_label(name);
        // The contract the network stores now, as for a registration: one cached before the
        // upgrade to DPNS v3 declares no tombstone
        let dpns_contract = self.fetch_current_dpns_contract().await?;
        // The confirmation is a proof the SDK decodes under the DPNS contract its context
        // provider resolves: give it the contract the tombstone is built against
        if let Some(provider) = self.context_provider() {
            provider.register_data_contract(Arc::new(dpns_contract.clone()));
        }
        let tombstone_document_type = dpns_contract
            .document_type_for_name(TOMBSTONE)
            .map_err(|_| {
                Error::Generic(
                    "the network's DPNS contract declares no tombstone: names can be tombstoned \
                     from protocol version 15"
                        .to_string(),
                )
            })?
            .to_owned_document_type();

        // The id of a new document commits to the identity contract nonce of its create
        // transition, so it is a placeholder here; the confirmed document carries the real one
        let tombstone = Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::default(),
            owner_id,
            properties: BTreeMap::from([
                (
                    "normalizedParentDomainName".to_string(),
                    Value::Text("dash".to_string()),
                ),
                ("normalizedLabel".to_string(), Value::Text(normalized_label)),
            ]),
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

        tombstone
            .put_to_platform_and_wait_for_response(
                self,
                tombstone_document_type,
                None, // entropy: generated together with the id once the nonce is known
                identity_public_key,
                None, // token payment info
                signer,
                None, // settings
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_types::Documents;
    use crate::SdkBuilder;
    use dpp::document::DocumentV0Getters;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

    fn dpns_v3() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("the DPNS contract loads"),
        )
    }

    /// A mock SDK at the latest version; a request without an expectation fails.
    fn mock_sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("the mock SDK builds")
    }

    fn name_document(owner: Identifier, normalized_label: &str) -> Document {
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::new([9u8; 32]),
            owner_id: owner,
            properties: BTreeMap::from([
                (
                    "normalizedParentDomainName".to_string(),
                    Value::Text("dash".to_string()),
                ),
                (
                    "normalizedLabel".to_string(),
                    Value::Text(normalized_label.to_string()),
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

    async fn expect_name_document(
        sdk: &mut Sdk,
        document_type_name: &str,
        normalized_label: &str,
        found: Option<Document>,
    ) {
        let query = dpns_name_query(dpns_v3(), document_type_name, normalized_label.to_string());
        sdk.mock()
            .expect_fetch_many::<Identifier, Document, _, Documents>(
                query,
                Some([(Identifier::new([9u8; 32]), found)].into_iter().collect()),
            )
            .await
            .expect("the expectation is set");
    }

    /// A name's `domain` and its `tombstone` are both found by the normalized parent and
    /// label, at most one of each
    #[test]
    fn should_query_a_name_by_its_normalized_parent_and_label() {
        for document_type_name in [DOMAIN, TOMBSTONE] {
            let query = dpns_name_query(dpns_v3(), document_type_name, "b0b".to_string());
            assert_eq!(query.document_type_name, document_type_name);
            assert_eq!(
                query.where_clauses,
                vec![
                    WhereClause {
                        field: "normalizedParentDomainName".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text("dash".to_string()),
                    },
                    WhereClause {
                        field: "normalizedLabel".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text("b0b".to_string()),
                    },
                ]
            );
            assert_eq!(query.limit, 1);
        }
    }

    #[tokio::test]
    async fn should_report_a_name_tombstoned_only_when_its_tombstone_exists() {
        let owner = Identifier::new([1u8; 32]);
        let mut sdk = mock_sdk();
        expect_name_document(
            &mut sdk,
            TOMBSTONE,
            "b0b",
            Some(name_document(owner, "b0b")),
        )
        .await;
        expect_name_document(&mut sdk, TOMBSTONE, "a11ce", None).await;

        assert!(sdk
            .dpns_name_tombstoned_under(dpns_v3(), "b0b".to_string(), None)
            .await
            .expect("the query answers"));
        assert!(!sdk
            .dpns_name_tombstoned_under(dpns_v3(), "a11ce".to_string(), None)
            .await
            .expect("the query answers"));
    }

    /// The found document is the one the network holds
    #[tokio::test]
    async fn should_find_the_document_of_a_name() {
        let owner = Identifier::new([1u8; 32]);
        let mut sdk = mock_sdk();
        expect_name_document(&mut sdk, DOMAIN, "b0b", Some(name_document(owner, "b0b"))).await;

        let found = sdk
            .dpns_name_document(dpns_v3(), DOMAIN, "b0b".to_string())
            .await
            .expect("the query answers")
            .expect("the name is found");
        assert_eq!(found.owner_id(), owner);
    }
}
