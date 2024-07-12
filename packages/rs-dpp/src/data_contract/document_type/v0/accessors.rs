use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::v0::DocumentTypeV0;

use platform_value::{Identifier, Value};

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

impl DocumentTypeV0Getters for DocumentTypeV0 {
    fn name(&self) -> &String {
        &self.name
    }

    fn schema(&self) -> &Value {
        &self.schema
    }

    fn schema_owned(self) -> Value {
        self.schema
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        &self.indices
    }

    fn find_contested_index(&self) -> Option<&Index> {
        self.indices
            .iter()
            .find(|(_, index)| index.contested_index.is_some())
            .map(|(_, contested_index)| contested_index)
    }

    fn index_structure(&self) -> &IndexLevel {
        &self.index_structure
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        &self.flattened_properties
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        &self.properties
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        &self.identifier_paths
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        &self.binary_paths
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        &self.required_fields
    }

    fn documents_keep_history(&self) -> bool {
        self.documents_keep_history
    }

    fn documents_mutable(&self) -> bool {
        self.documents_mutable
    }

    fn documents_can_be_deleted(&self) -> bool {
        self.documents_can_be_deleted
    }

    fn documents_transferable(&self) -> Transferable {
        self.documents_transferable
    }

    fn trade_mode(&self) -> TradeMode {
        self.trade_mode
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        self.creation_restriction_mode
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        self.requires_identity_encryption_bounded_key
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        self.requires_identity_decryption_bounded_key
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        self.security_level_requirement
    }
}
