use crate::data_contract::document_type::config::DocumentTypeConfig;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;

mod v0;

impl DocumentType {
    pub fn try_from_schema(
        data_contract_id: Identifier,
        name: &str,
        schema: Value,
        schema_defs: Option<&Value>,
        config: DocumentTypeConfig,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .try_from_schema
        {
            0 => DocumentTypeV0::try_from_schema_v0(
                data_contract_id,
                name,
                schema,
                schema_defs,
                config,
                validate,
                platform_version,
            )
            .map(|document_type| document_type.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "try_from_schema".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
