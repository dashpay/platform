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
use crate::platform::{DataContract, Document, FetchMany};
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use drive::query::{SelectProjection, WhereClause, WhereOperator};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The DPNS document type of a registered name.
const DOMAIN: &str = "domain";

/// The DPNS document type of a name given up for good, from DPNS v3.
const TOMBSTONE: &str = "tombstone";

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

impl Sdk {
    /// The `document_type_name` document of the name `normalized_label` under `dash`, if the
    /// network holds one. A proven absence comes back as entries holding `None`.
    pub(super) async fn dpns_name_document(
        &self,
        dpns_contract: Arc<DataContract>,
        document_type_name: &str,
        normalized_label: String,
    ) -> Result<Option<Document>, Error> {
        let query = dpns_name_query(dpns_contract, document_type_name, normalized_label);
        Ok(Document::fetch_many(self, query)
            .await?
            .into_values()
            .flatten()
            .next())
    }

    /// Whether the name `normalized_label` under `dash` is tombstoned, read under
    /// `dpns_contract`. Never before DPNS v3, whose contract declares no tombstone.
    pub(super) async fn dpns_name_tombstoned_under(
        &self,
        dpns_contract: Arc<DataContract>,
        normalized_label: String,
    ) -> Result<bool, Error> {
        if !dpns_contract.has_document_type_for_name(TOMBSTONE) {
            return Ok(false);
        }
        Ok(self
            .dpns_name_document(dpns_contract, TOMBSTONE, normalized_label)
            .await?
            .is_some())
    }

    /// Whether a DPNS name is tombstoned: its owner gave it up for good, and nobody can
    /// register it while the tombstone exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The username label (e.g., "alice") or full DPNS name (e.g., "alice.dash")
    pub async fn is_dpns_name_tombstoned(&self, name: &str) -> Result<bool, Error> {
        let normalized_label = normalize_dpns_label(name);
        if normalized_label.is_empty() {
            return Ok(false);
        }
        let dpns_contract = self.fetch_dpns_contract().await?;
        self.dpns_name_tombstoned_under(dpns_contract, normalized_label)
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
    use dpp::version::PlatformVersion;

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
            .dpns_name_tombstoned_under(dpns_v3(), "b0b".to_string())
            .await
            .expect("the query answers"));
        assert!(!sdk
            .dpns_name_tombstoned_under(dpns_v3(), "a11ce".to_string())
            .await
            .expect("the query answers"));
    }

    /// DPNS v2 declares no tombstone, so nothing is read: the mock SDK holds no expectation
    #[tokio::test]
    async fn should_report_no_name_tombstoned_before_dpns_v3() {
        let dpns_v2 = Arc::new(
            load_system_data_contract(
                SystemDataContract::DPNS,
                PlatformVersion::get(14).expect("protocol version 14"),
            )
            .expect("the DPNS contract loads"),
        );
        assert!(!mock_sdk()
            .dpns_name_tombstoned_under(dpns_v2, "b0b".to_string())
            .await
            .expect("nothing is read"));
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
