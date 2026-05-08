use platform_value::Value;

/// Top-level property names allowed to survive the v11→v12 migration that
/// transitions stored contracts into v1-document-meta-schema-conforming bytes.
///
/// This is intentionally a *subset* of v1 meta-schema's `properties`: keys
/// introduced for protocol v12 (e.g. `documentsCountable`, `rangeCountable`)
/// are accepted by the v1 meta-schema for new v12 contracts but must NOT
/// appear here, because pre-v12 contracts could not legitimately have set
/// them — leaving them in stored bytes would let the v2 parser reinterpret
/// a `NormalTree` as a count tree post-upgrade.
pub const ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES: &[&str] = &[
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
        ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES.contains(&key_str)
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

    #[test]
    fn transition_allowlist_is_subset_of_v1_meta_schema_properties() {
        // Every key the migration is willing to keep must be a key the v1
        // meta-schema actually accepts; otherwise the migration would leave
        // behind keys that v1 validation later rejects. The reverse direction
        // is intentionally NOT enforced — v1 has v12-introduced properties
        // (documentsCountable, rangeCountable) that pre-v12 contracts could
        // not legitimately set, and those are deliberately excluded from the
        // transition allowlist so they get stripped from stored bytes.
        let v1_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schema/meta_schemas/document/v1/document-meta.json"
        ))
        .expect("v1 document meta-schema JSON must be valid");

        let schema_properties: std::collections::BTreeSet<&str> = v1_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("v1 meta-schema must have a 'properties' object")
            .keys()
            .map(|k| k.as_str())
            .collect();

        let allowlist: std::collections::BTreeSet<&str> =
            ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES
                .iter()
                .copied()
                .collect();

        let in_allowlist_not_schema: Vec<&&str> =
            allowlist.difference(&schema_properties).collect();

        assert!(
            in_allowlist_not_schema.is_empty(),
            "Properties in ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES but not in v1 meta-schema: {:?}",
            in_allowlist_not_schema
        );
    }
}
