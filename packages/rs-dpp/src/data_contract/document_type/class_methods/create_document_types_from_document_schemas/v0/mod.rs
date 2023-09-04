use crate::data_contract::document_type::config::DocumentTypeConfig;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::DocumentName;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentTypeV0 {
    pub(in crate::data_contract) fn create_document_types_from_document_schemas_v0(
        data_contract_id: Identifier,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&Value>,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        let config = DocumentTypeConfig::default_with_platform_version(platform_version)?;

        for (name, schema) in document_schemas.into_iter() {
            let document_type = match platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .structure_version
            {
                0 => DocumentType::try_from_schema(
                    data_contract_id,
                    &name,
                    schema,
                    schema_defs,
                    config.clone(),
                    validate,
                    platform_version,
                )?,
                version => {
                    return Err(ProtocolError::UnknownVersionMismatch {
                        method: "get_document_types_from_value_array_v0 inner document type"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })
                }
            };

            contract_document_types.insert(name.to_string(), document_type);
        }
        Ok(contract_document_types)
    }
}
