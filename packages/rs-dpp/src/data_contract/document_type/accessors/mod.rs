mod v0;

use crate::data_contract::document_type::document_field::DocumentField;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};
pub use v0::*;

impl DocumentTypeV0Getters for DocumentType {
    fn name(&self) -> &String {
        match self {
            DocumentType::V0(v0) => v0.name(),
        }
    }

    fn indices(&self) -> &Vec<Index> {
        match self {
            DocumentType::V0(v0) => v0.indices(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentType::V0(v0) => v0.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentField> {
        match self {
            DocumentType::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &BTreeMap<String, DocumentField> {
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

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentType::V0(v0) => v0.data_contract_id(),
        }
    }

    fn indices_mut(&mut self) -> &mut Vec<Index> {
        match self {
            DocumentType::V0(v0) => v0.indices_mut(),
        }
    }

    fn flattened_properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField> {
        match self {
            DocumentType::V0(v0) => v0.flattened_properties_mut(),
        }
    }

    fn properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField> {
        match self {
            DocumentType::V0(v0) => v0.properties_mut(),
        }
    }

    fn identifier_paths_mut(&mut self) -> &mut BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.identifier_paths_mut(),
        }
    }

    fn binary_paths_mut(&mut self) -> &mut BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.binary_paths_mut(),
        }
    }

    fn required_fields_mut(&mut self) -> &mut BTreeSet<String> {
        match self {
            DocumentType::V0(v0) => v0.required_fields_mut(),
        }
    }

    fn index_structure_mut(&mut self) -> &mut IndexLevel {
        match self {
            DocumentType::V0(v0) => v0.index_structure_mut(),
        }
    }
}

impl<'a> DocumentTypeV0Getters for DocumentTypeRef<'a> {
    fn name(&self) -> &String {
        match self {
            DocumentTypeRef::V0(v0) => v0.name(),
        }
    }

    fn indices(&self) -> &Vec<Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.indices(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentField> {
        match self {
            DocumentTypeRef::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &BTreeMap<String, DocumentField> {
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

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeRef::V0(v0) => v0.data_contract_id(),
        }
    }

    fn indices_mut(&mut self) -> &mut Vec<Index> {
        match self {
            DocumentTypeRef::V0(v0) => v0.indices_mut(),
        }
    }

    fn flattened_properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField> {
        match self {
            DocumentTypeRef::V0(v0) => v0.flattened_properties_mut(),
        }
    }

    fn properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField> {
        match self {
            DocumentTypeRef::V0(v0) => v0.properties_mut(),
        }
    }

    fn identifier_paths_mut(&mut self) -> &mut BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.identifier_paths_mut(),
        }
    }

    fn binary_paths_mut(&mut self) -> &mut BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.binary_paths_mut(),
        }
    }

    fn required_fields_mut(&mut self) -> &mut BTreeSet<String> {
        match self {
            DocumentTypeRef::V0(v0) => v0.required_fields_mut(),
        }
    }

    fn index_structure_mut(&mut self) -> &mut IndexLevel {
        match self {
            DocumentTypeRef::V0(v0) => v0.index_structure_mut(),
        }
    }
}

impl DocumentTypeV0Setters for DocumentType {
    fn set_name(&mut self, name: String) {
        match self {
            DocumentType::V0(v0) => v0.set_name(name),
        }
    }

    fn set_indices(&mut self, indices: Vec<Index>) {
        match self {
            DocumentType::V0(v0) => v0.set_indices(indices),
        }
    }

    fn set_index_structure(&mut self, index_structure: IndexLevel) {
        match self {
            DocumentType::V0(v0) => v0.set_index_structure(index_structure),
        }
    }

    fn set_flattened_properties(&mut self, flattened_properties: BTreeMap<String, DocumentField>) {
        match self {
            DocumentType::V0(v0) => v0.set_flattened_properties(flattened_properties),
        }
    }

    fn set_properties(&mut self, properties: BTreeMap<String, DocumentField>) {
        match self {
            DocumentType::V0(v0) => v0.set_properties(properties),
        }
    }

    fn set_identifier_paths(&mut self, identifier_paths: BTreeSet<String>) {
        match self {
            DocumentType::V0(v0) => v0.set_identifier_paths(identifier_paths),
        }
    }

    fn set_binary_paths(&mut self, binary_paths: BTreeSet<String>) {
        match self {
            DocumentType::V0(v0) => v0.set_binary_paths(binary_paths),
        }
    }

    fn set_required_fields(&mut self, required_fields: BTreeSet<String>) {
        match self {
            DocumentType::V0(v0) => v0.set_required_fields(required_fields),
        }
    }

    fn set_documents_keep_history(&mut self, documents_keep_history: bool) {
        match self {
            DocumentType::V0(v0) => v0.set_documents_keep_history(documents_keep_history),
        }
    }

    fn set_documents_mutable(&mut self, documents_mutable: bool) {
        match self {
            DocumentType::V0(v0) => v0.set_documents_mutable(documents_mutable),
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentType::V0(v0) => v0.set_data_contract_id(data_contract_id),
        }
    }
}

impl<'a> DocumentTypeV0Setters for DocumentTypeRef<'a> {
    fn set_name(&mut self, name: String) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_name(name),
        }
    }

    fn set_indices(&mut self, indices: Vec<Index>) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_indices(indices),
        }
    }

    fn set_index_structure(&mut self, index_structure: IndexLevel) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_index_structure(index_structure),
        }
    }

    fn set_flattened_properties(&mut self, flattened_properties: BTreeMap<String, DocumentField>) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_flattened_properties(flattened_properties),
        }
    }

    fn set_properties(&mut self, properties: BTreeMap<String, DocumentField>) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_properties(properties),
        }
    }

    fn set_identifier_paths(&mut self, identifier_paths: BTreeSet<String>) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_identifier_paths(identifier_paths),
        }
    }

    fn set_binary_paths(&mut self, binary_paths: BTreeSet<String>) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_binary_paths(binary_paths),
        }
    }

    fn set_required_fields(&mut self, required_fields: BTreeSet<String>) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_required_fields(required_fields),
        }
    }

    fn set_documents_keep_history(&mut self, documents_keep_history: bool) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_documents_keep_history(documents_keep_history),
        }
    }

    fn set_documents_mutable(&mut self, documents_mutable: bool) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_documents_mutable(documents_mutable),
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentTypeRef::V0(v0) => v0.set_data_contract_id(data_contract_id),
        }
    }
}
