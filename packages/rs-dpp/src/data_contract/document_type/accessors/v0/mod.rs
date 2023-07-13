use crate::data_contract::document_type::document_field::DocumentField;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};

pub trait DocumentTypeV0Getters {
    /// Returns the name of the document type.
    fn name(&self) -> &String;

    /// Returns the indices of the document type.
    fn indices(&self) -> &Vec<Index>;

    /// Returns the index structure of the document type.
    fn index_structure(&self) -> &IndexLevel;

    /// Returns the flattened properties of the document type.
    fn flattened_properties(&self) -> &BTreeMap<String, DocumentField>;

    /// Returns the properties of the document type.
    fn properties(&self) -> &BTreeMap<String, DocumentField>;

    /// Returns the identifier paths of the document type.
    fn identifier_paths(&self) -> &BTreeSet<String>;

    /// Returns the binary paths of the document type.
    fn binary_paths(&self) -> &BTreeSet<String>;

    /// Returns the required fields of the document type.
    fn required_fields(&self) -> &BTreeSet<String>;

    /// Returns the documents keep history flag of the document type.
    fn documents_keep_history(&self) -> bool;

    /// Returns the documents mutable flag of the document type.
    fn documents_mutable(&self) -> bool;

    /// Returns the data contract id of the document type.
    fn data_contract_id(&self) -> Identifier;

    /// Returns a mutable reference to the indices of the document type.
    fn indices_mut(&mut self) -> &mut Vec<Index>;

    /// Returns a mutable reference to the flattened properties of the document type.
    fn flattened_properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField>;

    /// Returns a mutable reference to the properties of the document type.
    fn properties_mut(&mut self) -> &mut BTreeMap<String, DocumentField>;

    /// Returns a mutable reference to the identifier paths of the document type.
    fn identifier_paths_mut(&mut self) -> &mut BTreeSet<String>;

    /// Returns a mutable reference to the binary paths of the document type.
    fn binary_paths_mut(&mut self) -> &mut BTreeSet<String>;

    /// Returns a mutable reference to the required fields of the document type.
    fn required_fields_mut(&mut self) -> &mut BTreeSet<String>;

    /// Returns a mutable reference to the index structure of the document type.
    fn index_structure_mut(&mut self) -> &mut IndexLevel;
}

pub trait DocumentTypeV0Setters {
    /// Sets the name of the document type.
    fn set_name(&mut self, name: String);

    /// Sets the indices of the document type.
    fn set_indices(&mut self, indices: Vec<Index>);

    /// Sets the index structure of the document type.
    fn set_index_structure(&mut self, index_structure: IndexLevel);

    /// Sets the flattened properties of the document type.
    fn set_flattened_properties(&mut self, flattened_properties: BTreeMap<String, DocumentField>);

    /// Sets the properties of the document type.
    fn set_properties(&mut self, properties: BTreeMap<String, DocumentField>);

    /// Sets the identifier paths of the document type.
    fn set_identifier_paths(&mut self, identifier_paths: BTreeSet<String>);

    /// Sets the binary paths of the document type.
    fn set_binary_paths(&mut self, binary_paths: BTreeSet<String>);

    /// Sets the required fields of the document type.
    fn set_required_fields(&mut self, required_fields: BTreeSet<String>);

    /// Sets the documents keep history flag of the document type.
    fn set_documents_keep_history(&mut self, documents_keep_history: bool);

    /// Sets the documents mutable flag of the document type.
    fn set_documents_mutable(&mut self, documents_mutable: bool);

    /// Sets the data contract id of the document type.
    fn set_data_contract_id(&mut self, data_contract_id: Identifier);
}
