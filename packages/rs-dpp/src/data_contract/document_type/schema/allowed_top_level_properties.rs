use platform_value::Value;

/// Top-level property names allowed to survive the v11→v12 migration that
/// transitions stored contracts into v1-document-meta-schema-conforming bytes.
///
/// This is intentionally a *subset* of v1 meta-schema's `properties`: keys
/// introduced for protocol v12 (`documentsCountable`, `rangeCountable`) are
/// accepted by v1 for new v12 contracts but must NOT appear here, because
/// pre-v12 contracts could not legitimately have set them — leaving them
/// in stored bytes would let the v2 parser reinterpret a `NormalTree` as a
/// count tree post-upgrade.
///
/// Standard JSON-Schema keywords v1 also adds at the top level (`required`,
/// `$comment`, `description`, `minProperties`, `maxProperties`,
/// `dependentRequired`) are kept: they are either pure documentation or
/// describe the document instance shape, not the storage layout, so
/// preserving them across the upgrade does not change consensus-critical
/// semantics.
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
    fn strips_keywords_from_document_schema() {
        // `keywords` was erroneously placed on the document-type meta schema
        // by PR #2523 — the intended location is contract-level
        // (`DataContractV1.keywords`). This test guards the v12 migration
        // path that removes any `keywords` key that slipped onto a
        // document-type schema in stored state.
        let mut schema = platform_value!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
            "keywords": ["one", "two"]
        });

        let changed = strip_unknown_properties_from_document_schema(&mut schema);
        assert!(changed);

        let map = schema.as_map().unwrap();
        let keys: Vec<&str> = map.iter().filter_map(|(k, _)| k.as_text()).collect();
        assert!(!keys.contains(&"keywords"));
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

    /// JSON-Schema-standard keywords that v1 meta-schema introduced at the
    /// top level (over v0) and that the v11→v12 transition deliberately
    /// preserves. They are either pure documentation (`$comment`,
    /// `description`) or describe instance shape (`required`,
    /// `minProperties`, `maxProperties`, `dependentRequired`) — none of
    /// them affect storage layout, so keeping them across the upgrade does
    /// not change consensus-critical semantics.
    ///
    /// Adding any other top-level property to v1 will fail
    /// [`transition_allowlist_equals_v0_plus_known_v1_additions`] and
    /// require an explicit decision: include it here (preserve across
    /// transition) or leave it out (strip from pre-v12 bytes).
    const STANDARD_V1_JSON_SCHEMA_KEYWORDS_KEPT_IN_TRANSITION: &[&str] = &[
        "required",
        "$comment",
        "description",
        "minProperties",
        "maxProperties",
        "dependentRequired",
    ];

    #[test]
    fn transition_allowlist_equals_v0_plus_known_v1_additions() {
        // The migration cleans pre-v12 stored bytes. Pre-v12 contracts were
        // validated against the v0 document meta-schema, so the allowlist
        // must contain exactly v0's top-level properties plus a deliberately
        // chosen set of v1-introduced JSON-Schema-standard keywords. Any
        // other v1 addition (e.g. `documentsCountable` / `rangeCountable`)
        // is excluded so the v2 parser cannot revive it on pre-v12 contracts
        // and reinterpret a `NormalTree` as a count tree.
        //
        // `keywords` is a special case: it was erroneously placed on the v0
        // document-type meta-schema by PR #2523 (its intended home is
        // contract-level, `DataContractV1.keywords`). The v11→v12 migration
        // deliberately strips it from stored bytes — see
        // `strips_keywords_from_document_schema` — so it is excluded from
        // the expected allowlist here even though it appears in v0.
        let v0_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schema/meta_schemas/document/v0/document-meta.json"
        ))
        .expect("v0 document meta-schema JSON must be valid");

        let v0_properties: std::collections::BTreeSet<&str> = v0_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("v0 meta-schema must have a 'properties' object")
            .keys()
            .map(|k| k.as_str())
            .filter(|k| *k != "keywords")
            .collect();

        let allowlist: std::collections::BTreeSet<&str> =
            ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES
                .iter()
                .copied()
                .collect();

        let known_v1_additions: std::collections::BTreeSet<&str> =
            STANDARD_V1_JSON_SCHEMA_KEYWORDS_KEPT_IN_TRANSITION
                .iter()
                .copied()
                .collect();

        let expected: std::collections::BTreeSet<&str> =
            v0_properties.union(&known_v1_additions).copied().collect();

        assert_eq!(
            allowlist, expected,
            "ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES must equal \
             v0 meta-schema properties ∪ STANDARD_V1_JSON_SCHEMA_KEYWORDS_KEPT_IN_TRANSITION",
        );
    }

    #[test]
    fn known_v1_additions_are_actually_in_v1_but_not_v0() {
        // Sanity check: every entry in the known-v1-additions list must
        // genuinely be a v1 addition (in v1, not in v0). Otherwise the
        // constant is misnamed and the design intent has drifted.
        let v0_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schema/meta_schemas/document/v0/document-meta.json"
        ))
        .expect("v0 document meta-schema JSON must be valid");
        let v1_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schema/meta_schemas/document/v1/document-meta.json"
        ))
        .expect("v1 document meta-schema JSON must be valid");

        let v0_props: std::collections::BTreeSet<&str> = v0_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        let v1_props: std::collections::BTreeSet<&str> = v1_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();

        for &k in STANDARD_V1_JSON_SCHEMA_KEYWORDS_KEPT_IN_TRANSITION {
            assert!(
                v1_props.contains(k),
                "{k} is in STANDARD_V1_JSON_SCHEMA_KEYWORDS_KEPT_IN_TRANSITION but not in v1 meta-schema"
            );
            assert!(
                !v0_props.contains(k),
                "{k} is in STANDARD_V1_JSON_SCHEMA_KEYWORDS_KEPT_IN_TRANSITION but already in v0 meta-schema (so not a v1 addition)"
            );
        }
    }
}
