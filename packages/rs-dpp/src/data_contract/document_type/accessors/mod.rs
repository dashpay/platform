mod v0;

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::{DocumentType, DocumentTypeMutRef, DocumentTypeRef};

use platform_value::{Identifier, Value};

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};
pub use v0::*;

impl DocumentTypeV0Getters for DocumentType {
    fn name(&self) -> &String {
        match self {
            DocumentType::V0(v0) => v0.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentType::V0(v0) => v0.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentType::V0(v0) => v0.schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentType::V0(v0) => v0.indexes(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentType::V0(v0) => v0.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.required_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_keep_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentType::V0(v0) => v0.documents_can_be_deleted(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentType::V0(v0) => v0.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentType::V0(v0) => v0.creation_restriction_mode(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentType::V0(v0) => v0.documents_transferable(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentType::V0(v0) => v0.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentType::V0(v0) => v0.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentType::V0(v0) => v0.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentType::V0(v0) => v0.security_level_requirement(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentType::V0(v0) => v0.find_contested_index(),
        }
    }
}

impl<'a> DocumentTypeV0Getters for DocumentTypeRef<'a> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeRef::V0(v0) => v0.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentTypeRef::V0(v0) => v0.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentTypeRef::V0(v0) => v0.clone().schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.indexes(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.required_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_keep_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentTypeRef::V0(v0) => v0.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentTypeRef::V0(v0) => v0.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentTypeRef::V0(v0) => v0.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeRef::V0(v0) => v0.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.security_level_requirement(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.find_contested_index(),
        }
    }
}

impl<'a> DocumentTypeV0Getters for DocumentTypeMutRef<'a> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.name(),
        }
    }

    fn schema(&self) -> &Value {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.schema(),
        }
    }

    fn schema_owned(self) -> Value {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.clone().schema_owned(),
        }
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.indexes(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.properties(),
        }
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.identifier_paths(),
        }
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.binary_paths(),
        }
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.required_fields(),
        }
    }

    fn documents_keep_history(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_keep_history(),
        }
    }

    fn documents_mutable(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_mutable(),
        }
    }

    fn documents_can_be_deleted(&self) -> bool {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_can_be_deleted(),
        }
    }

    fn documents_transferable(&self) -> Transferable {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.documents_transferable(),
        }
    }

    fn trade_mode(&self) -> TradeMode {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.trade_mode(),
        }
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.creation_restriction_mode(),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.data_contract_id(),
        }
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.requires_identity_encryption_bounded_key(),
        }
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.requires_identity_decryption_bounded_key(),
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.security_level_requirement(),
        }
    }

    fn find_contested_index(&self) -> Option<&Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.find_contested_index(),
        }
    }
}
