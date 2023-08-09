use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV0Setters,
};
use crate::data_contract::document_type::document_field::DocumentProperty;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{JsonSchema, PropertyPath};
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

    fn binary_properties(&self) -> &BTreeMap<PropertyPath, Value> {
        &self.binary_properties
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

impl DocumentTypeV0Setters for DocumentTypeV0 {
    fn set_schema(
        &mut self,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let DocumentTypeV0 {
            schema,
            indices,
            index_structure,
            flattened_properties,
            properties,
            binary_properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            ..
        } = DocumentTypeV0::from_platform_value(
            self.data_contract_id,
            self.name.as_str(),
            schema,
            schema_defs,
            self.documents_keep_history,
            self.documents_mutable,
            platform_version,
        )?;

        self.schema = schema;
        self.indices = indices;
        self.index_structure = index_structure;
        self.flattened_properties = flattened_properties;
        self.properties = properties;
        self.binary_properties = binary_properties;
        self.identifier_paths = identifier_paths;
        self.binary_paths = binary_paths;
        self.required_fields = required_fields;
        self.documents_keep_history = documents_keep_history;
        self.documents_mutable = documents_mutable;

        Ok(())
    }
}
