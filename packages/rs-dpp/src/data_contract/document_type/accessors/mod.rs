mod v0;
mod v1;

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::{DocumentType, DocumentTypeMutRef, DocumentTypeRef};

use platform_value::{Identifier, Value};

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::TokenContractPosition;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};
pub use v0::*;
pub use v1::*;

impl DocumentTypeV0MutGetters for DocumentType {
    fn schema_mut(&mut self) -> &mut Value {
        match self {
            DocumentType::V0(v0) => v0.schema_mut(),
            DocumentType::V1(v1) => v1.schema_mut(),
        }
    }
}

impl DocumentTypeV0Getters for DocumentType {
    fn name(&self) -> &String {
        match self {
            DocumentType::V0(v0) => v0.name(),
            DocumentType::V1(v1) => v1.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentType::V0(v0) => v0.schema(),
            DocumentType::V1(v1) => v1.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentType::V0(v0) => v0.schema_owned(),
            DocumentType::V1(v1) => v1.schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentType::V0(v0) => v0.indexes(),
            DocumentType::V1(v1) => v1.indexes(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentType::V0(v0) => v0.find_contested_index(),
            DocumentType::V1(v1) => v1.find_contested_index(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentType::V0(v0) => v0.index_structure(),
            DocumentType::V1(v1) => v1.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.flattened_properties(),
            DocumentType::V1(v1) => v1.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.properties(),
            DocumentType::V1(v1) => v1.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.identifier_paths(),
            DocumentType::V1(v1) => v1.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.binary_paths(),
            DocumentType::V1(v1) => v1.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.required_fields(),
            DocumentType::V1(v1) => v1.required_fields(),
        }
    }

    fn transient_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.transient_fields(),
            DocumentType::V1(v1) => v1.transient_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_keep_history(),
            DocumentType::V1(v1) => v1.documents_keep_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_mutable(),
            DocumentType::V1(v1) => v1.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_can_be_deleted(),
            DocumentType::V1(v1) => v1.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentType::V0(v0) => v0.documents_transferable(),
            DocumentType::V1(v1) => v1.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentType::V0(v0) => v0.trade_mode(),
            DocumentType::V1(v1) => v1.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentType::V0(v0) => v0.creation_restriction_mode(),
            DocumentType::V1(v1) => v1.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentType::V0(v0) => v0.data_contract_id(),
            DocumentType::V1(v1) => v1.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentType::V0(v0) => v0.requires_identity_encryption_bounded_key(),
            DocumentType::V1(v1) => v1.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentType::V0(v0) => v0.requires_identity_decryption_bounded_key(),
            DocumentType::V1(v1) => v1.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentType::V0(v0) => v0.security_level_requirement(),
            DocumentType::V1(v1) => v1.security_level_requirement(),
        }
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        match self {
            DocumentType::V0(v0) => v0.json_schema_validator_ref(),
            DocumentType::V1(v1) => v1.json_schema_validator_ref(),
        }
    }
}

impl DocumentTypeV0Setters for DocumentType {
    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentType::V0(v0) => v0.set_data_contract_id(data_contract_id),
            DocumentType::V1(v1) => v1.set_data_contract_id(data_contract_id),
        }
    }
}

impl DocumentTypeV1Setters for DocumentType {
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_creation_token_cost(cost),
        }
    }

    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_replacement_token_cost(cost),
        }
    }

    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_deletion_token_cost(cost),
        }
    }

    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_transfer_token_cost(cost),
        }
    }

    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_price_update_token_cost(cost),
        }
    }

    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_purchase_token_cost(cost),
        }
    }
}

impl DocumentTypeV0Getters for DocumentTypeRef<'_> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeRef::V0(v0) => v0.name(),
            DocumentTypeRef::V1(v1) => v1.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentTypeRef::V0(v0) => v0.schema(),
            DocumentTypeRef::V1(v1) => v1.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentTypeRef::V0(v0) => v0.clone().schema_owned(),
            DocumentTypeRef::V1(v1) => v1.clone().schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.indexes(),
            DocumentTypeRef::V1(v1) => v1.indexes(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.find_contested_index(),
            DocumentTypeRef::V1(v1) => v1.find_contested_index(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.index_structure(),
            DocumentTypeRef::V1(v1) => v1.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.flattened_properties(),
            DocumentTypeRef::V1(v1) => v1.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.properties(),
            DocumentTypeRef::V1(v1) => v1.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.identifier_paths(),
            DocumentTypeRef::V1(v1) => v1.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.binary_paths(),
            DocumentTypeRef::V1(v1) => v1.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.required_fields(),
            DocumentTypeRef::V1(v1) => v1.required_fields(),
        }
    }

    fn transient_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.transient_fields(),
            DocumentTypeRef::V1(v1) => v1.transient_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_keep_history(),
            DocumentTypeRef::V1(v1) => v1.documents_keep_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_mutable(),
            DocumentTypeRef::V1(v1) => v1.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_can_be_deleted(),
            DocumentTypeRef::V1(v1) => v1.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_transferable(),
            DocumentTypeRef::V1(v1) => v1.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentTypeRef::V0(v0) => v0.trade_mode(),
            DocumentTypeRef::V1(v1) => v1.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentTypeRef::V0(v0) => v0.creation_restriction_mode(),
            DocumentTypeRef::V1(v1) => v1.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeRef::V0(v0) => v0.data_contract_id(),
            DocumentTypeRef::V1(v1) => v1.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_identity_encryption_bounded_key(),
            DocumentTypeRef::V1(v1) => v1.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_identity_decryption_bounded_key(),
            DocumentTypeRef::V1(v1) => v1.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.security_level_requirement(),
            DocumentTypeRef::V1(v1) => v1.security_level_requirement(),
        }
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        match self {
            DocumentTypeRef::V0(v0) => v0.json_schema_validator_ref(),
            DocumentTypeRef::V1(v1) => v1.json_schema_validator_ref(),
        }
    }
}
impl DocumentTypeV0Getters for DocumentTypeMutRef<'_> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.name(),
            DocumentTypeMutRef::V1(v1) => v1.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.schema(),
            DocumentTypeMutRef::V1(v1) => v1.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.clone().schema_owned(),
            DocumentTypeMutRef::V1(v1) => v1.clone().schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.indexes(),
            DocumentTypeMutRef::V1(v1) => v1.indexes(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.find_contested_index(),
            DocumentTypeMutRef::V1(v1) => v1.find_contested_index(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.index_structure(),
            DocumentTypeMutRef::V1(v1) => v1.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.flattened_properties(),
            DocumentTypeMutRef::V1(v1) => v1.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.properties(),
            DocumentTypeMutRef::V1(v1) => v1.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.identifier_paths(),
            DocumentTypeMutRef::V1(v1) => v1.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.binary_paths(),
            DocumentTypeMutRef::V1(v1) => v1.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.required_fields(),
            DocumentTypeMutRef::V1(v1) => v1.required_fields(),
        }
    }

    fn transient_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.transient_fields(),
            DocumentTypeMutRef::V1(v1) => v1.transient_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_keep_history(),
            DocumentTypeMutRef::V1(v1) => v1.documents_keep_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_mutable(),
            DocumentTypeMutRef::V1(v1) => v1.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_can_be_deleted(),
            DocumentTypeMutRef::V1(v1) => v1.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_transferable(),
            DocumentTypeMutRef::V1(v1) => v1.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.trade_mode(),
            DocumentTypeMutRef::V1(v1) => v1.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.creation_restriction_mode(),
            DocumentTypeMutRef::V1(v1) => v1.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.data_contract_id(),
            DocumentTypeMutRef::V1(v1) => v1.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.requires_identity_encryption_bounded_key(),
            DocumentTypeMutRef::V1(v1) => v1.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.requires_identity_decryption_bounded_key(),
            DocumentTypeMutRef::V1(v1) => v1.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.security_level_requirement(),
            DocumentTypeMutRef::V1(v1) => v1.security_level_requirement(),
        }
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.json_schema_validator_ref(),
            DocumentTypeMutRef::V1(v1) => v1.json_schema_validator_ref(),
        }
    }
}

impl DocumentTypeV0Setters for DocumentTypeMutRef<'_> {
    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.set_data_contract_id(data_contract_id),
            DocumentTypeMutRef::V1(v1) => v1.set_data_contract_id(data_contract_id),
        }
    }
}

impl DocumentTypeV1Getters for DocumentType {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_transfer_token_cost(),
        }
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_update_price_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_purchase_token_cost(),
        }
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => vec![],
            DocumentType::V1(v1) => v1.all_document_token_costs(),
        }
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        match self {
            DocumentType::V0(_) => BTreeMap::new(),
            DocumentType::V1(v1) => v1.all_external_token_costs_contract_tokens(),
        }
    }
}

impl DocumentTypeV1Getters for DocumentTypeRef<'_> {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_transfer_token_cost(),
        }
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_update_price_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_purchase_token_cost(),
        }
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => vec![],
            DocumentTypeRef::V1(v1) => v1.all_document_token_costs(),
        }
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        match self {
            DocumentTypeRef::V0(_) => BTreeMap::new(),
            DocumentTypeRef::V1(v1) => v1.all_external_token_costs_contract_tokens(),
        }
    }
}

impl DocumentTypeV1Getters for DocumentTypeMutRef<'_> {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_transfer_token_cost(),
        }
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_update_price_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_purchase_token_cost(),
        }
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => vec![],
            DocumentTypeMutRef::V1(v1) => v1.all_document_token_costs(),
        }
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        match self {
            DocumentTypeMutRef::V0(_) => BTreeMap::new(),
            DocumentTypeMutRef::V1(v1) => v1.all_external_token_costs_contract_tokens(),
        }
    }
}
