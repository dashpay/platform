use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::data_contract_methods::base::DataContractBaseMethodsV0;
use crate::data_contract::data_contract_methods::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
use crate::data_contract::data_contract_methods::schema::DataContractSchemaMethodsV0;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DataContract, DefinitionName, DocumentName, JsonSchema};
use crate::version::PlatformVersion;
use crate::{identifier, ProtocolError};
use itertools::{Either, Itertools};
use platform_value::string_encoding::Encoding;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashSet};

impl DataContractSchemaMethodsV0 for DataContractV0 {
    fn set_document_json_schema(
        &mut self,
        doc_type: String,
        schema: JsonSchema,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let binary_properties = DataContract::create_binary_properties(&schema, platform_version)?;
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

    fn set_document_schemas(
        &self,
        schemas: BTreeMap<DocumentName, JsonSchema>,
        defs: Option<BTreeMap<DefinitionName, JsonSchema>>,
    ) {
        todo!()
    }

    fn document_schemas(&self) -> &BTreeMap<DocumentName, JsonSchema> {
        self.document_types
            .iter()
            .map(|name, r#type| (name, r#type.schema()))
            .collect()
    }

    fn schema_defs(&self) -> &Option<BTreeMap<DefinitionName, JsonSchema>> {
        &self.schema_defs
    }

    fn set_schema_defs(&self, defs: Option<BTreeMap<DefinitionName, JsonSchema>>) {
        todo!()
    }
}
