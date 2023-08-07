mod v0;

use crate::data_contract::document_type::document_field::DocumentProperty;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::{DocumentType, DocumentTypeMutRef, DocumentTypeRef};
use crate::data_contract::{JsonSchema, PropertyPath};
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
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

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentProperty> {
        match self {
            DocumentType::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &BTreeMap<String, DocumentProperty> {
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

    fn binary_properties(&self) -> &BTreeMap<PropertyPath, Value> {
        match self {
            DocumentType::V0(v0) => v0.binary_properties(),
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

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &BTreeMap<String, DocumentProperty> {
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

    fn binary_properties(&self) -> &BTreeMap<PropertyPath, Value> {
        match self {
            DocumentTypeRef::V0(v0) => v0.binary_properties(),
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

    fn indices(&self) -> &Vec<Index> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.indices(),
        }
    }

    fn index_structure(&self) -> &IndexLevel {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.index_structure(),
        }
    }

    fn flattened_properties(&self) -> &BTreeMap<String, DocumentProperty> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.flattened_properties(),
        }
    }

    fn properties(&self) -> &BTreeMap<String, DocumentProperty> {
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

    fn binary_properties(&self) -> &BTreeMap<PropertyPath, Value> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.binary_properties(),
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

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.data_contract_id(),
        }
    }
}

impl DocumentTypeV0Setters for DocumentType {
    fn set_schema(
        &mut self,
        schema: Value,
        schema_defs: &Option<BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.set_schema(schema, schema_defs, platform_version),
        }
    }
}

impl<'a> DocumentTypeV0Setters for DocumentTypeMutRef<'a> {
    fn set_schema(
        &mut self,
        schema: Value,
        schema_defs: &Option<BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DocumentTypeMutRef::V0(v0) => v0.set_schema(schema, schema_defs, platform_version),
        }
    }
}
