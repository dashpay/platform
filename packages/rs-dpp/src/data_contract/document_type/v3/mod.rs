use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;

use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::accessors::TokenCostSettersV0;
use crate::data_contract::document_type::token_costs::TokenCosts;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use platform_value::{Identifier, Value};

mod accessors;
#[cfg(feature = "random-document-types")]
pub mod random_document_type;

/// The document type of protocol version 15 and later: [`DocumentTypeV2`] plus
/// the `canBeErased` keyword of meta-schema v4. Parser generation 4 produces it;
/// earlier generations keep producing the version their grammar knows.
#[derive(Debug, PartialEq, Clone)]
pub struct DocumentTypeV3 {
    pub(in crate::data_contract) name: String,
    pub(in crate::data_contract) schema: Value,
    pub(in crate::data_contract) indices: BTreeMap<String, Index>,
    pub(in crate::data_contract) index_structure: IndexLevel,
    /// Flattened properties flatten all objects for quick lookups for indexes
    /// Document field should not contain sub objects.
    pub(in crate::data_contract) flattened_properties: IndexMap<String, DocumentProperty>,
    /// Document field can contain sub objects.
    pub(in crate::data_contract) properties: IndexMap<String, DocumentProperty>,
    pub(in crate::data_contract) identifier_paths: BTreeSet<String>,
    pub(in crate::data_contract) binary_paths: BTreeSet<String>,
    /// The required fields on the document type
    pub(in crate::data_contract) required_fields: BTreeSet<String>,
    /// The transient fields on the document type
    pub(in crate::data_contract) transient_fields: BTreeSet<String>,
    /// Should documents keep history?
    pub(in crate::data_contract) documents_keep_history: bool,
    /// Should transfers of documents of this type be recorded in the document
    /// history system contract?
    pub(in crate::data_contract) documents_keep_transfer_history: bool,
    /// Should purchases of documents of this type be recorded in the document
    /// history system contract?
    pub(in crate::data_contract) documents_keep_purchase_history: bool,
    /// Should price updates on documents of this type be recorded in the
    /// document history system contract?
    pub(in crate::data_contract) documents_keep_pricing_history: bool,
    /// Are documents mutable?
    pub(in crate::data_contract) documents_mutable: bool,
    /// Can documents of this type be deleted?
    pub(in crate::data_contract) documents_can_be_deleted: bool,
    /// Can documents be transferred without a trade?
    pub(in crate::data_contract) documents_transferable: Transferable,
    /// How are these documents traded?
    pub(in crate::data_contract) trade_mode: TradeMode,
    /// Is document creation restricted?
    pub(in crate::data_contract) creation_restriction_mode: CreationRestrictionMode,
    /// The data contract id
    pub(in crate::data_contract) data_contract_id: Identifier,
    /// Encryption key storage requirements
    pub(in crate::data_contract) requires_identity_encryption_bounded_key:
        Option<StorageKeyRequirements>,
    /// Decryption key storage requirements
    pub(in crate::data_contract) requires_identity_decryption_bounded_key:
        Option<StorageKeyRequirements>,
    pub(in crate::data_contract) security_level_requirement: SecurityLevel,
    #[cfg(feature = "validation")]
    pub(in crate::data_contract) json_schema_validator: StatelessJsonSchemaLazyValidator,
    /// The token costs associated with state transitions on this document type
    pub(in crate::data_contract) token_costs: TokenCosts,
    /// When true, the primary key tree uses a CountTree enabling O(1) total document count queries
    pub(in crate::data_contract) documents_countable: bool,
    /// When true, the primary key tree uses a ProvableCountTree enabling range countable.
    /// Implies documents_countable = true.
    pub(in crate::data_contract) range_countable: bool,
    /// When `Some(property_name)`, the primary key tree is a `SumTree` (or
    /// `ProvableSumTree` if [`Self::range_summable`] is also set) summing
    /// the named integer property across every document of this type.
    /// Enables O(log n) `GetDocumentsSum` queries with no `where` filter.
    ///
    /// The named property must be `type: integer` and listed in
    /// [`Self::required_fields`]; the parser enforces this at contract
    /// creation. Composes orthogonally with `documents_countable` —
    /// setting both yields a `CountSumTree` (or `ProvableCountSumTree`)
    /// that carries both a count and a sum, queryable independently.
    pub(in crate::data_contract) documents_summable: Option<String>,
    /// When true, the primary key sum tree is a `ProvableSumTree`
    /// (committing aggregated sub-sums to every internal merk node),
    /// enabling O(log n) `AggregateSumOnRange` queries. Implies
    /// [`Self::documents_summable`] is `Some` — enforced by
    /// [`crate::data_contract::document_type::accessors::DocumentTypeV2Setters::set_range_summable`].
    pub(in crate::data_contract) range_summable: bool,
    /// When true, documents of this type are **indexOnly**: nothing is
    /// written to primary storage (there is no `[0]` primary-key tree at
    /// all) — the index entries are the rows, each terminating in an `Item`
    /// keyed by the index's `terminal` property instead of a `Reference`
    /// keyed by the document id. Only what is in the indexes exists and is
    /// recoverable. The parser (`apply_index_only`) enforces the structural
    /// constraints this layout depends on: every property required and
    /// indexed, `$ownerId` recoverable from at least one index, immutable /
    /// non-transferable / no history, and per-index terminal typing.
    pub(in crate::data_contract) index_only: bool,
    /// When true, a deleted document of this type may have its retained
    /// revisions purged by an erase transition. Requires
    /// [`Self::documents_keep_history`] and [`Self::documents_can_be_deleted`]:
    /// erase applies to deleted documents only, so an erase-only type would
    /// have nothing to act on. The parser enforces the pairing at contract
    /// registration and the update validator keeps the flag immutable, because
    /// narrowing it after a first chunk has irreversibly removed revisions
    /// would strand a partially erased document forever.
    pub(in crate::data_contract) documents_can_be_erased: bool,
}

impl DocumentTypeBasicMethods for DocumentTypeV3 {}

impl DocumentTypeV0Methods for DocumentTypeV3 {}

impl crate::data_contract::document_type::accessors::DocumentTypeV1Setters for DocumentTypeV3 {
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_creation_token_cost(cost)
    }

    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_replacement_token_cost(cost)
    }

    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_deletion_token_cost(cost)
    }

    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_transfer_token_cost(cost)
    }

    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_price_update_token_cost(cost)
    }

    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_purchase_token_cost(cost)
    }
}

impl From<DocumentTypeV0> for DocumentTypeV3 {
    fn from(value: DocumentTypeV0) -> Self {
        DocumentTypeV3 {
            name: value.name,
            schema: value.schema,
            indices: value.indices,
            index_structure: value.index_structure,
            flattened_properties: value.flattened_properties,
            properties: value.properties,
            identifier_paths: value.identifier_paths,
            binary_paths: value.binary_paths,
            required_fields: value.required_fields,
            transient_fields: value.transient_fields,
            documents_keep_history: value.documents_keep_history,
            documents_keep_transfer_history: value.documents_keep_transfer_history,
            documents_keep_purchase_history: value.documents_keep_purchase_history,
            documents_keep_pricing_history: value.documents_keep_pricing_history,
            documents_mutable: value.documents_mutable,
            documents_can_be_deleted: value.documents_can_be_deleted,
            documents_transferable: value.documents_transferable,
            trade_mode: value.trade_mode,
            creation_restriction_mode: value.creation_restriction_mode,
            data_contract_id: value.data_contract_id,
            requires_identity_encryption_bounded_key: value
                .requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: value
                .requires_identity_decryption_bounded_key,
            security_level_requirement: value.security_level_requirement,
            #[cfg(feature = "validation")]
            json_schema_validator: value.json_schema_validator,
            token_costs: TokenCosts::V0(Default::default()),
            documents_countable: false,
            range_countable: false,
            documents_summable: None,
            range_summable: false,
            index_only: false,
            documents_can_be_erased: false,
        }
    }
}

impl From<DocumentTypeV1> for DocumentTypeV3 {
    fn from(value: DocumentTypeV1) -> Self {
        DocumentTypeV3 {
            name: value.name,
            schema: value.schema,
            indices: value.indices,
            index_structure: value.index_structure,
            flattened_properties: value.flattened_properties,
            properties: value.properties,
            identifier_paths: value.identifier_paths,
            binary_paths: value.binary_paths,
            required_fields: value.required_fields,
            transient_fields: value.transient_fields,
            documents_keep_history: value.documents_keep_history,
            documents_keep_transfer_history: value.documents_keep_transfer_history,
            documents_keep_purchase_history: value.documents_keep_purchase_history,
            documents_keep_pricing_history: value.documents_keep_pricing_history,
            documents_mutable: value.documents_mutable,
            documents_can_be_deleted: value.documents_can_be_deleted,
            documents_transferable: value.documents_transferable,
            trade_mode: value.trade_mode,
            creation_restriction_mode: value.creation_restriction_mode,
            data_contract_id: value.data_contract_id,
            requires_identity_encryption_bounded_key: value
                .requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: value
                .requires_identity_decryption_bounded_key,
            security_level_requirement: value.security_level_requirement,
            #[cfg(feature = "validation")]
            json_schema_validator: value.json_schema_validator,
            token_costs: value.token_costs,
            documents_countable: false,
            range_countable: false,
            documents_summable: None,
            range_summable: false,
            index_only: false,
            documents_can_be_erased: false,
        }
    }
}

impl From<DocumentTypeV2> for DocumentTypeV3 {
    fn from(value: DocumentTypeV2) -> Self {
        DocumentTypeV3 {
            name: value.name,
            schema: value.schema,
            indices: value.indices,
            index_structure: value.index_structure,
            flattened_properties: value.flattened_properties,
            properties: value.properties,
            identifier_paths: value.identifier_paths,
            binary_paths: value.binary_paths,
            required_fields: value.required_fields,
            transient_fields: value.transient_fields,
            documents_keep_history: value.documents_keep_history,
            documents_keep_transfer_history: value.documents_keep_transfer_history,
            documents_keep_purchase_history: value.documents_keep_purchase_history,
            documents_keep_pricing_history: value.documents_keep_pricing_history,
            documents_mutable: value.documents_mutable,
            documents_can_be_deleted: value.documents_can_be_deleted,
            documents_transferable: value.documents_transferable,
            trade_mode: value.trade_mode,
            creation_restriction_mode: value.creation_restriction_mode,
            data_contract_id: value.data_contract_id,
            requires_identity_encryption_bounded_key: value
                .requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: value
                .requires_identity_decryption_bounded_key,
            security_level_requirement: value.security_level_requirement,
            #[cfg(feature = "validation")]
            json_schema_validator: value.json_schema_validator,
            token_costs: value.token_costs,
            documents_countable: value.documents_countable,
            range_countable: value.range_countable,
            documents_summable: value.documents_summable,
            range_summable: value.range_summable,
            index_only: value.index_only,
            documents_can_be_erased: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::accessors::{
        DocumentTypeV0Getters, DocumentTypeV2Getters, DocumentTypeV3Getters,
    };
    use crate::data_contract::document_type::v0::DocumentTypeV0;
    use crate::data_contract::document_type::DocumentType;

    fn make_v0() -> DocumentTypeV0 {
        DocumentTypeV0 {
            name: "test".to_string(),
            schema: Value::Null,
            indices: BTreeMap::new(),
            index_structure: IndexLevel::try_from_indices(
                Vec::<Index>::new(),
                "test",
                platform_version::version::PlatformVersion::latest(),
            )
            .unwrap(),
            flattened_properties: IndexMap::new(),
            properties: IndexMap::new(),
            identifier_paths: BTreeSet::new(),
            binary_paths: BTreeSet::new(),
            required_fields: BTreeSet::new(),
            transient_fields: BTreeSet::new(),
            documents_keep_history: true,
            documents_keep_transfer_history: false,
            documents_keep_purchase_history: false,
            documents_keep_pricing_history: false,
            documents_mutable: true,
            documents_can_be_deleted: true,
            documents_transferable: Transferable::Never,
            trade_mode: TradeMode::None,
            creation_restriction_mode: CreationRestrictionMode::NoRestrictions,
            data_contract_id: Identifier::default(),
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
            security_level_requirement: SecurityLevel::HIGH,
            #[cfg(feature = "validation")]
            json_schema_validator: Default::default(),
        }
    }

    #[test]
    fn should_keep_every_earlier_field_and_default_erasability_to_false() {
        let mut v2: DocumentTypeV2 = make_v0().into();
        v2.index_only = false;
        v2.documents_countable = true;
        let v3: DocumentTypeV3 = v2.into();

        assert_eq!(v3.name(), "test");
        assert!(v3.documents_keep_history());
        assert!(v3.documents_can_be_deleted());
        assert!(v3.documents_countable());
        assert!(
            !v3.documents_can_be_erased(),
            "a type parsed by an earlier grammar never declared the keyword"
        );
    }

    #[test]
    fn should_report_erasability_only_from_the_version_that_knows_it() {
        let mut v3: DocumentTypeV3 = make_v0().into();
        v3.documents_can_be_erased = true;

        assert!(DocumentType::V3(v3).documents_can_be_erased());
        assert!(!DocumentType::V0(make_v0()).documents_can_be_erased());
        assert!(!DocumentType::V2(make_v0().into()).documents_can_be_erased());
    }
}
