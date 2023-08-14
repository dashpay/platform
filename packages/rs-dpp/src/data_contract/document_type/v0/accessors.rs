use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
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

    fn indices(&self) -> &Vec<Index> {
        &self.indices
    }

    fn index_structure(&self) -> &IndexLevel {
        &self.index_structure
    }

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentProperty> {
        &self.flattened_properties
    }

    fn properties(&self) -> &BTreeMap<String, DocumentProperty> {
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

    fn document_revisions(&self) -> bool {
        self.documents_keep_history
    }

    fn documents_read_only(&self) -> bool {
        self.documents_mutable
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
}
