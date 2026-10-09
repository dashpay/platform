mod v0;

use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DocumentType {
    /// Rewrites the property type shorthands of `schema` into the long form
    /// they stand for, so that everything reading the schema reads one form:
    ///
    /// * `"type": "identifier"` is `"type": "array", "byteArray": true,
    ///   "minItems": 32, "maxItems": 32, "contentMediaType":
    ///   "application/x.dash.dpp.identifier"`;
    /// * `"type": "bytes", "size": n` is `"type": "array", "byteArray": true,
    ///   "minItems": n, "maxItems": n`.
    ///
    /// `schema` is a document type schema, the root schema
    /// [`DocumentType::enrich_with_base_schema`] builds from one, or any other
    /// object whose `properties` and `$defs` hold property schemas. A shorthand
    /// is read on every property schema below them: the members of an object,
    /// at any depth, and the `items` of a typed array. A property schema may
    /// not write `byteArray`, `minItems`, `maxItems` or `contentMediaType`
    /// beside a shorthand, nor `size` beside `identifier`, and `bytes` needs a
    /// `size` from 1 to `max_field_value_size` (the largest value any document
    /// field may hold, so a larger size could never be filled), the bound
    /// checked under `full_validation` only, like every other bound read from
    /// the tables (`InvalidContractStructure`, 10231).
    ///
    /// Returns `None` when there is nothing to rewrite, so the caller keeps the
    /// schema it holds. The contract keeps the schema as sent: this is a view
    /// of it, never what is stored.
    ///
    /// Versioned on `expand_property_type_shorthands` in the platform
    /// version's document type schema versions. `None` selects the behavior of
    /// the versions that predate the shorthands: nothing is rewritten, and
    /// every earlier meta-schema refuses both shorthands (`identifier` and
    /// `bytes` are no JSON Schema type), so the shipped callers that reach this
    /// on those versions behave exactly as before.
    pub fn expand_property_type_shorthands(
        schema: &Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Value>, DataContractError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .expand_property_type_shorthands
        {
            None => Ok(None),
            Some(0) => {
                v0::expand_property_type_shorthands_v0(schema, full_validation, platform_version)
            }
            Some(version) => Err(DataContractError::Unsupported(format!(
                "expand_property_type_shorthands version {version} is not supported"
            ))),
        }
    }

    /// [`DocumentType::expand_property_type_shorthands`] for the contract's
    /// `$defs`, each definition a property schema, as the parse and the
    /// contract update comparison hold them. Returns `None` when there is
    /// nothing to rewrite.
    pub fn expand_schema_defs_property_type_shorthands(
        schema_defs: &BTreeMap<String, Value>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Option<BTreeMap<String, Value>>, DataContractError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .expand_property_type_shorthands
        {
            None => Ok(None),
            Some(0) => v0::expand_schema_defs_property_type_shorthands_v0(
                schema_defs,
                full_validation,
                platform_version,
            ),
            Some(version) => Err(DataContractError::Unsupported(format!(
                "expand_property_type_shorthands version {version} is not supported"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use platform_value::platform_value;

    fn schema_with(properties: Value) -> Value {
        platform_value!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false
        })
    }

    fn expand(schema: &Value) -> Result<Option<Value>, DataContractError> {
        DocumentType::expand_property_type_shorthands(schema, true, PlatformVersion::latest())
    }

    fn refusal(schema: &Value) -> String {
        match expand(schema) {
            Err(DataContractError::InvalidContractStructure(message)) => message,
            other => panic!("expected InvalidContractStructure, got {other:?}"),
        }
    }

    #[test]
    fn should_expand_an_identifier_to_the_identifier_byte_array() {
        let expanded = expand(&schema_with(platform_value!({
            "recipientId": { "type": "identifier", "position": 0 }
        })))
        .expect("expected the shorthand to expand")
        .expect("expected a rewrite");

        let property = expanded
            .get_optional_value("properties")
            .ok()
            .flatten()
            .and_then(|properties| properties.get_optional_value("recipientId").ok().flatten())
            .expect("expected the property");
        assert_eq!(
            property.get_optional_str("type").ok().flatten(),
            Some("array")
        );
        assert_eq!(
            property.get_optional_bool("byteArray").ok().flatten(),
            Some(true)
        );
        assert_eq!(
            property
                .get_optional_integer::<u16>("minItems")
                .ok()
                .flatten(),
            Some(32)
        );
        assert_eq!(
            property
                .get_optional_integer::<u16>("maxItems")
                .ok()
                .flatten(),
            Some(32)
        );
        assert_eq!(
            property.get_optional_str("contentMediaType").ok().flatten(),
            Some("application/x.dash.dpp.identifier")
        );
        assert_eq!(
            property
                .get_optional_integer::<u32>("position")
                .ok()
                .flatten(),
            Some(0)
        );
    }

    #[test]
    fn should_expand_bytes_to_a_fixed_size_byte_array() {
        let expanded = expand(&schema_with(platform_value!({
            "txHash": { "type": "bytes", "size": 20, "position": 0 }
        })))
        .expect("expected the shorthand to expand")
        .expect("expected a rewrite");

        let property = expanded
            .get_optional_value("properties")
            .ok()
            .flatten()
            .and_then(|properties| properties.get_optional_value("txHash").ok().flatten())
            .expect("expected the property");
        assert_eq!(
            property.get_optional_str("type").ok().flatten(),
            Some("array")
        );
        assert_eq!(
            property
                .get_optional_integer::<u16>("minItems")
                .ok()
                .flatten(),
            Some(20)
        );
        assert_eq!(
            property
                .get_optional_integer::<u16>("maxItems")
                .ok()
                .flatten(),
            Some(20)
        );
        assert!(property.get_optional_value("size").ok().flatten().is_none());
        assert!(property
            .get_optional_value("contentMediaType")
            .ok()
            .flatten()
            .is_none());
    }

    #[test]
    fn should_return_none_when_nothing_is_a_shorthand() {
        let schema = schema_with(platform_value!({
            "name": { "type": "string", "maxLength": 10, "position": 0 },
            "size": { "type": "integer", "position": 1 }
        }));
        assert_matches!(expand(&schema), Ok(None));
    }

    #[test]
    fn should_leave_the_schema_as_sent_before_protocol_version_14() {
        let schema = schema_with(platform_value!({
            "recipientId": { "type": "identifier", "position": 0 }
        }));
        let pv13 = PlatformVersion::get(13).expect("expected protocol version 13");
        assert_matches!(
            DocumentType::expand_property_type_shorthands(&schema, true, pv13),
            Ok(None)
        );
        let defs = BTreeMap::from([("id".to_string(), platform_value!({ "type": "identifier" }))]);
        assert_matches!(
            DocumentType::expand_schema_defs_property_type_shorthands(&defs, true, pv13),
            Ok(None)
        );
    }

    #[test]
    fn should_expand_nested_members_typed_array_items_and_definitions() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "payment": {
                    "type": "object",
                    "properties": {
                        "to": { "type": "identifier", "position": 0 }
                    },
                    "additionalProperties": false,
                    "position": 0
                },
                "hashes": {
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "bytes", "size": 32 },
                    "position": 1
                }
            },
            "$defs": {
                "owner": { "type": "identifier" }
            },
            "additionalProperties": false
        });
        let expanded = expand(&schema)
            .expect("expected the shorthands to expand")
            .expect("expected a rewrite");
        let to = expanded
            .get_value_at_path("properties.payment.properties.to")
            .expect("expected the nested member");
        assert_eq!(to.get_optional_str("type").ok().flatten(), Some("array"));
        let items = expanded
            .get_value_at_path("properties.hashes.items")
            .expect("expected the items");
        assert_eq!(
            items.get_optional_integer::<u16>("maxItems").ok().flatten(),
            Some(32)
        );
        let owner = expanded
            .get_value_at_path("$defs.owner")
            .expect("expected the definition");
        assert_eq!(
            owner.get_optional_str("contentMediaType").ok().flatten(),
            Some("application/x.dash.dpp.identifier")
        );
        // The typed array itself keeps its own type and element count
        let hashes = expanded
            .get_value_at_path("properties.hashes")
            .expect("expected the array");
        assert_eq!(
            hashes
                .get_optional_integer::<u16>("maxItems")
                .ok()
                .flatten(),
            Some(4)
        );
    }

    #[test]
    fn should_refuse_a_long_form_keyword_beside_a_shorthand() {
        for keyword in ["byteArray", "minItems", "maxItems", "contentMediaType"] {
            let value = match keyword {
                "byteArray" => Value::Bool(true),
                "contentMediaType" => Value::Text("application/x.dash.dpp.identifier".into()),
                _ => Value::U64(32),
            };
            let mut identifier = platform_value!({ "type": "identifier", "position": 0 });
            identifier
                .insert(keyword.to_string(), value.clone())
                .expect("expected a map");
            let message = refusal(&schema_with(platform_value!({ "id": identifier })));
            assert!(
                message.contains(keyword) && message.contains("\"id\""),
                "{keyword}: {message}"
            );

            let mut bytes = platform_value!({ "type": "bytes", "size": 32, "position": 0 });
            bytes
                .insert(keyword.to_string(), value)
                .expect("expected a map");
            let message = refusal(&schema_with(platform_value!({ "hash": bytes })));
            assert!(
                message.contains(keyword) && message.contains("\"hash\""),
                "{keyword}: {message}"
            );
        }
    }

    #[test]
    fn should_refuse_size_on_an_identifier() {
        let message = refusal(&schema_with(platform_value!({
            "id": { "type": "identifier", "size": 32, "position": 0 }
        })));
        assert!(message.contains("size"), "{message}");
    }

    #[test]
    fn should_refuse_bytes_without_a_size() {
        let message = refusal(&schema_with(platform_value!({
            "hash": { "type": "bytes", "position": 0 }
        })));
        assert!(message.contains("size"), "{message}");
    }

    #[test]
    fn should_refuse_a_size_outside_one_to_the_largest_field_value() {
        let max_field_value_size = PlatformVersion::latest().system_limits.max_field_value_size;
        for size in [
            Value::U64(0),
            Value::I64(-1),
            Value::Float(4.0),
            Value::Text("4".into()),
            Value::U64(u64::from(max_field_value_size) + 1),
        ] {
            let mut bytes = platform_value!({ "type": "bytes", "position": 0 });
            bytes
                .insert("size".to_string(), size.clone())
                .expect("expected a map");
            let message = refusal(&schema_with(platform_value!({ "hash": bytes })));
            assert!(message.contains("size"), "{size:?}: {message}");
        }

        // The largest field value is the largest size
        let largest = schema_with(platform_value!({
            "hash": { "type": "bytes", "size": max_field_value_size, "position": 0 }
        }));
        assert_matches!(expand(&largest), Ok(Some(_)));
    }

    #[test]
    fn should_check_the_size_bound_under_full_validation_only() {
        let max_field_value_size = PlatformVersion::latest().system_limits.max_field_value_size;
        let schema = schema_with(platform_value!({
            "hash": { "type": "bytes", "size": max_field_value_size + 1, "position": 0 }
        }));
        assert_matches!(
            DocumentType::expand_property_type_shorthands(
                &schema,
                false,
                PlatformVersion::latest()
            ),
            Ok(Some(_))
        );
    }

    #[test]
    fn should_expand_a_definitions_map() {
        let defs = BTreeMap::from([
            (
                "owner".to_string(),
                platform_value!({ "type": "identifier" }),
            ),
            (
                "hash".to_string(),
                platform_value!({ "type": "bytes", "size": 32 }),
            ),
            ("note".to_string(), platform_value!({ "type": "string" })),
        ]);
        let expanded = DocumentType::expand_schema_defs_property_type_shorthands(
            &defs,
            true,
            PlatformVersion::latest(),
        )
        .expect("expected the definitions to expand")
        .expect("expected a rewrite");
        assert_eq!(
            expanded["owner"].get_optional_str("type").ok().flatten(),
            Some("array")
        );
        assert_eq!(
            expanded["hash"]
                .get_optional_integer::<u16>("minItems")
                .ok()
                .flatten(),
            Some(32)
        );
        assert_eq!(expanded["note"], defs["note"]);

        let untouched =
            BTreeMap::from([("note".to_string(), platform_value!({ "type": "string" }))]);
        assert_matches!(
            DocumentType::expand_schema_defs_property_type_shorthands(
                &untouched,
                true,
                PlatformVersion::latest()
            ),
            Ok(None)
        );
    }
}
