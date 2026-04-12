mod v0;
mod v1;

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
            1 => Ok(
                v1::enrich_with_base_schema_v1(schema, schema_defs).map_err(|e| {
                    ProtocolError::ConsensusError(
                        ConsensusError::BasicError(BasicError::ContractError(e)).into(),
                    )
                })?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "enrich_with_base_schema".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::{platform_value, ValueMapHelper};

    fn minimal_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "additionalProperties": false
        })
    }

    #[test]
    fn v0_enrichment_injects_v0_schema_uri() {
        let platform_version = PlatformVersion::get(11).expect("expected v11");
        let enriched =
            DocumentType::enrich_with_base_schema(minimal_schema(), None, platform_version)
                .expect("enrichment should succeed");

        let map = enriched.to_map_ref().expect("should be map");
        let schema_value = map
            .get_optional_key("$schema")
            .expect("should have $schema");
        let schema_uri = schema_value.as_text().expect("should be text");

        assert!(
            schema_uri.contains("/v0/document-meta.json"),
            "pre-v12 should use v0 URI, got: {schema_uri}"
        );
    }

    #[test]
    fn v1_enrichment_injects_v1_schema_uri() {
        let platform_version = PlatformVersion::latest();
        let enriched =
            DocumentType::enrich_with_base_schema(minimal_schema(), None, platform_version)
                .expect("enrichment should succeed");

        let map = enriched.to_map_ref().expect("should be map");
        let schema_value = map
            .get_optional_key("$schema")
            .expect("should have $schema");
        let schema_uri = schema_value.as_text().expect("should be text");

        assert!(
            schema_uri.contains("/v1/document-meta.json"),
            "v12+ should use v1 URI, got: {schema_uri}"
        );
    }
}
