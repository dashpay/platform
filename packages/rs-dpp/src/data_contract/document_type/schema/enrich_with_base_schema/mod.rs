mod v0;

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DocumentType {
    pub fn enrich_with_base_schema(
        schema: Value,
        schema_defs: Option<Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .enrich_with_base_schema
        {
            0 => Ok(
                v0::enrich_with_base_schema_v0(schema, schema_defs).map_err(|e| {
                    ProtocolError::ConsensusError(
                        ConsensusError::BasicError(BasicError::ContractError(e)).into(),
                    )
                })?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "enrich_with_base_schema".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
