use crate::{
    codes::ErrorWithCode,
    consensus::{basic::JsonSchemaError, ConsensusError},
    data_contract::validation::data_contract_validator::DataContractValidator,
    errors::consensus::basic::{BasicError, IndexError},
    prelude::*,
    tests::fixtures::get_data_contract_fixture,
    util::json_value::JsonValueExt,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
    Convertible,
};
use jsonschema::error::ValidationErrorKind;
use log::trace;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use test_case::test_case;

struct TestData {
    data_contract_validator: DataContractValidator,
    data_contract: DataContract,
    raw_data_contract: JsonValue,
}

fn setup_test() -> TestData {
    init();

    let data_contract = get_data_contract_fixture(None);
    let raw_data_contract = data_contract.to_object(false).unwrap();

    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());

    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));

    TestData {
        data_contract,
        raw_data_contract,
        data_contract_validator,
    }
}

fn init() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .try_init();
}

fn get_schema_error(result: &ValidationResult, number: usize) -> &JsonSchemaError {
    result
        .errors
        .get(number)
        .expect("the error should be returned in validation result")
        .json_schema_error()
        .expect("the error should be json schema error")
}

fn get_basic_error(consensus_error: &ConsensusError) -> &BasicError {
    match consensus_error {
        ConsensusError::BasicError(basic_error) => &**basic_error,
        _ => panic!("error '{:?}' isn't a basic error", consensus_error),
    }
}

fn get_index_error(consensus_error: &ConsensusError) -> &IndexError {
    match consensus_error {
        ConsensusError::BasicError(basic_error) => match &**basic_error {
            BasicError::IndexError(index_error) => index_error,
            _ => panic!("error '{:?}' isn't a index error", consensus_error),
        },
        _ => panic!("error '{:?}' isn't a basic error", consensus_error),
    }
}

fn print_json_schema_errors(result: &ValidationResult) {
    for (i, e) in result.errors.iter().enumerate() {
        let schema_error = e.json_schema_error().unwrap();
        println!(
            "error_{}:  {:>30} -({:>20?}-{:>20?}) -  {}",
            i,
            schema_error.schema_path(),
            schema_error.keyword(),
            schema_error.kind(),
            schema_error.instance_path()
        )
    }
}

#[test_case("protocolVersion")]
#[test_case("$schema")]
#[test_case("$id")]
#[test_case("documents")]
#[test_case("ownerId")]
fn property_should_be_present(property: &str) {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    raw_data_contract
        .remove(property)
        .unwrap_or_else(|_| panic!("the {} should exist and be removed", "protocolVersion"));

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert!(matches!(
        schema_error.kind(),
        ValidationErrorKind::Required {
            property: JsonValue::String(missing_property)
        } if missing_property == property
    ));
}
mod protocol {
    use super::*;

    #[test]
    fn protocol_version_should_be_integer() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["protocolVersion"] = json!("1");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/protocolVersion", schema_error.instance_path().to_string());
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn protocol_version_should_be_valid() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["protocolVersion"] = json!(-1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect_err("protocol error should be returned");
        trace!("The validation result is: {:#?}", result);

        assert!(matches!(result, ProtocolError::Error(..)))
    }
}

#[test]
fn defs_should_be_object() {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();
    raw_data_contract["$defs"] = json!(1);

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");
    trace!("The validation result is: {:#?}", result);

    let schema_error = get_schema_error(&result, 0);
    assert_eq!("/$defs", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

mod defs {
    use super::*;
    #[test]
    fn defs_should_not_be_empty() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();
        raw_data_contract["$defs"] = json!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$defs", schema_error.instance_path().to_string());
        assert_eq!(Some("minProperties"), schema_error.keyword(),);
    }

    #[test]
    fn defs_should_have_no_non_alphanumeric_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();
        raw_data_contract["$defs"] = json!({ "$subSchema" : {}});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$defs", schema_error.instance_path().to_string());
        assert_eq!(Some("pattern"), schema_error.keyword(),);
    }

    #[test]
    fn defs_should_have_valid_property_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let valid_names = [
            "validName",
            "valid_name",
            "valid-name",
            "abc",
            "a123123bc",
            "ab123c",
            "ValidName",
            "validName",
            "abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz",
            "abc_gbf_gdb",
            "abc-gbf-gdb",
        ];

        for property_name in valid_names {
            raw_data_contract["$defs"][property_name] = json!({"type" : "string"})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }

    #[test]
    fn defs_with_invalid_property_names_should_return_error() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let invalid_names = [
            "-invalidname",
            "_invalidname",
            "invalidname-",
            "invalidname_",
            "*(*&^",
            "$test",
            "123abci",
            "ab",
        ];
        for property_name in invalid_names {
            raw_data_contract["$defs"][property_name] = json!({"type" : "string"})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!("/$defs", schema_error.instance_path().to_string());
        assert_eq!(Some("pattern"), schema_error.keyword(),);
    }

    #[test]
    fn defs_should_have_no_more_100_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        for i in 1..101 {
            raw_data_contract["$defs"][format!("def_{}", i)] = json!({"type" : "string"})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!("/$defs", schema_error.instance_path().to_string());
        assert_eq!(Some("maxProperties"), schema_error.keyword(),);
    }
}

mod schema {
    use super::*;
    #[test]
    fn schema_should_be_string() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["$schema"] = json!(1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$schema", schema_error.instance_path().to_string());
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn schema_should_be_url() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["$schema"] = json!("wrong");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$schema", schema_error.instance_path().to_string());
        assert_eq!(Some("const"), schema_error.keyword(),);
    }
}

#[test_case("ownerId")]
#[test_case("$id")]
fn owner_id_should_be_byte_array(property_name: &str) {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    let array = ["string"; 32];
    raw_data_contract[property_name] = json!(array);

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    let byte_array_schema_error = get_schema_error(&result, 1);

    assert_eq!(
        format!("/{}/0", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("type"), schema_error.keyword(),);
    assert_eq!(
        format!("/properties/{}/byteArray/items/type", property_name),
        byte_array_schema_error.schema_path().to_string()
    );
}

#[test_case("ownerId")]
#[test_case("$id")]
fn owner_id_should_be_no_less_32_bytes(property_name: &str) {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    let array = [0u8; 31];
    raw_data_contract[property_name] = json!(array);

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("minItems"), schema_error.keyword(),);
}

#[test_case("ownerId")]
#[test_case("$id")]
fn owner_id_should_be_no_longer_32_bytes(property_name: &str) {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    let mut too_long_id = Vec::new();
    too_long_id.resize(33, 0u8);
    raw_data_contract[property_name] = json!(too_long_id);

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("maxItems"), schema_error.keyword(),);
}

mod documents {
    use super::*;
    #[test]
    fn documents_should_be_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"] = json!(1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/documents", schema_error.instance_path().to_string());
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn documents_should_not_be_empty() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"] = json!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/documents", schema_error.instance_path().to_string());
        assert_eq!(Some("minProperties"), schema_error.keyword(),);
    }

    #[test]
    fn documents_should_have_valid_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let nice_document_data_contract = raw_data_contract["documents"]["niceDocument"].clone();
        raw_data_contract["documents"] = json!({});

        let valid_names = [
            "validName",
            "valid_name",
            "valid-name",
            "abc",
            "a123123bc",
            "ab123c",
            "ValidName",
            "validName",
            "abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz",
            "abc_gbf_gdb",
            "abc-gbf-gdb",
        ];

        for document_name in valid_names {
            raw_data_contract["documents"][document_name] = nice_document_data_contract.clone()
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }

    #[test]
    fn documents_with_invalid_format_should_return_error() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let nice_document_data_contract = raw_data_contract["documents"]["niceDocument"].clone();
        raw_data_contract["documents"] = json!({});
        let invalid_names = [
            "-invalidname",
            "_invalidname",
            "invalidname-",
            "invalidname_",
            "*(*&^",
            "$test",
            "123abci",
            "ab",
        ];
        for document_name in invalid_names {
            raw_data_contract["documents"][document_name] = nice_document_data_contract.clone()
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!("/documents", schema_error.instance_path().to_string());
        assert_eq!(Some("pattern"), schema_error.keyword(),);
    }

    #[test]
    fn documents_should_have_no_more_100_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let nice_document_data_contract = raw_data_contract["documents"]["niceDocument"].clone();

        for i in 1..101 {
            raw_data_contract["documents"][format!("document_{}", i)] =
                nice_document_data_contract.clone()
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!("/documents", schema_error.instance_path().to_string());
        assert_eq!(Some("maxProperties"), schema_error.keyword(),);
    }

    #[test]
    fn document_schema_properties_should_not_be_empty() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"] = json!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minProperties"), schema_error.keyword(),);
    }

    #[test]
    fn document_schema_properties_should_have_type_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["type"] = json!("string");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/type",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword(),);
    }

    #[test]
    fn document_schema_should_have_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]
            .remove("properties")
            .expect("the properties should exist and be removed");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword(),);
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "properties"
        ));
    }

    #[test]
    fn document_schema_should_have_nested_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"]["object"] = json!({
          "type": "array",
          "prefixItems": [
            {
              "type": "object",
              "properties": {
                "something": {
                  "type": "object",
                },
              },
              "additionalProperties": false,
            },
          ],
          "items": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/properties/object/prefixItems/0/properties/something",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword(),);
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "properties"
        ));
    }

    #[test]
    fn documents_should_have_valid_property_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let valid_names = [
            "validName",
            "valid_name",
            "valid-name",
            "abc",
            "a123123bc",
            "ab123c",
            "ValidName",
            "validName",
            "abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz",
            "abc_gbf_gdb",
            "abc-gbf-gdb",
        ];

        for property_name in valid_names {
            raw_data_contract["documents"]["niceDocument"]["properties"][property_name] =
                json!({ "type" : "string"})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }

    #[test]
    fn documents_should_have_valid_nested_property_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"]["something"] =
            json!({"type": "object", "properties": json!({}), "additionalProperties" : false});

        let valid_names = [
            "validName",
            "valid_name",
            "valid-name",
            "abc",
            "a123123bc",
            "ab123c",
            "ValidName",
            "validName",
            "abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz",
            "abc_gbf_gdb",
            "abc-gbf-gdb",
        ];

        for property_name in valid_names {
            raw_data_contract["documents"]["niceDocument"]["properties"]["something"]
                ["properties"][property_name] = json!({ "type" : "string"})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        assert!(result.is_valid());
    }

    #[test]
    fn documents_schema_with_invalid_property_names_should_return_error() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let invalid_names = [
            "-invalidname",
            "_invalidname",
            "invalidname-",
            "invalidname_",
            "*(*&^",
            "$test",
            "123abci",
            "ab",
        ];
        for property_name in invalid_names {
            raw_data_contract["documents"]["niceDocument"]["properties"][property_name] = json!({})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/niceDocument/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("pattern"), schema_error.keyword(),);
    }

    #[test]
    fn should_return_invalid_result_if_nested_property_has_invalid_format() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let invalid_names = [
            "-invalidname",
            "_invalidname",
            "invalidname-",
            "invalidname_",
            "*(*&^",
            "$test",
            "123abci",
            "ab",
        ];

        raw_data_contract["documents"]["niceDocument"]["properties"]["something"] = json!({
            "properties" :   json!({}),
            "additionalProperties" :  false,

        });

        for property_name in invalid_names {
            raw_data_contract["documents"]["niceDocument"]["properties"]["something"]
                ["properties"][property_name] = json!({});

            let result = data_contract_validator
                .validate(&raw_data_contract)
                .expect("validation result should be returned");
            let schema_error = get_schema_error(&result, 0);

            assert_eq!(4, result.errors.len());
            assert_eq!(
                "/documents/niceDocument/properties/something/properties",
                schema_error.instance_path().to_string()
            );
            assert_eq!(Some("pattern"), schema_error.keyword());

            raw_data_contract["documents"]["niceDocument"]["properties"]["something"]["properties"]
                .remove(property_name)
                .expect("the property should exist and be removed");
        }
    }

    #[test]
    fn documents_should_have_additional_properties_defined() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]
            .remove("additionalProperties")
            .expect("property should exist and be removed");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword());
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "additionalProperties"
        ));
    }

    #[test]
    fn documents_should_have_additional_properties_defined_to_false() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["additionalProperties"] = json!(true);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/additionalProperties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    fn documents_with_additional_properties_should_be_invalid() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["additionalProperty"] = json!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("", schema_error.instance_path().to_string());
        assert_eq!(Some("additionalProperties"), schema_error.keyword());
    }

    #[test]
    fn documents_should_have_no_more_than_100_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"] = json!({});

        for i in 0..101 {
            raw_data_contract["documents"]["niceDocument"]["properties"][format!("p_{}", i)] = json!({
                "properties": {
                    "something" :  {
                        "type" : "string"
                    }
                },
                "additionalProperties": false,
            })
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/niceDocument/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxProperties"), schema_error.keyword());
    }

    #[test]
    fn documents_should_have_sub_schema_in_items_for_arrays() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["new"] = json!( {
          "properties": {
            "something": {
              "type": "array",
              "items": [
                {
                  "type": "string",
                },
                {
                  "type": "number",
                },
              ],
            },
          },
          "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/new/properties/something/items",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword());
    }

    #[test]
    fn should_have_items_if_prefix_items_is_used_for_arrays() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["new"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "array",
                "prefixItems": [
                  {
                    "type": "string",
                  },
                  {
                    "type": "number",
                  },
                ],
                "minItems": 2,
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/new/properties/something",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword());
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "items"
        ));
    }

    #[test]
    fn should_not_have_items_disabled_if_prefix_items_is_used_for_arrays() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["new"] = json!({
            "properties": {
                "something": {
                  "type": "array",
                  "prefixItems": [
                    {
                      "type": "string",
                    },
                    {
                      "type": "number",
                    },
                  ],
                  "items": true,
                },
              },
              "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/new/properties/something/items",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    #[ignore = "unevaluatedProperties is waiting for implementation https://github.com/Stranger6667/jsonschema-rs/issues/288"]
    fn should_return_invalid_result_if_default_keyword_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["properties"]["firstName"]["default"] =
            json!("1");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        print_json_schema_errors(&result);

        assert_eq!(
            "/documents/new/properties/something/items",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    fn documents_should_be_invalid_if_remote_ref_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] =
            json!({"$ref" : "http://remote.com/schema#"});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/$ref",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("pattern"), schema_error.keyword());
    }

    #[test]
    #[ignore = "unevaluatedProperties is waiting for implementation https://github.com/Stranger6667/jsonschema-rs/issues/288"]
    fn documents_should_not_have_property_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
              },
            },
            "propertyNames": {
              "pattern": "abc",
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        print_json_schema_errors(&result);
        assert_eq!(
            "/documents/indexedDocument",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("unevaluatedProperties"), schema_error.keyword());
    }

    #[test]
    fn documents_should_have_max_items_if_unique_items_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "array",
                "uniqueItems": true,
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/properties/something",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword());
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "maxItems"
        ));
    }

    #[test]
    fn documents_should_have_max_items_not_bigger_than_100000_if_unique_items_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!(
            {
                "type": "object",
                "properties": {
                  "something": {
                    "type": "array",
                    "uniqueItems": true,
                    "maxItems": 200000,
                    "items": {
                      "type": "object",
                      "properties": {
                        "property": {
                          "type": "string",
                        },
                      },
                      "additionalProperties": false,
                    },
                  },
                },
                "additionalProperties": false,
            }
        );

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/properties/something/maxItems",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maximum"), schema_error.keyword());
    }

    #[test]
    fn documents_is_not_valid_and_invalid_result_should_be_returned() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
                "format": "lalala",
                "maxLength": 100,
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        // in the case of rust-dpp, the behavior is slightly different. SchemaError is returned
        // instead of SchemaCompilationError
        assert!(!result.is_valid());
        assert_eq!(
            "/properties/something",
            schema_error.instance_path().to_string()
        );
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Format {format}  if format == &"unknown format"
        ));
    }

    #[test]
    fn documents_should_have_max_length_if_pattern_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
                "pattern": "a",
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/properties/something",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword());
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "maxLength"
        ));
    }

    #[test]
    fn documents_should_have_max_length_no_bigger_than_50000_if_pattern_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
                "format": "url",
                "maxLength": 60000,
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/properties/something/maxLength",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maximum"), schema_error.keyword());
    }

    #[test]
    fn documents_should_not_have_incompatible_patterns() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
                "maxLength": 100,
                "pattern": "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$",
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let pattern_error = result
            .errors
            .get(0)
            .expect("the error in result should exist");

        assert_eq!(1009, pattern_error.get_code());
        assert!(
            matches!(pattern_error, ConsensusError::IncompatibleRe2PatternError { path, pattern, .. }
            if  {
                path == "/documents/indexedDocument/properties/something" &&
                pattern == "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$"
            })
        );
    }
}

mod byte_array {
    use super::*;
    #[test]
    fn byte_array_should_be_boolean() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]
            ["byteArray"] = json!(1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/byteArrayField/byteArray",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword());
    }

    #[test]
    fn byte_array_should_equal_to_true() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]
            ["byteArray"] = json!(false);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/byteArrayField/byteArray",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    fn byte_array_should_be_used_with_type_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]["type"] =
            json!("string");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/byteArrayField/type",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    fn byte_array_should_not_be_used_with_items() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]["items"] =
            json!({ "type" : "string"});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let validation_error = result.errors.get(0).expect("validation error should exist");

        assert_eq!(1004, validation_error.code());
    }
}

mod identifier {
    use super::*;
    #[test]
    fn content_media_type_identifier_should_be_used_with_byte_array_only() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["identifierField"]
            .remove("byteArray")
            .expect("byteArray should exist and be removed");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/identifierField",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword());
    }

    #[test]
    fn content_media_type_identifier_should_be_used_with_byte_array_not_shorter_than_32_bytes() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["identifierField"]
            ["minItems"] = json!(31);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/identifierField/minItems",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    fn content_media_type_identifier_should_be_used_with_byte_array_not_longer_than_32_bytes() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["identifierField"]
            ["maxItems"] = json!(31);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/identifierField/maxItems",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }
}

mod indices {
    use super::*;
    #[test]
    fn indices_should_be_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] =
            json!("definitely not an array");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/indexedDocument/indices",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn indices_should_at_least_one_item() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] = json!([]);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minItems"), schema_error.keyword(),);
    }

    #[test]
    fn indices_should_return_invalid_result_if_there_are_duplicated_indices() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let mut index_definition =
            raw_data_contract["documents"]["indexedDocument"]["indices"][0].clone();
        index_definition["name"] = json!("otherIndexName");

        if let Some(JsonValue::Array(ref mut arr)) =
            raw_data_contract["documents"]["indexedDocument"].get_mut("indices")
        {
            arr.push(index_definition)
        } else {
            panic!("indices is not array")
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let validation_error = result
            .errors
            .get(0)
            .expect("the validation error should be returned");
        let index_error = get_index_error(validation_error);

        assert_eq!(1008, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::DuplicateIndexError { document_type, .. } if document_type == "indexedDocument")
        );
    }

    #[test]
    fn indices_should_return_invalid_result_if_there_are_duplicated_index_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let index_definition =
            raw_data_contract["documents"]["indexedDocument"]["indices"][0].clone();

        if let Some(JsonValue::Array(ref mut arr)) =
            raw_data_contract["documents"]["indexedDocument"].get_mut("indices")
        {
            arr.push(index_definition)
        } else {
            panic!("indices is not array")
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let validation_error = result
            .errors
            .get(0)
            .expect("the validation error should be returned");
        let basic_error = get_basic_error(validation_error);

        assert_eq!(1048, basic_error.get_code());
        assert!(
            matches!(basic_error, BasicError::DuplicateIndexNameError { document_type, duplicate_index_name }
            if  {
                document_type == "indexedDocument" &&
                duplicate_index_name == "index1"
            })
        );
    }

    #[test]
    fn index_should_be_an_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] = json!(["something else"]);
        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn index_should_have_properties_definition() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] = json!([{}]);
        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("required"), schema_error.keyword(),);
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == "properties"
        ));
    }

    #[test]
    fn index_properties_definition_should_be_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"] =
            json!("something else");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn index_properties_definition_should_have_at_least_one_property_defined() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"] = json!([]);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minItems"), schema_error.keyword(),);
    }

    #[test]
    fn index_properties_definition_should_have_no_more_than_10_property_def() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        for i in 0..10 {
            if let Some(JsonValue::Array(ref mut properties)) = raw_data_contract["documents"]
                ["indexedDocument"]["indices"][0]
                .get_mut("properties")
            {
                let field_name = format!("field{}", i);
                properties.push(json!({
                    field_name : "asc"
                }))
            }
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxItems"), schema_error.keyword(),);
    }

    #[test]
    fn index_property_definition_should_be_an_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"][0] =
            json!("something else");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn index_properties_should_have_at_least_one_property() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"] = json!([]);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minItems"), schema_error.keyword(),);
    }

    #[test]
    fn index_property_should_have_no_more_than_one_property() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let property =
            &mut raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"][0];
        property["anotherField"] = json!("something");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 1);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxProperties"), schema_error.keyword(),);
    }

    #[test]
    fn index_properties_should_have_value_asc_desc() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"][0]
            ["$ownerId"] = json!("wrong");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties/0/$ownerId",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("enum"), schema_error.keyword(),);
    }

    #[test]
    fn index_properties_should_have_unique_flag_to_be_of_boolean_type() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["unique"] = json!(12);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/unique",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn indices_should_have_no_more_than_10_indices() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        for i in 0..10 {
            let property_name = format!("field{}", i);
            raw_data_contract["documents"]["indexedDocument"]["properties"]
                .insert(property_name.clone(), json!({ "type" : "string"}))
                .expect("properties should be present");

            if let Some(JsonValue::Array(ref mut indices)) =
                raw_data_contract["documents"]["indexedDocument"].get_mut("indices")
            {
                indices.push(json!({
                   "name" : format!("{}_index", property_name),
                   "properties" : [ { property_name : "asc"}]
                }))
            }
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxItems"), schema_error.keyword(),);
    }

    #[test]
    fn indices_should_have_no_more_than_3_unique_indices() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        for i in 0..4 {
            let property_name = format!("field{}", i);
            raw_data_contract["documents"]["indexedDocument"]["properties"]
                .insert(
                    property_name.clone(),
                    json!({ "type" : "string", "maxLength" : 63 }),
                )
                .expect("properties should be present");

            if let Some(JsonValue::Array(ref mut indices)) =
                raw_data_contract["documents"]["indexedDocument"].get_mut("indices")
            {
                indices.push(json!({
                   "name" : format!("index_{}", i),
                   "properties" : [ { property_name : "asc"}],
                   "unique" : true
                }))
            }
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1017, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::UniqueIndicesLimitReachedError { document_type, index_limit }
            if  {
                document_type == "indexedDocument" &&
                index_limit == &3
            })
        );
    }

    #[test]
    fn index_property_should_not_be_named_id() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let index_definition = json!({
            "name" : "index_1",
            "properties" : [
                { "$id"  : "asc"},
                { "firstName"  : "desc"},
            ]
        });

        if let Some(JsonValue::Array(ref mut indices)) =
            raw_data_contract["documents"]["indexedDocument"].get_mut("indices")
        {
            indices.push(index_definition)
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1015, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::SystemPropertyIndexAlreadyPresentError { document_type, property_name, .. }
            if  {
                document_type == "indexedDocument" &&
                property_name == "$id"
            })
        );
    }

    #[test]
    fn index_should_not_have_undefined_property() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        if let Some(JsonValue::Array(ref mut index_properties)) =
            raw_data_contract["documents"]["indexedDocument"]["indices"][0].get_mut("properties")
        {
            index_properties.push(json!({ "missingProperty"  : "asc"}))
        } else {
            panic!("the index properties are not array")
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1016, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::UndefinedIndexPropertyError { document_type, property_name, .. }
            if  {
                document_type == "indexedDocument" &&
                property_name == "missingProperty"
            })
        );
    }

    #[test]
    fn index_property_should_not_be_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let object_property = json!({
            "type" : "object",
            "properties" :  {
                "something" : {
                    "type" : "string"
                }
            },
            "additionalProperties" : false
        });

        raw_data_contract["documents"]["indexedDocument"]["properties"]["objectProperty"] =
            object_property;
        if let Some(JsonValue::Array(ref mut required)) =
            raw_data_contract["documents"]["indexedDocument"].get_mut("required")
        {
            required.push(json!("objectProperty"))
        }
        if let Some(JsonValue::Array(ref mut properties)) =
            raw_data_contract["documents"]["indexedDocument"]["indices"][0].get_mut("properties")
        {
            properties.push(json!({"objectProperty" : "asc" }))
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1013, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::InvalidIndexPropertyTypeError { document_type, property_name, property_type, ..}
            if  {
                document_type == "indexedDocument" &&
                property_name == "objectProperty" &&
                property_type == "object"
            })
        );
    }

    #[test]
    fn index_property_should_not_point_to_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedArray"] = json!({
            "type": "object",
            "indices": [
              {
                "name": "index1",
                "properties": [
                  { "mentions": "asc" },
                ],
              },
            ],
            "properties": {
              "mentions": {
                "type": "array",
                "prefixItems": [
                  {
                    "type": "string",
                    "maxLength": 100,
                  },
                ],
                "minItems": 1,
                "maxItems": 5,
                "items": false,
              },
            },
            "additionalProperties": false,
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1013, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::InvalidIndexPropertyTypeError { document_type, property_name, property_type, ..}
            if  {
                document_type == "indexedArray" &&
                property_name == "mentions" &&
                property_type == "array"
            })
        );
    }

    // This section is originally commented out
    // https://github.com/dashevo/platform/blob/ab6391f4b47a970c733e7b81115b44329fbdf993/packages/js-dpp/test/integration/dataContract/validation/validateDataContractFactory.spec.js#L1603
    //
    // it('should return invalid result if index property is array of objects', async () => {
    //   const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;
    //
    //   indexedDocumentDefinition.properties.arrayProperty = {
    //     type: 'array',
    //     items: {
    //       type: 'object',
    //       properties: {
    //         something: {
    //           type: 'string',
    //         },
    //       },
    //       additionalProperties: false,
    //     },
    //   };
    //
    //   indexedDocumentDefinition.required.push('arrayProperty');
    //
    //   const indexDefinition = indexedDocumentDefinition.indices[0];
    //
    //   indexDefinition.properties.push({
    //     arrayProperty: 'asc',
    //   });
    //
    //   const result = await validateDataContract(rawDataContract);
    //
    //   expectValidationError(result, InvalidIndexPropertyTypeError);
    //
    //   const [error] = result.getErrors();
    //
    //   expect(error.getCode()).to.equal(1013);
    //   expect(error.getPropertyName()).to.equal('arrayProperty');
    //   expect(error.getPropertyType()).to.equal('array');
    //   expect(error.getDocumentType()).to.deep.equal('indexedDocument');
    //   expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    // });

    // it('should return invalid result if index property is an array of different types',
    // async () => {
    //   const indexedDocumentDefinition = rawDataContract.documents.indexedArray;
    //
    //   const indexDefinition = indexedDocumentDefinition.indices[0];
    //
    //   rawDataContract.documents.indexedArray.properties.mentions.prefixItems = [
    //     {
    //       type: 'string',
    //     },
    //     {
    //       type: 'number',
    //     },
    //   ];
    //
    //   rawDataContract.documents.indexedArray.properties.mentions.minItems = 2;
    //
    //   const result = await validateDataContract(rawDataContract);
    //   expectValidationError(result, InvalidIndexPropertyTypeError);
    //
    //   const error = result.getFirstError();
    //
    //   expect(error.getCode()).to.equal(1013);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getPropertyType()).to.equal('array');
    //   expect(error.getDocumentType()).to.deep.equal('indexedArray');
    //   expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    // });
    //
    // it('should return invalid result if index property contained prefixItems array of arrays',
    // async () => {
    //   const indexedDocumentDefinition = rawDataContract.documents.indexedArray;
    //
    //   const indexDefinition = indexedDocumentDefinition.indices[0];
    //
    //   rawDataContract.documents.indexedArray.properties.mentions.prefixItems = [
    //     {
    //       type: 'array',
    //       items: {
    //         type: 'string',
    //       },
    //     },
    //   ];
    //
    //   const result = await validateDataContract(rawDataContract);
    //   expectValidationError(result, InvalidIndexPropertyTypeError);
    //
    //   const error = result.getFirstError();
    //
    //   expect(error.getCode()).to.equal(1013);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getPropertyType()).to.equal('array');
    //   expect(error.getDocumentType()).to.deep.equal('indexedArray');
    //   expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    // });

    // it('should return invalid result if index property contained prefixItems array of objects',
    // async () => {
    //   const indexedDocumentDefinition = rawDataContract.documents.indexedArray;
    //
    //   const indexDefinition = indexedDocumentDefinition.indices[0];
    //
    //   rawDataContract.documents.indexedArray.properties.mentions.prefixItems = [
    //     {
    //       type: 'object',
    //       properties: {
    //         something: {
    //           type: 'string',
    //         },
    //       },
    //       additionalProperties: false,
    //     },
    //   ];
    //
    //   const result = await validateDataContract(rawDataContract);
    //   expectValidationError(result, InvalidIndexPropertyTypeError);
    //
    //   const error = result.getFirstError();
    //
    //   expect(error.getCode()).to.equal(1013);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getPropertyType()).to.equal('array');
    //   expect(error.getDocumentType()).to.deep.equal('indexedArray');
    //   expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    // });
    //
    // it('should return invalid result if index property is array of arrays', async () => {
    //   const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;
    //
    //   indexedDocumentDefinition.properties.arrayProperty = {
    //     type: 'array',
    //     items: {
    //       type: 'array',
    //       items: {
    //         type: 'string',
    //       },
    //     },
    //   };
    //
    //   indexedDocumentDefinition.required.push('arrayProperty');
    //
    //   const indexDefinition = indexedDocumentDefinition.indices[0];
    //
    //   indexDefinition.properties.push({
    //     arrayProperty: 'asc',
    //   });
    //
    //   const result = await validateDataContract(rawDataContract);
    //
    //   expectValidationError(result, InvalidIndexPropertyTypeError);
    //
    //   const [error] = result.getErrors();
    //
    //   expect(error.getCode()).to.equal(1013);
    //   expect(error.getPropertyName()).to.equal('arrayProperty');
    //   expect(error.getPropertyType()).to.equal('array');
    //   expect(error.getDocumentType()).to.deep.equal('indexedDocument');
    //   expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    // });

    #[test]
    fn should_return_invalid_result_if_index_property_is_array_with_different_item_definitions() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let indexed_document_definition = &mut raw_data_contract["documents"]["indexedDocument"];
        indexed_document_definition["properties"]["arrayProperty"] = json!({
            "type": "array",
            "prefixItems": [
              {
                "type": "string",
              },
              {
                "type": "number",
              },
            ],
            "minItems": 2,
            "items": false,
        });
        indexed_document_definition["required"]
            .push(json!("arrayProperty"))
            .expect("array should exist");
        let index_definition = &mut indexed_document_definition["indices"][0];
        index_definition["properties"]
            .push(json!({ "arrayProperty" : "asc"}))
            .expect("properties of index should exist");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1013, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::InvalidIndexPropertyTypeError { document_type, property_name, property_type, ..}
            if  {
                document_type == "indexedDocument" &&
                property_name == "arrayProperty" &&
                property_type == "array"
            })
        );
    }

    #[test]
    fn should_return_invalid_result_if_unique_compound_index_contains_both_required_and_optional_properties(
    ) {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        if let Some(JsonValue::Array(arr)) =
            raw_data_contract["documents"]["optionalUniqueIndexedDocument"].get_mut("required")
        {
            arr.pop();
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_index_error(error);

        assert_eq!(1010, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::InvalidCompoundIndexError { document_type, ..}
            if  {
                document_type == "optionalUniqueIndexedDocument"
            })
        );
    }
}

mod signature_level {
    use super::*;
    #[test]
    fn signature_level_requirement_should_be_number() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["signatureSecurityLevelRequirement"] =
            json!("definitely not a number");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/signatureSecurityLevelRequirement",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn signature_level_requirement_should_be_one_of_available_values() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["signatureSecurityLevelRequirement"] =
            json!(199);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/signatureSecurityLevelRequirement",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("enum"), schema_error.keyword(),);
    }
}

mod dependent_schemas {
    use super::*;

    #[test]
    fn dependent_schemas_should_be_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "abc": {
                "type": "string",
              },
            },
            "additionalProperties": false,
            "dependentSchemas": "string",
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/dependentSchemas",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn dependent_required_should_be_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "abc": {
                "type": "string",
              },
            },
            "additionalProperties": false,
            "dependentRequired": "string",
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/dependentRequired",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn dependent_required_should_have_array_value() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "abc": {
                "type": "string",
              },
            },
            "additionalProperties": false,
            "dependentRequired":  {
                "zxy": {
                    "type": "number",
              }}
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/dependentRequired/zxy",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn dependent_required_should_have_array_of_strings() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "abc": {
                "type": "string",
              },
            },
            "additionalProperties": false,
            "dependentRequired":  {
                "zxy":  [ 1, "2" ]
              }
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/dependentRequired/zxy/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn dependent_required_should_have_array_of_unique_strings() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = json!({
            "type": "object",
            "properties": {
              "abc": {
                "type": "string",
              },
            },
            "additionalProperties": false,
            "dependentRequired":  {
                "zxy":  [ "1", "2", "2" ]
              }
        });

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/dependentRequired/zxy",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("uniqueItems"), schema_error.keyword(),);
    }
}

#[test]
fn should_return_invalid_result_with_circular_ref_pointer() {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    raw_data_contract["$defs"]["object"] = json!({ "$ref" : "#/$defs/object"});

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");
    let validation_error = result
        .errors
        .get(0)
        .expect("the validation error should exist");
    let basic_error = get_basic_error(validation_error);

    assert_eq!(1014, validation_error.get_code());
    assert!(
        matches!(basic_error, BasicError::InvalidJsonSchemaRefError { ref_error}
        if  {
            ref_error == "the ref '#/$defs/object' contains cycles"
        })
    );
}

#[test]
fn should_return_invalid_result_if_indexed_string_property_missing_max_length_constraint() {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    raw_data_contract["documents"]["indexedDocument"]["properties"]["firstName"]
        .remove("maxLength")
        .expect("the property should exist and be removed");

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let validation_error = result
        .errors
        .get(0)
        .expect("the validation error should exist");
    let index_error = get_index_error(validation_error);

    assert_eq!(1012, index_error.get_code());
    assert!(
        matches!(index_error, IndexError::InvalidIndexedPropertyConstraintError { property_name, constraint_name, reason, ..}
        if  {
            property_name == "firstName" &&
            constraint_name == "maxLength" &&
            reason == "should be less or equal than 63"
        })
    );
}

mod indexed_array {
    use super::*;
    // This section is originally commented out
    // https://github.com/dashevo/platform/blob/ab6391f4b47a970c733e7b81115b44329fbdf993/packages/js-dpp/test/integration/dataContract/validation/validateDataContractFactory.spec.js#L2015
    //
    // it('should return invalid result if indexed array property missing maxItems constraint',
    // async () => {
    //   delete rawDataContract.documents.indexedArray.properties.mentions.maxItems;
    //
    //   const result = await validateDataContract(rawDataContract);
    //
    //   expectValidationError(result, InvalidIndexedPropertyConstraintError);
    //
    //   const [error] = result.getErrors();
    //
    //   expect(error.getCode()).to.equal(1012);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getConstraintName()).to.equal('maxItems');
    //   expect(error.getReason()).to.equal('should be less or equal 63');
    // });
    //
    // it('should return invalid result if indexed array property have to big maxItems', async () => {
    //   rawDataContract.documents.indexedArray.properties.mentions.maxItems = 2048;
    //
    //   const result = await validateDataContract(rawDataContract);
    //
    //   expectValidationError(result, InvalidIndexedPropertyConstraintError);
    //
    //   const [error] = result.getErrors();
    //
    //   expect(error.getCode()).to.equal(1012);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getConstraintName()).to.equal('maxItems');
    //   expect(error.getReason()).to.equal('should be less or equal 63');
    // });
    //
    // it('should return invalid result if indexed array property
    // have string item without maxItems constraint', async () => {
    //   delete rawDataContract.documents.indexedArray.properties.mentions.maxItems;
    //
    //   const result = await validateDataContract(rawDataContract);
    //
    //   expectValidationError(result, InvalidIndexedPropertyConstraintError);
    //
    //   const [error] = result.getErrors();
    //
    //   expect(error.getCode()).to.equal(1012);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getConstraintName()).to.equal('maxItems');
    //   expect(error.getReason()).to.equal('should be less or equal 63');
    // });
    //
    // it('should return invalid result if indexed array property have
    // string item with maxItems bigger than 1024', async () => {
    //   rawDataContract.documents.indexedArray.properties.mentions.maxItems = 2048;
    //
    //   const result = await validateDataContract(rawDataContract);
    //
    //   expectValidationError(result, InvalidIndexedPropertyConstraintError);
    //
    //   const [error] = result.getErrors();
    //
    //   expect(error.getCode()).to.equal(1012);
    //   expect(error.getPropertyName()).to.equal('mentions');
    //   expect(error.getConstraintName()).to.equal('maxItems');
    //   expect(error.getReason()).to.equal('should be less or equal 63');
    // });

    #[test]
    fn should_return_invalid_result_if_indexed_byte_array_property_missing_max_items_constraint() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]
            .remove("maxItems")
            .expect("the property should exist and be removed");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let validation_error = result
            .errors
            .get(0)
            .expect("the validation error should exist");
        let index_error = get_index_error(validation_error);

        assert_eq!(1012, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::InvalidIndexedPropertyConstraintError { property_name, constraint_name, reason, ..}
            if  {
                property_name == "byteArrayField" &&
                constraint_name == "maxItems" &&
                reason == "should be less or equal 255"
            })
        );
    }

    #[test]
    fn should_return_invalid_result_if_indexed_byte_array_property_have_to_big_max_items() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]
            ["maxItems"] = json!(8192);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let validation_error = result
            .errors
            .get(0)
            .expect("the validation error should exist");
        let index_error = get_index_error(validation_error);

        assert_eq!(1012, index_error.get_code());
        assert!(
            matches!(index_error, IndexError::InvalidIndexedPropertyConstraintError { property_name, constraint_name, reason, ..}
            if  {
                property_name == "byteArrayField" &&
                constraint_name == "maxItems" &&
                reason == "should be less or equal 255"
            })
        );
    }
}

#[test]
fn should_return_valid_result_if_data_contract_is_valid() {
    let TestData {
        raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");
    assert!(result.is_valid());
}
