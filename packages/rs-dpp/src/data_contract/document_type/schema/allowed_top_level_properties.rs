use platform_value::Value;

/// Top-level property names introduced in protocol v12. Pre-v12 contracts
/// could not legitimately use these keys, but raw schema bytes may contain
/// them (the v0/v1 meta-schemas accept arbitrary booleans for these names).
/// The v11→v12 migration must strip these from stored contracts so the v2
/// parser does not revive them and reinterpret a `NormalTree` as a
/// `CountTree` / `ProvableCountTree`.
pub const V12_INTRODUCED_DOCUMENT_SCHEMA_TOP_LEVEL_PROPERTIES: &[&str] =
    &["documentsCountable", "rangeCountable"];

/// The set of top-level property names allowed on a document type schema object
/// as defined by the v1 document meta-schema.
///
/// Used to validate new v12+ contracts. The v11→v12 migration uses the stricter
/// [`ALLOWED_DOCUMENT_SCHEMA_PRE_V12_PROPERTIES`] list so smuggled v12-introduced
/// flags are stripped from pre-v12 contracts.
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
    "documentsCountable",
    "rangeCountable",
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

/// Top-level property names that pre-v12 contracts were permitted to declare.
/// Equal to [`ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES`] minus
/// [`V12_INTRODUCED_DOCUMENT_SCHEMA_TOP_LEVEL_PROPERTIES`]. Used by the v11→v12
/// migration so smuggled v12-introduced flags are stripped from stored pre-v12
/// contracts before the v2 parser ever sees them.
pub const ALLOWED_DOCUMENT_SCHEMA_PRE_V12_PROPERTIES: &[&str] = &[
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
/// is not in `allowed`. Returns `true` if any keys were removed.
fn strip_to_allowlist(schema: &mut Value, allowed: &[&str]) -> bool {
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
        allowed.contains(&key_str)
    });
    map.len() != before
}

/// Strips any top-level key from the document type schema `Value::Map` that
/// is not in [`ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES`]. Returns `true` if any
/// keys were removed.
pub fn strip_unknown_properties_from_document_schema(schema: &mut Value) -> bool {
    strip_to_allowlist(schema, ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES)
}

/// Strips any top-level key from the document type schema `Value::Map` that
/// is not in [`ALLOWED_DOCUMENT_SCHEMA_PRE_V12_PROPERTIES`]. This is the strip
/// invoked by the v11→v12 protocol upgrade migration so smuggled
/// v12-introduced flags are removed from stored pre-v12 contracts. Returns
/// `true` if any keys were removed.
pub fn strip_unknown_properties_from_pre_v12_document_schema(schema: &mut Value) -> bool {
    strip_to_allowlist(schema, ALLOWED_DOCUMENT_SCHEMA_PRE_V12_PROPERTIES)
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
    fn pre_v12_strip_removes_smuggled_v12_flags() {
        // A pre-v12 contract that smuggled v12-introduced flags into its raw
        // schema would, if left untouched at v12 upgrade, be reinterpreted by
        // the v2 parser as countable / range-countable — a storage-layout
        // mismatch with the underlying NormalTree. The migration must strip
        // these flags.
        let mut schema = platform_value!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
            "documentsCountable": true,
            "rangeCountable": true,
        });

        let changed = strip_unknown_properties_from_pre_v12_document_schema(&mut schema);
        assert!(changed, "expected the strip to report changes");

        let map = schema.as_map().unwrap();
        let keys: Vec<&str> = map.iter().filter_map(|(k, _)| k.as_text()).collect();
        assert!(!keys.contains(&"documentsCountable"));
        assert!(!keys.contains(&"rangeCountable"));
        assert!(keys.contains(&"type"));
        assert!(keys.contains(&"properties"));
        assert!(keys.contains(&"additionalProperties"));
    }

    #[test]
    fn pre_v12_strip_keeps_v12_flags_under_v1_strip() {
        // The non-pre-v12 strip is for new v12 contracts: documentsCountable /
        // rangeCountable must survive there.
        let mut schema = platform_value!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
            "documentsCountable": true,
            "rangeCountable": true,
        });

        let changed = strip_unknown_properties_from_document_schema(&mut schema);
        assert!(!changed);
    }

    #[test]
    fn pre_v12_allowlist_is_v1_minus_v12_introduced() {
        let v1: std::collections::BTreeSet<&str> = ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES
            .iter()
            .copied()
            .collect();
        let pre_v12: std::collections::BTreeSet<&str> = ALLOWED_DOCUMENT_SCHEMA_PRE_V12_PROPERTIES
            .iter()
            .copied()
            .collect();
        let v12_introduced: std::collections::BTreeSet<&str> =
            V12_INTRODUCED_DOCUMENT_SCHEMA_TOP_LEVEL_PROPERTIES
                .iter()
                .copied()
                .collect();

        // pre_v12 ∪ v12_introduced == v1
        let union: std::collections::BTreeSet<&str> =
            pre_v12.union(&v12_introduced).copied().collect();
        assert_eq!(
            union, v1,
            "pre-v12 allowlist + v12-introduced should equal the v1 allowlist"
        );

        // pre_v12 ∩ v12_introduced == ∅
        let intersection: std::collections::BTreeSet<&str> =
            pre_v12.intersection(&v12_introduced).copied().collect();
        assert!(
            intersection.is_empty(),
            "pre-v12 allowlist must not contain v12-introduced properties: {:?}",
            intersection
        );
    }

    #[test]
    fn allowlist_matches_v1_meta_schema_properties() {
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

        let allowlist: std::collections::BTreeSet<&str> = ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES
            .iter()
            .copied()
            .collect();

        let in_allowlist_not_schema: Vec<&&str> =
            allowlist.difference(&schema_properties).collect();
        let in_schema_not_allowlist: Vec<&&str> =
            schema_properties.difference(&allowlist).collect();

        assert!(
            in_allowlist_not_schema.is_empty(),
            "Properties in ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES but not in v1 meta-schema: {:?}",
            in_allowlist_not_schema
        );
        assert!(
            in_schema_not_allowlist.is_empty(),
            "Properties in v1 meta-schema but not in ALLOWED_DOCUMENT_SCHEMA_V1_PROPERTIES: {:?}",
            in_schema_not_allowlist
        );
    }
}
