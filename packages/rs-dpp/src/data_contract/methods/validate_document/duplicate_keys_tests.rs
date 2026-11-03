use super::DataContractDocumentValidationMethodsV0;
use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::document::{Document, DocumentV0};
use crate::prelude::DataContract;
use crate::tests::fixtures::get_data_contract_fixture;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::{platform_value, Value};
use platform_version::version::PlatformVersion;

const REPEATED_KEY: &str = "document properties contain a repeated map key";

fn contract(platform_version: &PlatformVersion, constraints: bool) -> DataContract {
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "meta": {
                "type": "object", "position": 0,
                "properties": {
                    "name": {"type": "string", "maxLength": 64, "position": 0},
                    "score": {"type": "integer", "minimum": 0, "position": 1}
                },
                "required": ["name", "score"], "additionalProperties": false
            },
            "tags": {
                "type": "array", "position": 1, "maxItems": 4,
                "items": {"type": "string", "maxLength": 32}
            }
        },
        "required": ["meta"], "additionalProperties": false
    });
    if !constraints {
        schema
            .get_mut("properties")
            .unwrap()
            .unwrap()
            .remove("tags")
            .unwrap();
    }
    if constraints {
        schema
            .get_mut("properties")
            .unwrap()
            .unwrap()
            .get_mut("meta")
            .unwrap()
            .unwrap()
            .get_mut("properties")
            .unwrap()
            .unwrap()
            .get_mut("name")
            .unwrap()
            .unwrap()
            .insert("maxBytes".into(), 8u32.into())
            .unwrap();
        schema
            .insert(
                "propertyConstraints".into(),
                platform_value!({
                    "scoreCapped": {"lessThanOrEqual": ["meta.score", 10]}
                }),
            )
            .unwrap();
    }
    contract
        .set_document_schema("nested", schema, true, &mut Vec::new(), platform_version)
        .expect("schema should parse");
    contract
}

fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(
        entries
            .iter()
            .map(|(key, value)| (Value::Text((*key).into()), value.clone()))
            .collect(),
    )
}

fn properties(meta: Value) -> Value {
    map(&[("meta", meta)])
}

fn validate(
    contract: &DataContract,
    value: Value,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    contract
        .validate_document_properties(
            "nested",
            value,
            &DocumentSystemValues::default(),
            platform_version,
        )
        .expect("validation should return a consensus result")
}

fn assert_repeated(result: &SimpleConsensusValidationResult) {
    let error = result
        .first_error()
        .expect("ambiguous nested properties must be rejected");
    assert_eq!(error.code(), 10103);
    assert_eq!(error.to_string(), REPEATED_KEY);
}

#[test]
fn should_reject_nested_duplicates_that_bypass_max_bytes() {
    let version = PlatformVersion::latest();
    let contract = contract(version, true);
    let last = properties(map(&[("name", "123456789".into()), ("score", 1u32.into())]));
    assert!(matches!(
        validate(&contract, last, version).first_error(),
        Some(ConsensusError::BasicError(
            BasicError::DocumentPropertyMaxBytesExceededError(_)
        ))
    ));
    let ambiguous = properties(map(&[
        ("name", "ok".into()),
        ("name", "123456789".into()),
        ("score", 1u32.into()),
    ]));
    assert_repeated(&validate(&contract, ambiguous, version));
}

#[test]
fn should_reject_nested_duplicates_that_bypass_property_constraints() {
    let version = PlatformVersion::latest();
    let contract = contract(version, true);
    let last = properties(map(&[("name", "ok".into()), ("score", 11u32.into())]));
    assert!(matches!(
        validate(&contract, last, version).first_error(),
        Some(ConsensusError::BasicError(
            BasicError::DocumentPropertyConstraintViolatedError(_)
        ))
    ));
    let ambiguous = properties(map(&[
        ("name", "ok".into()),
        ("score", 1u32.into()),
        ("score", 11u32.into()),
    ]));
    assert_repeated(&validate(&contract, ambiguous, version));
}

#[test]
fn should_reject_equal_duplicates_and_duplicate_parents_through_both_dispatchers() {
    let version = PlatformVersion::latest();
    let contract = contract(version, true);
    for meta in [
        map(&[
            ("name", "ok".into()),
            ("name", "ok".into()),
            ("score", 1u32.into()),
        ]),
        map(&[
            ("name", "ok".into()),
            ("score", 1u32.into()),
            ("score", 1u32.into()),
        ]),
    ] {
        let value = properties(meta);
        assert_repeated(&validate(&contract, value.clone(), version));
        let document = Document::V0(DocumentV0 {
            properties: value.into_btree_string_map().unwrap(),
            ..Default::default()
        });
        assert_repeated(
            &contract
                .validate_document("nested", &document, version)
                .unwrap(),
        );
    }
    let meta = map(&[("name", "ok".into()), ("score", 1u32.into())]);
    assert_repeated(&validate(
        &contract,
        map(&[("meta", meta.clone()), ("meta", meta)]),
        version,
    ));
}

#[test]
fn should_report_repeated_keys_before_array_schema_and_field_size_errors() {
    let version = PlatformVersion::latest();
    let contract = contract(version, true);
    let repeated = map(&[("name", "ok".into()), ("name", "ok".into())]);
    let mut value = properties(map(&[("name", "ok".into()), ("score", 1u32.into())]));
    value
        .insert("tags".into(), Value::Array(vec![repeated]))
        .unwrap();
    assert_repeated(&validate(&contract, value, version));
    let oversized = "x".repeat(version.system_limits.max_field_value_size as usize + 1);
    assert_repeated(&validate(
        &contract,
        properties(map(&[
            ("name", "ok".into()),
            ("name", oversized.into()),
            ("score", 1u32.into()),
        ])),
        version,
    ));
}

#[test]
fn should_preserve_invalid_type_depth_and_non_text_key_errors() {
    let version = PlatformVersion::latest();
    let contract = contract(version, true);
    let duplicate = map(&[("name", "ok".into()), ("name", "ok".into())]);
    let result = contract
        .validate_document_properties(
            "absent",
            duplicate.clone(),
            &DocumentSystemValues::default(),
            version,
        )
        .unwrap();
    assert!(matches!(
        result.first_error(),
        Some(ConsensusError::BasicError(
            BasicError::InvalidDocumentTypeError(_)
        ))
    ));
    let depth = version.system_limits.max_document_value_depth.unwrap() as usize;
    let too_deep = (0..depth).fold(duplicate, |value, _| Value::Array(vec![value]));
    let result = validate(&contract, properties(too_deep), version);
    assert_eq!(
        result.first_error().unwrap().to_string(),
        format!(
            "document value depth {} exceeds system maximum {depth}",
            depth + 1
        )
    );
    let non_text = Value::Map(vec![
        (Value::U32(1), "a".into()),
        (Value::U32(1), "b".into()),
    ]);
    for value in [
        properties(non_text.clone()),
        properties(Value::Array(vec![non_text])),
    ] {
        let result = validate(&contract, value, version);
        assert_eq!(result.first_error().unwrap().code(), 10103);
        assert_ne!(result.first_error().unwrap().to_string(), REPEATED_KEY);
    }
}

#[test]
fn should_allow_latest_document_depth_at_the_limit_before_field_size_validation() {
    let version = PlatformVersion::latest();
    let contract = contract(version, true);
    let depth = version.system_limits.max_document_value_depth.unwrap() as usize;
    let leaf = Value::Text("x".repeat(version.system_limits.max_field_value_size as usize + 1));
    let nested = (0..depth).fold(leaf, |value, index| {
        if index % 2 == 0 {
            Value::Array(vec![value])
        } else {
            map(&[("child", value)])
        }
    });
    let result = validate(&contract, properties(nested), version);
    assert!(matches!(
        result.first_error(),
        Some(ConsensusError::BasicError(
            BasicError::DocumentFieldMaxSizeExceededError(_)
        ))
    ));
}

#[test]
fn should_keep_distinct_nested_keys_valid_and_prior_version_duplicates_unchanged() {
    let version = PlatformVersion::latest();
    let latest_contract = contract(version, true);
    let clean = properties(map(&[("name", "ok".into()), ("score", 1u32.into())]));
    assert!(validate(&latest_contract, clean, version).is_valid());
    let old = PlatformVersion::get(13).unwrap();
    let old_contract = contract(old, false);
    let ambiguous = properties(map(&[
        ("name", "first".into()),
        ("name", "last".into()),
        ("score", 1u32.into()),
    ]));
    assert!(validate(&old_contract, ambiguous, old).is_valid());
}
