use crate::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV0MutGetters, DocumentTypeV0Setters};
use crate::data_contract::document_type::document_field::DocumentField;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};

impl DocumentTypeV0Getters for DocumentTypeV0 {
    fn name(&self) -> &String {
        &self.name
    }

    fn indices(&self) -> &Vec<Index> {
        &self.indices
    }

    fn index_structure(&self) -> &IndexLevel {
        &self.index_structure
    }

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentField> {
        &self.flattened_properties
    }

    fn properties(&self) -> &BTreeMap<String, DocumentField> {
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

    fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
}

impl DocumentTypeV0MutGetters for DocumentTypeV0 {

    fn indices_mut(&mut self) -> &mut Vec<Index> {
        &mut self.indices
    }

    fn flattened_properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField> {
        &mut self.flattened_properties
    }

    fn properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField> {
        &mut self.properties
    }

    fn identifier_paths_mut(&mut self) -> &mut BTreeSet<String> {
        &mut self.identifier_paths
    }

    fn binary_paths_mut(&mut self) -> &mut BTreeSet<String> {
        &mut self.binary_paths
    }

    fn required_fields_mut(&mut self) -> &mut BTreeSet<String> {
        &mut self.required_fields
    }

    fn index_structure_mut(&mut self) -> &mut IndexLevel {
        &mut self.index_structure
    }
}

impl DocumentTypeV0Setters for DocumentTypeV0 {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn set_indices(&mut self, indices: Vec<Index>) {
        self.indices = indices;
    }

    fn set_index_structure(&mut self, index_structure: IndexLevel) {
        self.index_structure = index_structure;
    }

    fn set_flattened_properties(&mut self, flattened_properties: BTreeMap<String, DocumentField>) {
        self.flattened_properties = flattened_properties;
    }

    fn set_properties(&mut self, properties: BTreeMap<String, DocumentField>) {
        self.properties = properties;
    }

    fn set_identifier_paths(&mut self, identifier_paths: BTreeSet<String>) {
        self.identifier_paths = identifier_paths;
    }

    fn set_binary_paths(&mut self, binary_paths: BTreeSet<String>) {
        self.binary_paths = binary_paths;
    }

    fn set_required_fields(&mut self, required_fields: BTreeSet<String>) {
        self.required_fields = required_fields;
    }

    fn set_documents_keep_history(&mut self, documents_keep_history: bool) {
        self.documents_keep_history = documents_keep_history;
    }

    fn set_documents_mutable(&mut self, documents_mutable: bool) {
        self.documents_mutable = documents_mutable;
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        self.data_contract_id = data_contract_id;
    }
}
