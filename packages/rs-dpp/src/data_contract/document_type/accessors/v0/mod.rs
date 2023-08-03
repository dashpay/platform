use crate::data_contract::document_type::document_field::DocumentField;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::{JsonSchema, PropertyPath};
use platform_value::{Identifier, Value};
use std::collections::{BTreeMap, BTreeSet};

// TODO: Verify we need all of these getters
pub trait DocumentTypeV0Getters {
    /// Returns the name of the document type.
    fn name(&self) -> &String;

    fn schema(&self) -> &Value;

    fn schema_owned(self) -> Value;

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

    fn binary_properties(&self) -> &BTreeMap<PropertyPath, Value>;

    /// Returns the required fields of the document type.
    fn required_fields(&self) -> &BTreeSet<String>;

    /// Returns the documents keep history flag of the document type.
    fn documents_keep_history(&self) -> bool;

    /// Returns the documents mutable flag of the document type.
    fn documents_mutable(&self) -> bool;

    /// Returns the data contract id of the document type.
    fn data_contract_id(&self) -> Identifier;
}

pub trait DocumentTypeV0Setters {
    /// Sets document json schema of the document type.
    fn set_schema(&mut self, schema: Value);
}
