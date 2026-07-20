mod v0;
mod v1;
mod v2;

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
pub use v2::*;

impl DocumentTypeV0MutGetters for DocumentType {
    fn schema_mut(&mut self) -> &mut Value {
        match self {
            DocumentType::V0(v0) => v0.schema_mut(),
            DocumentType::V1(v1) => v1.schema_mut(),
            DocumentType::V2(v2) => v2.schema_mut(),
        }
    }
}

impl DocumentTypeV0Getters for DocumentType {
    fn name(&self) -> &String {
        match self {
            DocumentType::V0(v0) => v0.name(),
            DocumentType::V1(v1) => v1.name(),
            DocumentType::V2(v2) => v2.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentType::V0(v0) => v0.schema(),
            DocumentType::V1(v1) => v1.schema(),
            DocumentType::V2(v2) => v2.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentType::V0(v0) => v0.schema_owned(),
            DocumentType::V1(v1) => v1.schema_owned(),
            DocumentType::V2(v2) => v2.schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentType::V0(v0) => v0.indexes(),
            DocumentType::V1(v1) => v1.indexes(),
            DocumentType::V2(v2) => v2.indexes(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentType::V0(v0) => v0.find_contested_index(),
            DocumentType::V1(v1) => v1.find_contested_index(),
            DocumentType::V2(v2) => v2.find_contested_index(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentType::V0(v0) => v0.index_structure(),
            DocumentType::V1(v1) => v1.index_structure(),
            DocumentType::V2(v2) => v2.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.flattened_properties(),
            DocumentType::V1(v1) => v1.flattened_properties(),
            DocumentType::V2(v2) => v2.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.properties(),
            DocumentType::V1(v1) => v1.properties(),
            DocumentType::V2(v2) => v2.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.identifier_paths(),
            DocumentType::V1(v1) => v1.identifier_paths(),
            DocumentType::V2(v2) => v2.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.binary_paths(),
            DocumentType::V1(v1) => v1.binary_paths(),
            DocumentType::V2(v2) => v2.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.required_fields(),
            DocumentType::V1(v1) => v1.required_fields(),
            DocumentType::V2(v2) => v2.required_fields(),
        }
    }

    fn transient_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.transient_fields(),
            DocumentType::V1(v1) => v1.transient_fields(),
            DocumentType::V2(v2) => v2.transient_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_keep_history(),
            DocumentType::V1(v1) => v1.documents_keep_history(),
            DocumentType::V2(v2) => v2.documents_keep_history(),
        }
    }

    fn documents_keep_transfer_history(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_keep_transfer_history(),
            DocumentType::V1(v1) => v1.documents_keep_transfer_history(),
            DocumentType::V2(v2) => v2.documents_keep_transfer_history(),
        }
    }

    fn documents_keep_purchase_history(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_keep_purchase_history(),
            DocumentType::V1(v1) => v1.documents_keep_purchase_history(),
            DocumentType::V2(v2) => v2.documents_keep_purchase_history(),
        }
    }

    fn documents_keep_pricing_history(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_keep_pricing_history(),
            DocumentType::V1(v1) => v1.documents_keep_pricing_history(),
            DocumentType::V2(v2) => v2.documents_keep_pricing_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_mutable(),
            DocumentType::V1(v1) => v1.documents_mutable(),
            DocumentType::V2(v2) => v2.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_can_be_deleted(),
            DocumentType::V1(v1) => v1.documents_can_be_deleted(),
            DocumentType::V2(v2) => v2.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentType::V0(v0) => v0.documents_transferable(),
            DocumentType::V1(v1) => v1.documents_transferable(),
            DocumentType::V2(v2) => v2.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentType::V0(v0) => v0.trade_mode(),
            DocumentType::V1(v1) => v1.trade_mode(),
            DocumentType::V2(v2) => v2.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentType::V0(v0) => v0.creation_restriction_mode(),
            DocumentType::V1(v1) => v1.creation_restriction_mode(),
            DocumentType::V2(v2) => v2.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentType::V0(v0) => v0.data_contract_id(),
            DocumentType::V1(v1) => v1.data_contract_id(),
            DocumentType::V2(v2) => v2.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentType::V0(v0) => v0.requires_identity_encryption_bounded_key(),
            DocumentType::V1(v1) => v1.requires_identity_encryption_bounded_key(),
            DocumentType::V2(v2) => v2.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentType::V0(v0) => v0.requires_identity_decryption_bounded_key(),
            DocumentType::V1(v1) => v1.requires_identity_decryption_bounded_key(),
            DocumentType::V2(v2) => v2.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentType::V0(v0) => v0.security_level_requirement(),
            DocumentType::V1(v1) => v1.security_level_requirement(),
            DocumentType::V2(v2) => v2.security_level_requirement(),
        }
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        match self {
            DocumentType::V0(v0) => v0.json_schema_validator_ref(),
            DocumentType::V1(v1) => v1.json_schema_validator_ref(),
            DocumentType::V2(v2) => v2.json_schema_validator_ref(),
        }
    }
}

impl DocumentTypeV0Setters for DocumentType {
    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentType::V0(v0) => v0.set_data_contract_id(data_contract_id),
            DocumentType::V1(v1) => v1.set_data_contract_id(data_contract_id),
            DocumentType::V2(v2) => v2.set_data_contract_id(data_contract_id),
        }
    }
}

impl DocumentTypeV1Setters for DocumentType {
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_creation_token_cost(cost),
            DocumentType::V2(v2) => v2.set_document_creation_token_cost(cost),
        }
    }

    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_replacement_token_cost(cost),
            DocumentType::V2(v2) => v2.set_document_replacement_token_cost(cost),
        }
    }

    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_deletion_token_cost(cost),
            DocumentType::V2(v2) => v2.set_document_deletion_token_cost(cost),
        }
    }

    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_transfer_token_cost(cost),
            DocumentType::V2(v2) => v2.set_document_transfer_token_cost(cost),
        }
    }

    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_price_update_token_cost(cost),
            DocumentType::V2(v2) => v2.set_document_price_update_token_cost(cost),
        }
    }

    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(v1) => v1.set_document_purchase_token_cost(cost),
            DocumentType::V2(v2) => v2.set_document_purchase_token_cost(cost),
        }
    }
}

impl DocumentTypeV0Getters for DocumentTypeRef<'_> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeRef::V0(v0) => v0.name(),
            DocumentTypeRef::V1(v1) => v1.name(),
            DocumentTypeRef::V2(v2) => v2.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentTypeRef::V0(v0) => v0.schema(),
            DocumentTypeRef::V1(v1) => v1.schema(),
            DocumentTypeRef::V2(v2) => v2.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentTypeRef::V0(v0) => v0.clone().schema_owned(),
            DocumentTypeRef::V1(v1) => v1.clone().schema_owned(),
            DocumentTypeRef::V2(v2) => v2.clone().schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.indexes(),
            DocumentTypeRef::V1(v1) => v1.indexes(),
            DocumentTypeRef::V2(v2) => v2.indexes(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.find_contested_index(),
            DocumentTypeRef::V1(v1) => v1.find_contested_index(),
            DocumentTypeRef::V2(v2) => v2.find_contested_index(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.index_structure(),
            DocumentTypeRef::V1(v1) => v1.index_structure(),
            DocumentTypeRef::V2(v2) => v2.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.flattened_properties(),
            DocumentTypeRef::V1(v1) => v1.flattened_properties(),
            DocumentTypeRef::V2(v2) => v2.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.properties(),
            DocumentTypeRef::V1(v1) => v1.properties(),
            DocumentTypeRef::V2(v2) => v2.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.identifier_paths(),
            DocumentTypeRef::V1(v1) => v1.identifier_paths(),
            DocumentTypeRef::V2(v2) => v2.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.binary_paths(),
            DocumentTypeRef::V1(v1) => v1.binary_paths(),
            DocumentTypeRef::V2(v2) => v2.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.required_fields(),
            DocumentTypeRef::V1(v1) => v1.required_fields(),
            DocumentTypeRef::V2(v2) => v2.required_fields(),
        }
    }

    fn transient_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.transient_fields(),
            DocumentTypeRef::V1(v1) => v1.transient_fields(),
            DocumentTypeRef::V2(v2) => v2.transient_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_keep_history(),
            DocumentTypeRef::V1(v1) => v1.documents_keep_history(),
            DocumentTypeRef::V2(v2) => v2.documents_keep_history(),
        }
    }

    fn documents_keep_transfer_history(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_keep_transfer_history(),
            DocumentTypeRef::V1(v1) => v1.documents_keep_transfer_history(),
            DocumentTypeRef::V2(v2) => v2.documents_keep_transfer_history(),
        }
    }

    fn documents_keep_purchase_history(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_keep_purchase_history(),
            DocumentTypeRef::V1(v1) => v1.documents_keep_purchase_history(),
            DocumentTypeRef::V2(v2) => v2.documents_keep_purchase_history(),
        }
    }

    fn documents_keep_pricing_history(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_keep_pricing_history(),
            DocumentTypeRef::V1(v1) => v1.documents_keep_pricing_history(),
            DocumentTypeRef::V2(v2) => v2.documents_keep_pricing_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_mutable(),
            DocumentTypeRef::V1(v1) => v1.documents_mutable(),
            DocumentTypeRef::V2(v2) => v2.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_can_be_deleted(),
            DocumentTypeRef::V1(v1) => v1.documents_can_be_deleted(),
            DocumentTypeRef::V2(v2) => v2.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_transferable(),
            DocumentTypeRef::V1(v1) => v1.documents_transferable(),
            DocumentTypeRef::V2(v2) => v2.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentTypeRef::V0(v0) => v0.trade_mode(),
            DocumentTypeRef::V1(v1) => v1.trade_mode(),
            DocumentTypeRef::V2(v2) => v2.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentTypeRef::V0(v0) => v0.creation_restriction_mode(),
            DocumentTypeRef::V1(v1) => v1.creation_restriction_mode(),
            DocumentTypeRef::V2(v2) => v2.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeRef::V0(v0) => v0.data_contract_id(),
            DocumentTypeRef::V1(v1) => v1.data_contract_id(),
            DocumentTypeRef::V2(v2) => v2.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_identity_encryption_bounded_key(),
            DocumentTypeRef::V1(v1) => v1.requires_identity_encryption_bounded_key(),
            DocumentTypeRef::V2(v2) => v2.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_identity_decryption_bounded_key(),
            DocumentTypeRef::V1(v1) => v1.requires_identity_decryption_bounded_key(),
            DocumentTypeRef::V2(v2) => v2.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.security_level_requirement(),
            DocumentTypeRef::V1(v1) => v1.security_level_requirement(),
            DocumentTypeRef::V2(v2) => v2.security_level_requirement(),
        }
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        match self {
            DocumentTypeRef::V0(v0) => v0.json_schema_validator_ref(),
            DocumentTypeRef::V1(v1) => v1.json_schema_validator_ref(),
            DocumentTypeRef::V2(v2) => v2.json_schema_validator_ref(),
        }
    }
}
impl DocumentTypeV0Getters for DocumentTypeMutRef<'_> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.name(),
            DocumentTypeMutRef::V1(v1) => v1.name(),
            DocumentTypeMutRef::V2(v2) => v2.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.schema(),
            DocumentTypeMutRef::V1(v1) => v1.schema(),
            DocumentTypeMutRef::V2(v2) => v2.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.clone().schema_owned(),
            DocumentTypeMutRef::V1(v1) => v1.clone().schema_owned(),
            DocumentTypeMutRef::V2(v2) => v2.clone().schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.indexes(),
            DocumentTypeMutRef::V1(v1) => v1.indexes(),
            DocumentTypeMutRef::V2(v2) => v2.indexes(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.find_contested_index(),
            DocumentTypeMutRef::V1(v1) => v1.find_contested_index(),
            DocumentTypeMutRef::V2(v2) => v2.find_contested_index(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.index_structure(),
            DocumentTypeMutRef::V1(v1) => v1.index_structure(),
            DocumentTypeMutRef::V2(v2) => v2.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.flattened_properties(),
            DocumentTypeMutRef::V1(v1) => v1.flattened_properties(),
            DocumentTypeMutRef::V2(v2) => v2.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.properties(),
            DocumentTypeMutRef::V1(v1) => v1.properties(),
            DocumentTypeMutRef::V2(v2) => v2.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.identifier_paths(),
            DocumentTypeMutRef::V1(v1) => v1.identifier_paths(),
            DocumentTypeMutRef::V2(v2) => v2.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.binary_paths(),
            DocumentTypeMutRef::V1(v1) => v1.binary_paths(),
            DocumentTypeMutRef::V2(v2) => v2.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.required_fields(),
            DocumentTypeMutRef::V1(v1) => v1.required_fields(),
            DocumentTypeMutRef::V2(v2) => v2.required_fields(),
        }
    }

    fn transient_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.transient_fields(),
            DocumentTypeMutRef::V1(v1) => v1.transient_fields(),
            DocumentTypeMutRef::V2(v2) => v2.transient_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_keep_history(),
            DocumentTypeMutRef::V1(v1) => v1.documents_keep_history(),
            DocumentTypeMutRef::V2(v2) => v2.documents_keep_history(),
        }
    }

    fn documents_keep_transfer_history(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_keep_transfer_history(),
            DocumentTypeMutRef::V1(v1) => v1.documents_keep_transfer_history(),
            DocumentTypeMutRef::V2(v2) => v2.documents_keep_transfer_history(),
        }
    }

    fn documents_keep_purchase_history(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_keep_purchase_history(),
            DocumentTypeMutRef::V1(v1) => v1.documents_keep_purchase_history(),
            DocumentTypeMutRef::V2(v2) => v2.documents_keep_purchase_history(),
        }
    }

    fn documents_keep_pricing_history(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_keep_pricing_history(),
            DocumentTypeMutRef::V1(v1) => v1.documents_keep_pricing_history(),
            DocumentTypeMutRef::V2(v2) => v2.documents_keep_pricing_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_mutable(),
            DocumentTypeMutRef::V1(v1) => v1.documents_mutable(),
            DocumentTypeMutRef::V2(v2) => v2.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_can_be_deleted(),
            DocumentTypeMutRef::V1(v1) => v1.documents_can_be_deleted(),
            DocumentTypeMutRef::V2(v2) => v2.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_transferable(),
            DocumentTypeMutRef::V1(v1) => v1.documents_transferable(),
            DocumentTypeMutRef::V2(v2) => v2.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.trade_mode(),
            DocumentTypeMutRef::V1(v1) => v1.trade_mode(),
            DocumentTypeMutRef::V2(v2) => v2.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.creation_restriction_mode(),
            DocumentTypeMutRef::V1(v1) => v1.creation_restriction_mode(),
            DocumentTypeMutRef::V2(v2) => v2.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.data_contract_id(),
            DocumentTypeMutRef::V1(v1) => v1.data_contract_id(),
            DocumentTypeMutRef::V2(v2) => v2.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.requires_identity_encryption_bounded_key(),
            DocumentTypeMutRef::V1(v1) => v1.requires_identity_encryption_bounded_key(),
            DocumentTypeMutRef::V2(v2) => v2.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.requires_identity_decryption_bounded_key(),
            DocumentTypeMutRef::V1(v1) => v1.requires_identity_decryption_bounded_key(),
            DocumentTypeMutRef::V2(v2) => v2.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.security_level_requirement(),
            DocumentTypeMutRef::V1(v1) => v1.security_level_requirement(),
            DocumentTypeMutRef::V2(v2) => v2.security_level_requirement(),
        }
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.json_schema_validator_ref(),
            DocumentTypeMutRef::V1(v1) => v1.json_schema_validator_ref(),
            DocumentTypeMutRef::V2(v2) => v2.json_schema_validator_ref(),
        }
    }
}

impl DocumentTypeV0Setters for DocumentTypeMutRef<'_> {
    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.set_data_contract_id(data_contract_id),
            DocumentTypeMutRef::V1(v1) => v1.set_data_contract_id(data_contract_id),
            DocumentTypeMutRef::V2(v2) => v2.set_data_contract_id(data_contract_id),
        }
    }
}

impl DocumentTypeV1Getters for DocumentType {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_creation_token_cost(),
            DocumentType::V2(v2) => v2.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_replacement_token_cost(),
            DocumentType::V2(v2) => v2.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_deletion_token_cost(),
            DocumentType::V2(v2) => v2.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_transfer_token_cost(),
            DocumentType::V2(v2) => v2.document_transfer_token_cost(),
        }
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_update_price_token_cost(),
            DocumentType::V2(v2) => v2.document_update_price_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(v1) => v1.document_purchase_token_cost(),
            DocumentType::V2(v2) => v2.document_purchase_token_cost(),
        }
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        match self {
            DocumentType::V0(_) => vec![],
            DocumentType::V1(v1) => v1.all_document_token_costs(),
            DocumentType::V2(v2) => v2.all_document_token_costs(),
        }
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        match self {
            DocumentType::V0(_) => BTreeMap::new(),
            DocumentType::V1(v1) => v1.all_external_token_costs_contract_tokens(),
            DocumentType::V2(v2) => v2.all_external_token_costs_contract_tokens(),
        }
    }
}

impl DocumentTypeV1Getters for DocumentTypeRef<'_> {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_creation_token_cost(),
            DocumentTypeRef::V2(v2) => v2.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_replacement_token_cost(),
            DocumentTypeRef::V2(v2) => v2.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_deletion_token_cost(),
            DocumentTypeRef::V2(v2) => v2.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_transfer_token_cost(),
            DocumentTypeRef::V2(v2) => v2.document_transfer_token_cost(),
        }
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_update_price_token_cost(),
            DocumentTypeRef::V2(v2) => v2.document_update_price_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(v1) => v1.document_purchase_token_cost(),
            DocumentTypeRef::V2(v2) => v2.document_purchase_token_cost(),
        }
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        match self {
            DocumentTypeRef::V0(_) => vec![],
            DocumentTypeRef::V1(v1) => v1.all_document_token_costs(),
            DocumentTypeRef::V2(v2) => v2.all_document_token_costs(),
        }
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        match self {
            DocumentTypeRef::V0(_) => BTreeMap::new(),
            DocumentTypeRef::V1(v1) => v1.all_external_token_costs_contract_tokens(),
            DocumentTypeRef::V2(v2) => v2.all_external_token_costs_contract_tokens(),
        }
    }
}

impl DocumentTypeV1Getters for DocumentTypeMutRef<'_> {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_creation_token_cost(),
            DocumentTypeMutRef::V2(v2) => v2.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_replacement_token_cost(),
            DocumentTypeMutRef::V2(v2) => v2.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_deletion_token_cost(),
            DocumentTypeMutRef::V2(v2) => v2.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_transfer_token_cost(),
            DocumentTypeMutRef::V2(v2) => v2.document_transfer_token_cost(),
        }
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_update_price_token_cost(),
            DocumentTypeMutRef::V2(v2) => v2.document_update_price_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(v1) => v1.document_purchase_token_cost(),
            DocumentTypeMutRef::V2(v2) => v2.document_purchase_token_cost(),
        }
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        match self {
            DocumentTypeMutRef::V0(_) => vec![],
            DocumentTypeMutRef::V1(v1) => v1.all_document_token_costs(),
            DocumentTypeMutRef::V2(v2) => v2.all_document_token_costs(),
        }
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        match self {
            DocumentTypeMutRef::V0(_) => BTreeMap::new(),
            DocumentTypeMutRef::V1(v1) => v1.all_external_token_costs_contract_tokens(),
            DocumentTypeMutRef::V2(v2) => v2.all_external_token_costs_contract_tokens(),
        }
    }
}

impl DocumentTypeV2Getters for DocumentType {
    fn documents_countable(&self) -> bool {
        match self {
            DocumentType::V0(_) => false,
            DocumentType::V1(_) => false,
            DocumentType::V2(v2) => v2.documents_countable(),
        }
    }

    fn range_countable(&self) -> bool {
        match self {
            DocumentType::V0(_) => false,
            DocumentType::V1(_) => false,
            DocumentType::V2(v2) => v2.range_countable(),
        }
    }

    fn documents_summable(&self) -> Option<&str> {
        match self {
            DocumentType::V0(_) => None,
            DocumentType::V1(_) => None,
            DocumentType::V2(v2) => v2.documents_summable(),
        }
    }

    fn range_summable(&self) -> bool {
        match self {
            DocumentType::V0(_) => false,
            DocumentType::V1(_) => false,
            DocumentType::V2(v2) => v2.range_summable(),
        }
    }
}

impl DocumentTypeV2Setters for DocumentType {
    fn set_documents_countable(&mut self, countable: bool) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(_) => { /* no-op */ }
            DocumentType::V2(v2) => v2.set_documents_countable(countable),
        }
    }

    fn set_range_countable(&mut self, range_countable: bool) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(_) => { /* no-op */ }
            DocumentType::V2(v2) => v2.set_range_countable(range_countable),
        }
    }

    fn set_documents_summable(&mut self, property: Option<String>) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(_) => { /* no-op */ }
            DocumentType::V2(v2) => v2.set_documents_summable(property),
        }
    }

    fn set_range_summable(&mut self, range_summable: bool) {
        match self {
            DocumentType::V0(_) => { /* no-op */ }
            DocumentType::V1(_) => { /* no-op */ }
            DocumentType::V2(v2) => v2.set_range_summable(range_summable),
        }
    }
}

impl DocumentTypeV2Getters for DocumentTypeRef<'_> {
    fn documents_countable(&self) -> bool {
        match self {
            DocumentTypeRef::V0(_) => false,
            DocumentTypeRef::V1(_) => false,
            DocumentTypeRef::V2(v2) => v2.documents_countable(),
        }
    }

    fn range_countable(&self) -> bool {
        match self {
            DocumentTypeRef::V0(_) => false,
            DocumentTypeRef::V1(_) => false,
            DocumentTypeRef::V2(v2) => v2.range_countable(),
        }
    }

    fn documents_summable(&self) -> Option<&str> {
        match self {
            DocumentTypeRef::V0(_) => None,
            DocumentTypeRef::V1(_) => None,
            DocumentTypeRef::V2(v2) => v2.documents_summable(),
        }
    }

    fn range_summable(&self) -> bool {
        match self {
            DocumentTypeRef::V0(_) => false,
            DocumentTypeRef::V1(_) => false,
            DocumentTypeRef::V2(v2) => v2.range_summable(),
        }
    }
}

impl DocumentTypeV2Getters for DocumentTypeMutRef<'_> {
    fn documents_countable(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(_) => false,
            DocumentTypeMutRef::V1(_) => false,
            DocumentTypeMutRef::V2(v2) => v2.documents_countable(),
        }
    }

    fn range_countable(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(_) => false,
            DocumentTypeMutRef::V1(_) => false,
            DocumentTypeMutRef::V2(v2) => v2.range_countable(),
        }
    }

    fn documents_summable(&self) -> Option<&str> {
        match self {
            DocumentTypeMutRef::V0(_) => None,
            DocumentTypeMutRef::V1(_) => None,
            DocumentTypeMutRef::V2(v2) => v2.documents_summable(),
        }
    }

    fn range_summable(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(_) => false,
            DocumentTypeMutRef::V1(_) => false,
            DocumentTypeMutRef::V2(v2) => v2.range_summable(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
    use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
    use crate::document::transfer::Transferable;
    use crate::nft::TradeMode;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;

    fn get_test_document_type() -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let doc_types = data_contract.document_types();
        // Use "indexedDocument" because it has the most properties and indices
        doc_types
            .get("indexedDocument")
            .expect("indexedDocument should exist")
            .clone()
    }

    // DocumentTypeV0Getters on DocumentType

    #[test]
    fn name_returns_document_type_name() {
        let doc_type = get_test_document_type();
        assert_eq!(doc_type.name(), "indexedDocument");
    }

    #[test]
    fn schema_returns_value() {
        let doc_type = get_test_document_type();
        let schema = doc_type.schema();
        // Schema should be a non-null Value
        assert!(!schema.is_null());
    }

    #[test]
    fn schema_owned_returns_value() {
        let doc_type = get_test_document_type();
        let schema = doc_type.schema_owned();
        assert!(!schema.is_null());
    }

    #[test]
    fn indexes_returns_indices() {
        let doc_type = get_test_document_type();
        let indexes = doc_type.indexes();
        // indexedDocument has 6 indices defined in the fixture
        assert_eq!(indexes.len(), 6);
        assert!(indexes.contains_key("index1"));
        assert!(indexes.contains_key("index2"));
    }

    #[test]
    fn find_contested_index_returns_none_for_normal_type() {
        let doc_type = get_test_document_type();
        // The fixture's indexedDocument has no contested index
        assert!(doc_type.find_contested_index().is_none());
    }

    #[test]
    fn index_structure_returns_index_level() {
        let doc_type = get_test_document_type();
        let _structure = doc_type.index_structure();
        // Just ensure it does not panic
    }

    #[test]
    fn flattened_properties_contains_expected_fields() {
        let doc_type = get_test_document_type();
        let props = doc_type.flattened_properties();
        assert!(props.contains_key("firstName"));
        assert!(props.contains_key("lastName"));
    }

    #[test]
    fn properties_contains_expected_fields() {
        let doc_type = get_test_document_type();
        let props = doc_type.properties();
        assert!(props.contains_key("firstName"));
        assert!(props.contains_key("lastName"));
    }

    #[test]
    fn required_fields_contains_expected_fields() {
        let doc_type = get_test_document_type();
        let required = doc_type.required_fields();
        assert!(required.contains("firstName"));
        assert!(required.contains("lastName"));
    }

    #[test]
    fn transient_fields_returns_set() {
        let doc_type = get_test_document_type();
        let _transient = doc_type.transient_fields();
        // The fixture does not define transient fields, so it should be empty
        assert!(doc_type.transient_fields().is_empty());
    }

    #[test]
    fn identifier_paths_returns_set() {
        let doc_type = get_test_document_type();
        let _paths = doc_type.identifier_paths();
        // Should not panic
    }

    #[test]
    fn binary_paths_returns_set() {
        let doc_type = get_test_document_type();
        let _paths = doc_type.binary_paths();
        // Should not panic
    }

    #[test]
    fn documents_keep_history_returns_bool() {
        let doc_type = get_test_document_type();
        // Default is false for the fixture
        let _keep = doc_type.documents_keep_history();
    }

    #[test]
    fn documents_mutable_returns_bool() {
        let doc_type = get_test_document_type();
        // Default is true for the fixture
        assert!(doc_type.documents_mutable());
    }

    #[test]
    fn documents_can_be_deleted_returns_bool() {
        let doc_type = get_test_document_type();
        assert!(doc_type.documents_can_be_deleted());
    }

    #[test]
    fn documents_transferable_returns_value() {
        let doc_type = get_test_document_type();
        assert_eq!(doc_type.documents_transferable(), Transferable::Never);
    }

    #[test]
    fn trade_mode_returns_value() {
        let doc_type = get_test_document_type();
        assert_eq!(doc_type.trade_mode(), TradeMode::None);
    }

    #[test]
    fn creation_restriction_mode_returns_value() {
        let doc_type = get_test_document_type();
        assert_eq!(
            doc_type.creation_restriction_mode(),
            CreationRestrictionMode::NoRestrictions
        );
    }

    #[test]
    fn data_contract_id_returns_identifier() {
        let doc_type = get_test_document_type();
        let id = doc_type.data_contract_id();
        // Should be a valid non-zero identifier set by the fixture
        assert_ne!(id, Identifier::default());
    }

    #[test]
    fn requires_identity_encryption_bounded_key_returns_option() {
        let doc_type = get_test_document_type();
        // The fixture's indexedDocument does not require encryption key
        assert_eq!(
            doc_type.requires_identity_encryption_bounded_key(),
            None::<StorageKeyRequirements>
        );
    }

    #[test]
    fn requires_identity_decryption_bounded_key_returns_option() {
        let doc_type = get_test_document_type();
        assert_eq!(
            doc_type.requires_identity_decryption_bounded_key(),
            None::<StorageKeyRequirements>
        );
    }

    #[test]
    fn security_level_requirement_returns_level() {
        let doc_type = get_test_document_type();
        let _level = doc_type.security_level_requirement();
        // Should not panic; default is HIGH
    }

    // DocumentTypeV0Setters on DocumentType

    #[test]
    fn set_data_contract_id_updates_id() {
        let mut doc_type = get_test_document_type();
        let new_id = Identifier::from([42u8; 32]);
        doc_type.set_data_contract_id(new_id);
        assert_eq!(doc_type.data_contract_id(), new_id);
    }

    // DocumentTypeV0Getters on DocumentTypeRef

    #[test]
    fn document_type_ref_getters_work() {
        let doc_type = get_test_document_type();
        let doc_ref = doc_type.as_ref();

        assert_eq!(doc_ref.name(), "indexedDocument");
        assert!(!doc_ref.schema().is_null());
        assert!(!doc_ref.indexes().is_empty());
        assert!(doc_ref.find_contested_index().is_none());
        let _structure = doc_ref.index_structure();
        assert!(!doc_ref.flattened_properties().is_empty());
        assert!(!doc_ref.properties().is_empty());
        let _id_paths = doc_ref.identifier_paths();
        let _bin_paths = doc_ref.binary_paths();
        assert!(!doc_ref.required_fields().is_empty());
        assert!(doc_ref.transient_fields().is_empty());
        assert!(doc_ref.documents_mutable());
        assert!(doc_ref.documents_can_be_deleted());
        assert_eq!(doc_ref.documents_transferable(), Transferable::Never);
        assert_eq!(doc_ref.trade_mode(), TradeMode::None);
        assert_eq!(
            doc_ref.creation_restriction_mode(),
            CreationRestrictionMode::NoRestrictions
        );
        assert_ne!(doc_ref.data_contract_id(), Identifier::default());
        assert_eq!(
            doc_ref.requires_identity_encryption_bounded_key(),
            None::<StorageKeyRequirements>
        );
        assert_eq!(
            doc_ref.requires_identity_decryption_bounded_key(),
            None::<StorageKeyRequirements>
        );
        let _sec_level = doc_ref.security_level_requirement();
    }

    #[test]
    fn document_type_ref_schema_owned() {
        let doc_type = get_test_document_type();
        let doc_ref = doc_type.as_ref();
        let schema = doc_ref.schema_owned();
        assert!(!schema.is_null());
    }

    // DocumentTypeV0MutGetters on DocumentType

    #[test]
    fn schema_mut_allows_modification() {
        let mut doc_type = get_test_document_type();
        let schema = doc_type.schema_mut();
        // Just verify it returns a mutable reference without panicking
        assert!(!schema.is_null());
    }

    // DocumentTypeV0Getters on DocumentTypeMutRef

    #[test]
    fn document_type_mut_ref_getters_work() {
        let mut doc_type = get_test_document_type();
        let doc_mut_ref = doc_type.as_mut_ref();

        assert_eq!(doc_mut_ref.name(), "indexedDocument");
        assert!(!doc_mut_ref.schema().is_null());
        assert!(!doc_mut_ref.indexes().is_empty());
        assert!(doc_mut_ref.find_contested_index().is_none());
        let _structure = doc_mut_ref.index_structure();
        assert!(!doc_mut_ref.flattened_properties().is_empty());
        assert!(!doc_mut_ref.properties().is_empty());
        let _id_paths = doc_mut_ref.identifier_paths();
        let _bin_paths = doc_mut_ref.binary_paths();
        assert!(!doc_mut_ref.required_fields().is_empty());
        assert!(doc_mut_ref.transient_fields().is_empty());
        assert!(doc_mut_ref.documents_mutable());
        assert!(doc_mut_ref.documents_can_be_deleted());
        assert_eq!(doc_mut_ref.documents_transferable(), Transferable::Never);
        assert_eq!(doc_mut_ref.trade_mode(), TradeMode::None);
        assert_eq!(
            doc_mut_ref.creation_restriction_mode(),
            CreationRestrictionMode::NoRestrictions
        );
        assert_ne!(doc_mut_ref.data_contract_id(), Identifier::default());
        assert_eq!(
            doc_mut_ref.requires_identity_encryption_bounded_key(),
            None::<StorageKeyRequirements>
        );
        assert_eq!(
            doc_mut_ref.requires_identity_decryption_bounded_key(),
            None::<StorageKeyRequirements>
        );
        let _sec_level = doc_mut_ref.security_level_requirement();
    }

    #[test]
    fn document_type_mut_ref_schema_owned() {
        let mut doc_type = get_test_document_type();
        let doc_mut_ref = doc_type.as_mut_ref();
        let schema = doc_mut_ref.schema_owned();
        assert!(!schema.is_null());
    }

    // DocumentTypeV0Setters on DocumentTypeMutRef

    #[test]
    fn document_type_mut_ref_set_data_contract_id() {
        let mut doc_type = get_test_document_type();
        let mut doc_mut_ref = doc_type.as_mut_ref();
        let new_id = Identifier::from([99u8; 32]);
        doc_mut_ref.set_data_contract_id(new_id);
        assert_eq!(doc_mut_ref.data_contract_id(), new_id);
    }

    // DocumentTypeV1Getters on DocumentType (V0 variant returns None/defaults)

    #[test]
    fn v1_getters_on_v0_document_type_return_none() {
        let doc_type = get_test_document_type();
        // The fixture creates V0 document types
        assert!(doc_type.document_creation_token_cost().is_none());
        assert!(doc_type.document_replacement_token_cost().is_none());
        assert!(doc_type.document_deletion_token_cost().is_none());
        assert!(doc_type.document_transfer_token_cost().is_none());
        assert!(doc_type.document_update_price_token_cost().is_none());
        assert!(doc_type.document_purchase_token_cost().is_none());
        assert!(doc_type.all_document_token_costs().is_empty());
        assert!(doc_type
            .all_external_token_costs_contract_tokens()
            .is_empty());
    }

    // DocumentTypeV1Getters on DocumentTypeRef (V0 variant returns None/defaults)

    #[test]
    fn v1_getters_on_v0_document_type_ref_return_none() {
        let doc_type = get_test_document_type();
        let doc_ref = doc_type.as_ref();

        assert!(doc_ref.document_creation_token_cost().is_none());
        assert!(doc_ref.document_replacement_token_cost().is_none());
        assert!(doc_ref.document_deletion_token_cost().is_none());
        assert!(doc_ref.document_transfer_token_cost().is_none());
        assert!(doc_ref.document_update_price_token_cost().is_none());
        assert!(doc_ref.document_purchase_token_cost().is_none());
        assert!(doc_ref.all_document_token_costs().is_empty());
        assert!(doc_ref
            .all_external_token_costs_contract_tokens()
            .is_empty());
    }

    // DocumentTypeV1Getters on DocumentTypeMutRef (V0 variant returns None/defaults)

    #[test]
    fn v1_getters_on_v0_document_type_mut_ref_return_none() {
        let mut doc_type = get_test_document_type();
        let doc_mut_ref = doc_type.as_mut_ref();

        assert!(doc_mut_ref.document_creation_token_cost().is_none());
        assert!(doc_mut_ref.document_replacement_token_cost().is_none());
        assert!(doc_mut_ref.document_deletion_token_cost().is_none());
        assert!(doc_mut_ref.document_transfer_token_cost().is_none());
        assert!(doc_mut_ref.document_update_price_token_cost().is_none());
        assert!(doc_mut_ref.document_purchase_token_cost().is_none());
        assert!(doc_mut_ref.all_document_token_costs().is_empty());
        assert!(doc_mut_ref
            .all_external_token_costs_contract_tokens()
            .is_empty());
    }

    // DocumentTypeV1Setters on DocumentType (V0 variant is no-op)

    #[test]
    fn v1_setters_on_v0_document_type_are_noop() {
        let mut doc_type = get_test_document_type();
        // These should not panic on V0 variants
        doc_type.set_document_creation_token_cost(None);
        doc_type.set_document_replacement_token_cost(None);
        doc_type.set_document_deletion_token_cost(None);
        doc_type.set_document_transfer_token_cost(None);
        doc_type.set_document_price_update_token_cost(None);
        doc_type.set_document_purchase_token_cost(None);
    }

    // Test with different document types from the fixture

    #[test]
    fn no_time_document_type_getters() {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let doc_types = data_contract.document_types();

        let no_time = doc_types
            .get("noTimeDocument")
            .expect("noTimeDocument should exist");

        assert_eq!(no_time.name(), "noTimeDocument");
        assert!(no_time.indexes().is_empty());
        assert!(no_time.documents_mutable());
    }

    #[test]
    fn with_byte_arrays_document_type_has_binary_paths() {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let doc_types = data_contract.document_types();

        let byte_doc = doc_types
            .get("withByteArrays")
            .expect("withByteArrays should exist");

        assert_eq!(byte_doc.name(), "withByteArrays");
        // This document type has byte array fields
        assert!(!byte_doc.binary_paths().is_empty() || !byte_doc.identifier_paths().is_empty());
        assert!(!byte_doc.indexes().is_empty());
    }

    #[test]
    fn nice_document_type_required_fields() {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let doc_types = data_contract.document_types();

        let nice = doc_types
            .get("niceDocument")
            .expect("niceDocument should exist");

        assert_eq!(nice.name(), "niceDocument");
        // niceDocument requires $createdAt
        assert!(nice.required_fields().contains("$createdAt"));
    }

    #[test]
    fn to_owned_document_type_from_ref() {
        let doc_type = get_test_document_type();
        let doc_ref = doc_type.as_ref();

        let owned = doc_ref.to_owned_document_type();
        assert_eq!(owned.name(), doc_type.name());
        assert_eq!(owned.data_contract_id(), doc_type.data_contract_id());
    }
}
