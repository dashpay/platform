use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::data_contract_methods::base::DataContractBaseMethodsV0;
use crate::data_contract::data_contract_methods::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
use crate::data_contract::data_contract_methods::schema::DataContractSchemaMethodsV0;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::{identifier, ProtocolError};
use itertools::{Either, Itertools};
use platform_value::string_encoding::Encoding;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashSet};

impl DataContractSchemaMethodsV0 for DataContractV0 {
    fn document_json_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ));
        };

        Ok(format!(
            "{}#/documents/{}",
            self.id.to_string(Encoding::Base58),
            doc_type
        ))
    }

    fn document_json_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        let document = self
            .documents
            .get(doc_type)
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ))?;
        Ok(document)
    }

    fn set_document_json_schema(
        &mut self,
        doc_type: String,
        schema: JsonSchema,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let binary_properties = DataContract::get_binary_properties(&schema, platform_version)?;
        self.documents.insert(doc_type.clone(), schema.clone());
        self.binary_properties
            .insert(doc_type.clone(), binary_properties);

        let document_type_value = platform_value::Value::from(schema);

        // Make sure the document_type_value is a map
        let Some(document_type_value_map) = document_type_value.as_map() else {
            return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            )));
        };

        let document_type = DocumentTypeV0::from_platform_value(
            self.id,
            &doc_type,
            document_type_value_map,
            &BTreeMap::new(),
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            platform_version,
        )?;

        self.document_types.insert(doc_type, document_type.into());

        Ok(())
    }
}
