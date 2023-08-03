use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

mod v0;

impl DataContract {
    // TODO: Used only in test for itself, remove?
    pub(crate) fn get_property_definition_by_path<'a>(
        document_definition: &'a JsonValue,
        path: &str,
        platform_version: &PlatformVersion,
    ) -> Result<&'a JsonValue, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_class_method_versions
            .get_property_definition_by_path
        {
            0 => Self::get_property_definition_by_path_v0(document_definition, path),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "get_property_definition_by_path".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::assert_error_contains;

    use super::*;

    #[test]
    fn test_get_system_properties() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result = DataContract::get_property_definition_by_path(
            &schema,
            "$protocolVersion",
            platform_version,
        );
        let expect = json!({
            "type": "integer",
            "$comment": "Maximum is the latest protocol version"
        });
        assert_eq!(result.unwrap(), &expect);
    }

    #[test]
    fn test_top_level_property_not_found() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result =
            DataContract::get_property_definition_by_path(&schema, "nope", platform_version);
        assert_error_contains!(result, "the top-level property 'nope' cannot be found");
    }

    #[test]
    fn test_top_level_property_returned() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result = DataContract::get_property_definition_by_path(&schema, "a", platform_version);
        assert_eq!(result.unwrap(), &json!({"type" : "string"}));
    }

    #[test]
    fn test_return_nested_def_from_array() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result =
            DataContract::get_property_definition_by_path(&schema, "b.inner", platform_version);
        let expect = json!({
                "type": "object",
                "properties": {
                    "abc": {
                        "type": "string"
                    }
                }
        });
        assert_eq!(result.unwrap(), &expect);
    }

    #[test]
    fn should_return_nested_definition_from_object() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result =
            DataContract::get_property_definition_by_path(&schema, "c.inner", platform_version);
        let expect = json!({
                "type": "object",
                "patternProperties": {
                    "[a-z]": {
                        "type": "string"
                    }
                }

        });
        assert_eq!(result.unwrap(), &expect);
    }

    #[test]
    fn test_should_return_error_if_not_match_pattern() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result = DataContract::get_property_definition_by_path(
            &schema,
            "c.inner.NOPE",
            platform_version,
        );
        assert_error_contains!(result, "Couldn't find 'properties'");
    }

    #[test]
    fn test_error_if_top_level_not_array_or_object() {
        let schema = get_schema();
        let platform_version = PlatformVersion::latest();
        let result =
            DataContract::get_property_definition_by_path(&schema, "a.someOther", platform_version);
        assert_error_contains!(result, "the 'someOther' is not array or object");
    }

    fn get_schema() -> JsonValue {
        json!({
            "properties": {
                "a": {
                    "type": "string"
                },
                "b": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "inner": {
                                "type": "object",
                                "properties": {
                                    "abc": {
                                        "type": "string"
                                    }
                                }
                            }
                        }
                    }
                },
                "c": {
                    "type": "object",
                    "properties": {
                        "inner": {
                            "type": "object",
                            "patternProperties": {
                                "[a-z]": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}
