use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DefinitionName, DocumentName};
use crate::validation::operations::ProtocolValidationOperation;
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
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        self.document_types = DocumentType::create_document_types_from_document_schemas(
            self.id,
            schemas,
            defs.as_ref(),
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            self.config.documents_can_be_deleted_contract_default(),
            validate,
            validation_operations,
            platform_version,
        )?;

        Ok(())
    }

    fn set_document_schema(
        &mut self,
        name: &str,
        schema: Value,
        validate: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let document_type = DocumentType::try_from_schema(
            self.id,
            name,
            schema,
            self.schema_defs.as_ref(),
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            self.config.documents_mutable_contract_default(),
            validate,
            validation_operations,
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
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let document_schemas = self
            .document_types
            .iter()
            .map(|(name, document_type)| (name.to_owned(), document_type.schema().to_owned()))
            .collect();

        self.set_document_schemas(
            document_schemas,
            defs.clone(),
            validate,
            validation_operations,
            platform_version,
        )?;

        self.schema_defs = defs;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
    use crate::data_contract::v0::DataContractV0;
    use platform_value::{platform_value, Identifier};

    #[test]
    fn should_set_a_new_schema_defs() {
        let platform_version = PlatformVersion::latest();

        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");

        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "string",
                    "maxLength": 10,
                    "position": 0
                }
            },
            "additionalProperties": false,
        });

        let serialization_format = DataContractInSerializationFormatV0 {
            id: Identifier::random(),
            config,
            version: 0,
            owner_id: Default::default(),
            schema_defs: None,
            document_schemas: BTreeMap::from([("document_type_name".to_string(), schema.clone())]),
        };

        let mut data_contract = DataContractV0::try_from_platform_versioned(
            serialization_format.into(),
            true,
            &mut vec![],
            platform_version,
        )
        .expect("should create a contract from serialization format");

        let defs = platform_value!({
            "test": {
                "type": "string",
            },
        });

        let defs_map = Some(defs.into_btree_string_map().expect("should convert to map"));

        data_contract
            .set_schema_defs(defs_map.clone(), true, &mut vec![], platform_version)
            .expect("should set defs");

        assert_eq!(defs_map.as_ref(), data_contract.schema_defs())
    }

    fn should_set_empty_schema_defs() {
        let platform_version = PlatformVersion::latest();

        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");

        let defs = platform_value!({
            "test": {
                "type": "string",
            },
        });

        let defs_map = Some(defs.into_btree_string_map().expect("should convert to map"));

        let serialization_format = DataContractInSerializationFormatV0 {
            id: Identifier::random(),
            config,
            version: 0,
            owner_id: Default::default(),
            schema_defs: defs_map,
            document_schemas: Default::default(),
        };

        let mut data_contract = DataContractV0::try_from_platform_versioned(
            serialization_format.into(),
            true,
            &mut vec![],
            platform_version,
        )
        .expect("should create a contract from serialization format");

        data_contract
            .set_schema_defs(None, true, &mut vec![], platform_version)
            .expect("should set defs");

        assert_eq!(None, data_contract.schema_defs())
    }
}
