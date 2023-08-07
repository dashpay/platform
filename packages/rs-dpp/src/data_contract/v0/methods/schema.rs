use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::data_contract_methods::schema::DataContractSchemaMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DefinitionName, DocumentName};
use crate::prelude::DataContract;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DataContractSchemaMethodsV0 for DataContractV0 {
    fn set_document_schemas(
        &mut self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        self.document_types = DataContract::create_document_types_from_document_schemas(
            self.id,
            schemas,
            &defs,
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            platform_version,
        )?;

        Ok(())
    }

    fn document_schemas(&self) -> BTreeMap<DocumentName, &Value> {
        self.document_types
            .iter()
            .map(|(name, r#type)| (name.to_owned(), r#type.schema()))
            .collect()
    }

    fn schema_defs(&self) -> &Option<BTreeMap<DefinitionName, Value>> {
        &self.schema_defs
    }

    fn set_schema_defs(
        &mut self,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let document_schemas = self
            .document_types
            .iter()
            .map(|(name, r#type)| (name.to_owned(), r#type.schema().to_owned()))
            .collect();

        self.set_document_schemas(document_schemas, defs, platform_version)
    }
}
