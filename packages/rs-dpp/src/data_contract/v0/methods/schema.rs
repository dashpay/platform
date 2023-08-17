use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DefinitionName, DocumentName};
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DataContractSchemaMethodsV0 for DataContractV0 {
    fn set_document_schemas(
        &mut self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        self.document_types = DocumentType::create_document_types_from_document_schemas(
            self.id,
            schemas,
            defs.as_ref(),
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            validate,
            platform_version,
        )?;

        Ok(())
    }

    fn set_document_schema(
        &mut self,
        name: &str,
        schema: Value,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let document_type = DocumentType::try_from_schema(
            self.id,
            name,
            schema,
            self.schema_defs.as_ref(),
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            validate,
            platform_version,
        )?;

        self.document_types
            .insert(document_type.name().clone(), document_type);

        Ok(())
    }

    fn document_schemas(&self) -> BTreeMap<DocumentName, &Value> {
        self.document_types
            .iter()
            .map(|(name, document_type)| (name.to_owned(), document_type.schema()))
            .collect()
    }

    fn schema_defs(&self) -> Option<&BTreeMap<DefinitionName, Value>> {
        self.schema_defs.as_ref()
    }

    fn set_schema_defs(
        &mut self,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let document_schemas = self
            .document_types
            .iter()
            .map(|(name, document_type)| (name.to_owned(), document_type.schema().to_owned()))
            .collect();

        self.set_document_schemas(document_schemas, defs, validate, platform_version)
    }
}
