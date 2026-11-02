use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MaxDepthValidationResult {
    pub depth: u16,
    pub size: u64,
}

pub fn validate_max_depth(
    value: &Value,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<MaxDepthValidationResult>, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validate_max_depth
    {
        0 => Ok(v0::validate_max_depth_v0(value, platform_version)),
        1 => Ok(v1::validate_max_depth_v1(value, platform_version)),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_max_depth".to_string(),
            known_versions: vec![0, 1],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use serde_json::json;

    /// A `$defs` level whose two `$ref`s to `d1` are separated by `$ref`s to a scalar, which
    /// generation 0 walks and generation 1 does not.
    fn scalar_ref_reset_schema() -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {
                "d0": {
                    "a": { "$ref": "#/$defs/d1" },
                    "m": { "$ref": "#/$schema" },
                    "y": { "$ref": "#/$defs/d1" },
                    "zz": { "$ref": "#/$schema" },
                },
                "d1": {},
            },
            "type": "object",
            "properties": { "foo": { "type": "integer" } },
            "additionalProperties": false,
        })
        .into()
    }

    fn ordinary_schema() -> Value {
        json!({
            "$defs": { "object": { "nested": { "type": "string" } } },
            "type": "object",
            "properties": {
                "foo": { "type": "integer" },
                "fooWithRef": { "$ref": "#/$defs/object" },
            },
            "additionalProperties": false,
        })
        .into()
    }

    #[test]
    fn should_accept_a_scalar_ref_reset_before_protocol_version_14_and_refuse_it_from_14() {
        let schema = scalar_ref_reset_schema();

        let historical = validate_max_depth(
            &schema,
            PlatformVersion::get(13).expect("expected protocol version 13"),
        )
        .expect("expected a known version");
        assert!(historical.is_valid(), "{:?}", historical.errors);
        assert_eq!(
            historical.data,
            Some(MaxDepthValidationResult { depth: 5, size: 18 })
        );

        let pv14 = validate_max_depth(
            &schema,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        )
        .expect("expected a known version");
        assert!(
            matches!(
                pv14.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::InvalidJsonSchemaRefError(e))]
                    if e.to_string().contains("contains cycles")
            ),
            "{:?}",
            pv14.errors
        );
    }

    #[test]
    fn should_measure_an_ordinary_schema_alike_across_protocol_version_14() {
        let schema = ordinary_schema();

        let historical = validate_max_depth(
            &schema,
            PlatformVersion::get(13).expect("expected protocol version 13"),
        )
        .expect("expected a known version");
        let pv14 = validate_max_depth(
            &schema,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        )
        .expect("expected a known version");
        assert!(historical.is_valid(), "{:?}", historical.errors);
        assert!(pv14.is_valid(), "{:?}", pv14.errors);
        assert_eq!(
            historical.data,
            Some(MaxDepthValidationResult { depth: 5, size: 14 })
        );
        assert_eq!(pv14.data, historical.data);
    }
}
