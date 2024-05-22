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
    static ref DATA_CONTRACT_V0: Value = serde_json::from_str::<Value>(include_str!(
        "../../../schema/meta_schemas/document/v0/document-meta.json"
    ))
    .unwrap();

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
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
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


    // Compiled version of data contract meta schema
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
            "https://json-schema.org/draft/2020-12/meta/applicator".to_string(),
            DRAFT202012_APPLICATOR.clone(),
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
        .compile(&DATA_CONTRACT_V0)
        .expect("Invalid data contract schema");
}
