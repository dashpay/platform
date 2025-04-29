use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;

use crate::data_contract::document_type::accessors::DocumentTypeV1Setters;
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::accessors::TokenCostSettersV0;
use crate::data_contract::document_type::token_costs::TokenCosts;
use crate::data_contract::document_type::v0::DocumentTypeV0;
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

#[derive(Debug, PartialEq, Clone)]
pub struct DocumentTypeV1 {
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
}

impl DocumentTypeBasicMethods for DocumentTypeV1 {}

impl DocumentTypeV0Methods for DocumentTypeV1 {}

impl DocumentTypeV1Setters for DocumentTypeV1 {
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

impl From<DocumentTypeV0> for DocumentTypeV1 {
    fn from(value: DocumentTypeV0) -> Self {
        DocumentTypeV1 {
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
        }
    }
}
