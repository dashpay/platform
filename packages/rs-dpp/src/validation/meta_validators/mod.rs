use super::byte_array_keyword::ByteArrayKeyword;
use jsonschema::{Draft, JSONSchema, RegexEngine, RegexOptions};
use lazy_static::lazy_static;
use serde_json::Value;

lazy_static! {
    static ref DRAFT202012: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/schema.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_CORE: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/meta/core.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_APPLICATOR: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/meta/applicator.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_UNEVALUATED: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/meta/unevaluated.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_VALIDATION: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/meta/validation.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_META_DATA: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/meta/meta-data.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_FORMAT_ANNOTATION: serde_json::Value = serde_json::from_str(
        include_str!("../../../schema/meta_schemas/draft2020-12/meta/format-annotation.json")
    )
    .expect("Valid schema!");
    static ref DRAFT202012_CONTENT: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schema/meta_schemas/draft2020-12/meta/content.json"
    ))
    .expect("Valid schema!");
    static ref DOCUMENT_META_JSON_V0: Value = serde_json::from_str::<Value>(include_str!(
        "../../../schema/meta_schemas/document/v0/document-meta.json"
    ))
    .expect("v0 document meta-schema JSON must be valid");
    static ref DOCUMENT_META_JSON_V1: Value = serde_json::from_str::<Value>(include_str!(
        "../../../schema/meta_schemas/document/v1/document-meta.json"
    ))
    .expect("v1 document meta-schema JSON must be valid");
    static ref DOCUMENT_META_JSON_V2: Value = serde_json::from_str::<Value>(include_str!(
        "../../../schema/meta_schemas/document/v2/document-meta.json"
    ))
    .expect("v1 document meta-schema JSON must be valid");
    static ref DOCUMENT_META_JSON_V3: Value = serde_json::from_str::<Value>(include_str!(
        "../../../schema/meta_schemas/document/v3/document-meta.json"
    ))
    .expect("v3 document meta-schema JSON must be valid");

    pub static ref DRAFT_202012_META_SCHEMA: JSONSchema = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .should_ignore_unknown_formats(false)
        .should_validate_formats(true)
        .with_patterns_regex_engine(RegexEngine::Regex(RegexOptions {
            size_limit: Some(5 * (1 << 20)),
            ..Default::default()
        }))
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/core".to_string(),
            DRAFT202012_CORE.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/unevaluated".to_string(),
            DRAFT202012_UNEVALUATED.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/validation".to_string(),
            DRAFT202012_VALIDATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/meta-data".to_string(),
            DRAFT202012_META_DATA.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/format-annotation".to_string(),
            DRAFT202012_FORMAT_ANNOTATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/content".to_string(),
            DRAFT202012_CONTENT.clone(),
        )
        .to_owned()
        .compile(&DRAFT202012)
        .expect("Invalid data contract schema");

    // Compiled version of document meta schema
    pub static ref DOCUMENT_META_SCHEMA_V0: JSONSchema = JSONSchema::options()
        .with_keyword(
            "byteArray",
            |_, _, _| Ok(Box::new(ByteArrayKeyword)),
        )
        .with_patterns_regex_engine(RegexEngine::Regex(RegexOptions {
            size_limit: Some(5 * (1 << 20)),
            ..Default::default()
        }))
        .should_ignore_unknown_formats(false)
        .should_validate_formats(true)
        .with_patterns_regex_engine(RegexEngine::Regex(Default::default()))
        .with_draft(Draft::Draft202012)
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/core".to_string(),
            DRAFT202012_CORE.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/unevaluated".to_string(),
            DRAFT202012_UNEVALUATED.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/validation".to_string(),
            DRAFT202012_VALIDATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/meta-data".to_string(),
            DRAFT202012_META_DATA.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/format-annotation".to_string(),
            DRAFT202012_FORMAT_ANNOTATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/content".to_string(),
            DRAFT202012_CONTENT.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/schema".to_string(),
            DRAFT202012.clone(),
        )
        .to_owned()
        .compile(&DOCUMENT_META_JSON_V0)
        .expect("Invalid data contract schema");

    // Compiled version of document meta schema v1
    // This version adds additionalProperties: false at the top level
    pub static ref DOCUMENT_META_SCHEMA_V1: JSONSchema = JSONSchema::options()
        .with_keyword(
            "byteArray",
            |_, _, _| Ok(Box::new(ByteArrayKeyword)),
        )
        .with_patterns_regex_engine(RegexEngine::Regex(RegexOptions {
            size_limit: Some(5 * (1 << 20)),
            ..Default::default()
        }))
        .should_ignore_unknown_formats(false)
        .should_validate_formats(true)
        .with_draft(Draft::Draft202012)
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/core".to_string(),
            DRAFT202012_CORE.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/unevaluated".to_string(),
            DRAFT202012_UNEVALUATED.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/validation".to_string(),
            DRAFT202012_VALIDATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/meta-data".to_string(),
            DRAFT202012_META_DATA.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/format-annotation".to_string(),
            DRAFT202012_FORMAT_ANNOTATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/content".to_string(),
            DRAFT202012_CONTENT.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/schema".to_string(),
            DRAFT202012.clone(),
        )
        .to_owned()
        .compile(&DOCUMENT_META_JSON_V1)
        .expect("Invalid data contract schema");

    // Compiled version of document meta schema v2
    // This version adds the keepsTransferHistory, keepsPurchaseHistory and
    // keepsPricingHistory document type configuration flags (protocol version 13)
    pub static ref DOCUMENT_META_SCHEMA_V2: JSONSchema = JSONSchema::options()
        .with_keyword(
            "byteArray",
            |_, _, _| Ok(Box::new(ByteArrayKeyword)),
        )
        .with_patterns_regex_engine(RegexEngine::Regex(RegexOptions {
            size_limit: Some(5 * (1 << 20)),
            ..Default::default()
        }))
        .should_ignore_unknown_formats(false)
        .should_validate_formats(true)
        .with_draft(Draft::Draft202012)
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/core".to_string(),
            DRAFT202012_CORE.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/unevaluated".to_string(),
            DRAFT202012_UNEVALUATED.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/validation".to_string(),
            DRAFT202012_VALIDATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/meta-data".to_string(),
            DRAFT202012_META_DATA.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/format-annotation".to_string(),
            DRAFT202012_FORMAT_ANNOTATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/content".to_string(),
            DRAFT202012_CONTENT.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/schema".to_string(),
            DRAFT202012.clone(),
        )
        .to_owned()
        .compile(&DOCUMENT_META_JSON_V2)
        .expect("Invalid data contract schema");

    // Compiled version of document meta schema v3
    // Introduced for protocol version 14 (contract-level ranked aggregates).
    // v2 plus the three index-level ranking keywords — `rankedCountable`,
    // `rankedSummable`, `rankedAverageable` — and the `dependentRequired`
    // rows tying each to its range axis. Hosting them on a schema only v14+
    // contracts validate against leaves v13 validation untouched: under v2
    // the keys still fail `additionalProperties: false` on an index entry.
    pub static ref DOCUMENT_META_SCHEMA_V3: JSONSchema = JSONSchema::options()
        .with_keyword(
            "byteArray",
            |_, _, _| Ok(Box::new(ByteArrayKeyword)),
        )
        .with_patterns_regex_engine(RegexEngine::Regex(RegexOptions {
            size_limit: Some(5 * (1 << 20)),
            ..Default::default()
        }))
        .should_ignore_unknown_formats(false)
        .should_validate_formats(true)
        .with_draft(Draft::Draft202012)
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/core".to_string(),
            DRAFT202012_CORE.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/unevaluated".to_string(),
            DRAFT202012_UNEVALUATED.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/validation".to_string(),
            DRAFT202012_VALIDATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/meta-data".to_string(),
            DRAFT202012_META_DATA.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/format-annotation".to_string(),
            DRAFT202012_FORMAT_ANNOTATION.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/meta/content".to_string(),
            DRAFT202012_CONTENT.clone(),
        )
        .with_document(
            "https://json-schema.org/draft/2020-12/schema".to_string(),
            DRAFT202012.clone(),
        )
        .to_owned()
        .compile(&DOCUMENT_META_JSON_V3)
        .expect("Invalid data contract schema");

}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document_schema_with_refers_to(refers_to: serde_json::Value) -> serde_json::Value {
        json!({
            "$schema": "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/document/v1/document-meta.json",
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": refers_to
                }
            },
            "additionalProperties": false
        })
    }

    #[test]
    fn should_accept_refers_to_in_v3_document_schema() {
        for target in ["identity", "contract", "token"] {
            let schema = document_schema_with_refers_to(json!({
                "type": target
            }));

            assert!(
                DOCUMENT_META_SCHEMA_V3.validate(&schema).is_ok(),
                "expected schema with {target} target to be valid"
            );
        }
    }

    #[test]
    fn should_accept_permanent_document_refers_to_in_v3_document_schema() {
        let schema = document_schema_with_refers_to(json!({
            "type": "permanentDocument",
            "contractId": "4Bqs6itzfoDXzmgQibYZQABbqYsXmawVf7SKe3mKDQVd",
            "documentType": "note"
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_ok(),
            "expected permanentDocument refersTo to be valid"
        );
    }

    #[test]
    fn should_accept_permanent_document_refers_to_with_byte_array_contract_id() {
        let schema = document_schema_with_refers_to(json!({
            "type": "permanentDocument",
            "contractId": vec![7u8; 32],
            "documentType": "note"
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_ok(),
            "expected a byte-array contractId to be valid"
        );
    }

    #[test]
    fn should_reject_permanent_document_refers_to_without_contract_id() {
        let schema = document_schema_with_refers_to(json!({
            "type": "permanentDocument",
            "documentType": "note"
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
            "expected permanentDocument refersTo without contractId to be invalid"
        );
    }

    #[test]
    fn should_reject_permanent_document_refers_to_without_document_type() {
        let schema = document_schema_with_refers_to(json!({
            "type": "permanentDocument",
            "contractId": "4Bqs6itzfoDXzmgQibYZQABbqYsXmawVf7SKe3mKDQVd"
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
            "expected permanentDocument refersTo without documentType to be invalid"
        );
    }

    #[test]
    fn should_reject_contract_id_on_non_document_refers_to_targets() {
        for target in ["identity", "contract", "token"] {
            let schema = document_schema_with_refers_to(json!({
                "type": target,
                "contractId": "4Bqs6itzfoDXzmgQibYZQABbqYsXmawVf7SKe3mKDQVd"
            }));

            assert!(
                DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
                "expected contractId on a {target} target to be invalid"
            );
        }
    }

    #[test]
    fn should_reject_permanent_document_refers_to_with_invalid_contract_id() {
        for bad in [json!("not-base58-0OIl"), json!(vec![7u8; 31]), json!(42)] {
            let schema = document_schema_with_refers_to(json!({
                "type": "permanentDocument",
                "contractId": bad,
                "documentType": "note"
            }));

            assert!(
                DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
                "expected invalid contractId {bad} to be rejected"
            );
        }
    }

    #[test]
    fn should_reject_refers_to_with_unknown_properties() {
        let schema = document_schema_with_refers_to(json!({
            "type": "identity",
            "mustExist": false
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
            "expected unknown refersTo properties to be rejected"
        );
    }

    #[test]
    fn should_reject_refers_to_with_unknown_type() {
        let schema = document_schema_with_refers_to(json!({
            "type": "unknown"
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
            "expected schema to be invalid"
        );
    }

    #[test]
    fn should_reject_refers_to_on_non_identifier_property() {
        let schema = json!({
            "$schema": "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/document/v1/document-meta.json",
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                    "refersTo": { "type": "identity" }
                }
            },
            "additionalProperties": false
        });

        assert!(
            DOCUMENT_META_SCHEMA_V3.validate(&schema).is_err(),
            "expected refersTo on a non-identifier property to be invalid"
        );
    }

    #[test]
    fn should_reject_refers_to_in_v2_document_schema() {
        let schema = document_schema_with_refers_to(json!({
            "type": "identity"
        }));

        assert!(
            DOCUMENT_META_SCHEMA_V2.validate(&schema).is_err(),
            "expected refersTo to be rejected by the v2 meta schema"
        );
    }
}
