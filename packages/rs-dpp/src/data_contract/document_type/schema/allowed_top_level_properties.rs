use platform_value::Value;

/// The set of top-level property names allowed on a document type schema object
/// as defined by the v1 document meta-schema.
///
/// Any key not in this list should be stripped from document type schemas
/// during the v12 protocol upgrade migration to prevent unknown properties
/// from changing storage semantics.
pub const ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES: &[&str] = &[
    "type",
    "$schema",
    "$defs",
    "indices",
    "signatureSecurityLevelRequirement",
    "documentsKeepHistory",
    "documentsMutable",
    "canBeDeleted",
    "transferable",
    "tradeMode",
    "creationRestrictionMode",
    "requiresIdentityEncryptionBoundedKey",
    "requiresIdentityDecryptionBoundedKey",
    "tokenCost",
    "properties",
    "transient",
    "keywords",
    "additionalProperties",
    "required",
    "$comment",
    "description",
    "minProperties",
    "maxProperties",
    "dependentRequired",
];

/// Strips any top-level key from the document type schema `Value::Map` that
/// is not in the allowed set. Returns `true` if any keys were removed.
pub fn strip_unknown_properties_from_document_schema(schema: &mut Value) -> bool {
    let map = match schema {
        Value::Map(map) => map,
        _ => return false,
    };

    let before = map.len();
    map.retain(|(key, _)| {
        let key_str = match key {
            Value::Text(s) => s.as_str(),
            _ => return true, // keep non-string keys (shouldn't happen but safe)
        };
        ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES.contains(&key_str)
    });
    map.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    #[test]
    fn strips_unknown_properties() {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
            "unknownProp": true,
            "anotherUnknown": 42
        });

        let changed = strip_unknown_properties_from_document_schema(&mut schema);
        assert!(changed);

        let map = schema.as_map().unwrap();
        let keys: Vec<&str> = map.iter().filter_map(|(k, _)| k.as_text()).collect();
        assert!(!keys.contains(&"unknownProp"));
        assert!(!keys.contains(&"anotherUnknown"));
        assert!(keys.contains(&"type"));
        assert!(keys.contains(&"properties"));
        assert!(keys.contains(&"additionalProperties"));
    }

    #[test]
    fn no_change_when_all_properties_are_known() {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
            "required": ["foo"],
            "$comment": "test"
        });

        let changed = strip_unknown_properties_from_document_schema(&mut schema);
        assert!(!changed);
    }

    #[test]
    fn handles_non_map_value() {
        let mut schema = Value::Text("not a map".to_string());
        let changed = strip_unknown_properties_from_document_schema(&mut schema);
        assert!(!changed);
    }
}
