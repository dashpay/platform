use std::sync::Arc;

use log::trace;
use platform_value::{platform_value, Value};
use test_case::test_case;

use crate::consensus::basic::BasicError;
use crate::errors::consensus::codes::ErrorWithCode;
use crate::tests::utils::json_schema_error;
use crate::{
    consensus::{basic::json_schema_error::JsonSchemaError, ConsensusError},
    prelude::*,
    tests::fixtures::get_data_contract_fixture,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

struct TestData {
    data_contract_validator: DataContractValidator,
    data_contract: DataContract,
    raw_data_contract: Value,
}

fn setup_test() -> TestData {
    init();

    let data_contract = get_data_contract_fixture(None).data_contract;
    let raw_data_contract = data_contract.to_object().unwrap();

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
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

fn get_schema_error<TData: Clone>(
    result: &ConsensusValidationResult<TData>,
    number: usize,
) -> &JsonSchemaError {
    json_schema_error(
        result
            .errors
            .get(number)
            .expect("the error should be returned in validation result"),
    )
}

fn get_basic_error(consensus_error: &ConsensusError) -> &BasicError {
    match consensus_error {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!("error '{:?}' isn't a basic error", consensus_error),
    }
}

fn print_json_schema_errors<TData: Clone>(result: &ConsensusValidationResult<TData>) {
    for (i, e) in result.errors.iter().enumerate() {
        let schema_error = json_schema_error(e);
        println!(
            "error_{}:  {:>30} {:>20} {:>30} -({:>30? }) -  {}",
            i,
            schema_error.schema_path(),
            schema_error.keyword(),
            schema_error.property_name(),
            schema_error.params(),
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

    assert_eq!(schema_error.keyword(), "required");
    assert_eq!(schema_error.property_name(), property);
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

        raw_data_contract
            .set_value("protocolVersion", "1".into())
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/protocolVersion", schema_error.instance_path());
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn protocol_version_should_be_valid() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract
            .set_value("protocolVersion", Value::I8(-1))
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/protocolVersion", schema_error.instance_path());
        assert_eq!("minimum", schema_error.keyword());
    }
}

#[test]
fn defs_should_be_object() {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    raw_data_contract
        .set_value("$defs", Value::U32(1))
        .expect("expected to set value");

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");
    trace!("The validation result is: {:#?}", result);

    let schema_error = get_schema_error(&result, 0);
    assert_eq!("/$defs", schema_error.instance_path());
    assert_eq!("type", schema_error.keyword());
}

mod defs {
    use super::*;
    use platform_value::platform_value;

    #[test]
    fn defs_should_not_be_empty() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();
        raw_data_contract
            .set_value("$defs", Value::Map(vec![]))
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$defs", schema_error.instance_path());
        assert_eq!("minProperties", schema_error.keyword());
    }

    #[test]
    fn defs_should_have_no_non_alphanumeric_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();
        raw_data_contract
            .set_value(
                "$defs",
                Value::Map(vec![(
                    Value::Text("$subSchema".to_string()),
                    Value::Map(vec![]),
                )]),
            )
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$defs", schema_error.instance_path());
        assert_eq!("propertyNames", schema_error.keyword());
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
            "-validname",
            "_validname",
            "validname-",
            "validname_",
            "a",
            "ab",
            "1",
            "123",
            "123_",
            "-123",
            "_123",
        ];

        for property_name in valid_names {
            raw_data_contract
                .set_value_at_path("$defs", property_name, platform_value!({"type" : "string"}))
                .expect("expected to set value");
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
            raw_data_contract
                .set_value_at_path("$defs", property_name, platform_value!({"type" : "string"}))
                .expect("expected to set value");
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!("/$defs", schema_error.instance_path());
        assert_eq!("propertyNames", schema_error.keyword());
    }

    #[test]
    fn defs_should_have_no_more_100_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        for i in 1..101 {
            raw_data_contract
                .set_value_at_path(
                    "$defs",
                    format!("def_{}", i).as_str(),
                    platform_value!({"type" : "string"}),
                )
                .expect("expected to set value");
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!("/$defs", schema_error.instance_path().to_string());
        assert_eq!("maxProperties", schema_error.keyword());
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

        raw_data_contract
            .set_value("$schema", Value::U64(1))
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$schema", schema_error.instance_path().to_string());
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn schema_should_be_url() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract
            .set_value("$schema", Value::Text("wrong".to_string()))
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$schema", schema_error.instance_path().to_string());
        assert_eq!("const", schema_error.keyword());
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
    raw_data_contract
        .set_value(property_name, platform_value!(array))
        .expect("expected to set value");

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    let byte_array_schema_error = get_schema_error(&result, 1);

    assert_eq!(
        format!("/{}/0", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!("type", schema_error.keyword());
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
    raw_data_contract
        .set_value(property_name, platform_value!(array))
        .expect("expected to set value");

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!("minItems", schema_error.keyword());
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
    raw_data_contract
        .set_value(property_name, platform_value!(too_long_id))
        .expect("expected to set value");

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!("maxItems", schema_error.keyword());
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

        raw_data_contract
            .set_value("documents", platform_value!(1))
            .expect("expected to set value");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/documents", schema_error.instance_path().to_string());
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn documents_should_not_be_empty() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract
            .set_value("documents", platform_value!({}))
            .expect("expected to set value");
        raw_data_contract["documents"] = platform_value!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/documents", schema_error.instance_path().to_string());
        assert_eq!("minProperties", schema_error.keyword());
    }

    #[test]
    fn documents_should_have_valid_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let nice_document_data_contract = raw_data_contract["documents"]["niceDocument"].clone();
        raw_data_contract["documents"] = platform_value!({});

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
        raw_data_contract["documents"] = platform_value!({});
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
        assert_eq!("propertyNames", schema_error.keyword());
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
        assert_eq!("maxProperties", schema_error.keyword());
    }

    #[test]
    fn document_schema_properties_should_not_be_empty() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"] = platform_value!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!("minProperties", schema_error.keyword());
    }

    #[test]
    fn document_schema_properties_should_have_type_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["type"] = platform_value!("string");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/type",
            schema_error.instance_path().to_string()
        );
        assert_eq!("const", schema_error.keyword());
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
        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "properties");
    }

    #[test]
    fn document_schema_should_have_nested_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"]["object"] = platform_value!({
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

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "properties");
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
                platform_value!({ "type" : "string"})
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

        raw_data_contract["documents"]["niceDocument"]["properties"]["something"] = platform_value!({"type": "object", "properties": platform_value!({}), "additionalProperties" : false});

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
                ["properties"][property_name] = platform_value!({ "type" : "string"})
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

        let invalid_names = ["*(*&^", "$test", ".", ".a"];
        for property_name in invalid_names {
            raw_data_contract["documents"]["niceDocument"]["properties"][property_name] =
                platform_value!({})
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/niceDocument/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!("propertyNames", schema_error.keyword());
    }

    #[test]
    fn should_return_invalid_result_if_nested_property_has_invalid_format() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let invalid_names = ["*(*&^", "$test", ".", ".a"];

        raw_data_contract["documents"]["niceDocument"]["properties"]["something"] = platform_value!({
            "properties" :   platform_value!({}),
            "additionalProperties" :  false,
        });

        for property_name in invalid_names {
            raw_data_contract["documents"]["niceDocument"]["properties"]["something"]
                ["properties"][property_name] = platform_value!({});

            let result = data_contract_validator
                .validate(&raw_data_contract)
                .expect("validation result should be returned");
            let schema_error = get_schema_error(&result, 0);

            assert_eq!(4, result.errors.len());
            assert_eq!(
                "/documents/niceDocument/properties/something/properties",
                schema_error.instance_path().to_string()
            );
            assert_eq!("propertyNames", schema_error.keyword());

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

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "additionalProperties");
    }

    #[test]
    fn documents_should_have_additional_properties_defined_to_false() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["additionalProperties"] =
            platform_value!(true);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/niceDocument/additionalProperties",
            schema_error.instance_path().to_string()
        );
        assert_eq!("const", schema_error.keyword());
    }

    #[test]
    fn documents_with_additional_properties_should_be_invalid() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["additionalProperty"] = platform_value!({});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("", schema_error.instance_path().to_string());
        assert_eq!("additionalProperties", schema_error.keyword());
    }

    #[test]
    fn documents_should_have_no_more_than_100_properties() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["niceDocument"]["properties"] = platform_value!({});

        for i in 0..101 {
            raw_data_contract["documents"]["niceDocument"]["properties"][format!("p_{}", i)] = platform_value!({
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
        assert_eq!("maxProperties", schema_error.keyword());
    }

    #[test]
    fn documents_should_have_sub_schema_in_items_for_arrays() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["new"] = platform_value!( {
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
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn should_have_items_if_prefix_items_is_used_for_arrays() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["new"] = platform_value!({
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
        assert_eq!("required", schema_error.keyword());

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "items");
    }

    #[test]
    fn should_not_have_items_disabled_if_prefix_items_is_used_for_arrays() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["new"] = platform_value!({
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
        assert_eq!("const", schema_error.keyword());
    }

    #[test]
    fn should_return_invalid_result_if_default_keyword_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["properties"]["firstName"]["default"] =
            platform_value!("1");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        print_json_schema_errors(&result);

        assert_eq!(
            "/documents/indexedDocument/properties/firstName/default",
            schema_error.instance_path().to_string()
        );
        assert_eq!("unevaluatedProperties", schema_error.keyword());
    }

    #[test]
    fn documents_should_be_invalid_if_remote_ref_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] =
            platform_value!({"$ref" : "http://remote.com/schema#"});

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/$ref",
            schema_error.instance_path().to_string()
        );
        assert_eq!("pattern", schema_error.keyword());
    }

    #[test]
    fn documents_should_not_have_property_names() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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
            "/documents/indexedDocument/propertyNames",
            schema_error.instance_path().to_string()
        );
        assert_eq!("unevaluatedProperties", schema_error.keyword());
    }

    #[test]
    fn documents_should_have_max_items_if_unique_items_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "maxItems");
    }

    #[test]
    fn documents_should_have_max_items_not_bigger_than_100000_if_unique_items_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!(
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
        assert_eq!("maximum", schema_error.keyword());
    }

    #[test]
    fn documents_is_not_valid_and_invalid_result_should_be_returned() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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

        let param_type = schema_error
            .params()
            .get_str("format")
            .expect("should get type");

        assert_eq!(schema_error.keyword(), "format");
        assert_eq!(param_type, "unknown format");
    }

    #[test]
    fn documents_should_have_max_length_if_pattern_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "maxLength");
    }

    #[test]
    fn documents_should_have_max_length_no_bigger_than_50000_if_pattern_is_used() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
                "format": "uri",
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
        assert_eq!("maximum", schema_error.keyword());
    }

    #[test]
    fn documents_should_not_have_incompatible_patterns() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
            "type": "object",
            "properties": {
              "something": {
                "type": "string",
                "maxLength": 100u64,
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

        assert_eq!(1009, pattern_error.code());

        match pattern_error {
            ConsensusError::BasicError(BasicError::IncompatibleRe2PatternError(err)) => {
                assert_eq!(
                    err.path(),
                    "/documents/indexedDocument/properties/something".to_string()
                );
                assert_eq!(
                    err.pattern(),
                    "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$".to_string()
                );
            }
            _ => panic!(
                "Expected IncompatibleRe2PatternError, got {:?}",
                pattern_error
            ),
        }
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
            ["byteArray"] = platform_value!(1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/byteArrayField/byteArray",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn byte_array_should_equal_to_true() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]
            ["byteArray"] = platform_value!(false);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/byteArrayField/byteArray",
            schema_error.instance_path().to_string()
        );
        assert_eq!("const", schema_error.keyword());
    }

    #[test]
    fn byte_array_should_be_used_with_type_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]["type"] =
            platform_value!("string");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/byteArrayField/type",
            schema_error.instance_path().to_string()
        );
        assert_eq!("const", schema_error.keyword());
    }

    #[test]
    fn byte_array_should_not_be_used_with_items() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]["items"] =
            platform_value!({ "type" : "string"});

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
        assert_eq!("required", schema_error.keyword());
    }

    #[test]
    fn content_media_type_identifier_should_be_used_with_byte_array_not_shorter_than_32_bytes() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["identifierField"]
            ["minItems"] = platform_value!(31);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/identifierField/minItems",
            schema_error.instance_path().to_string()
        );
        assert_eq!("const", schema_error.keyword());
    }

    #[test]
    fn content_media_type_identifier_should_be_used_with_byte_array_not_longer_than_32_bytes() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["identifierField"]
            ["maxItems"] = platform_value!(31);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/withByteArrays/properties/identifierField/maxItems",
            schema_error.instance_path().to_string()
        );
        assert_eq!("const", schema_error.keyword());
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
            platform_value!("definitely not an array");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/indexedDocument/indices",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn indices_should_at_least_one_item() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] = platform_value!([]);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices",
            schema_error.instance_path().to_string()
        );
        assert_eq!("minItems", schema_error.keyword());
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
        index_definition["name"] = platform_value!("otherIndexName");

        if let Some(Value::Array(ref mut arr)) = raw_data_contract["documents"]["indexedDocument"]
            .get_mut("indices")
            .unwrap()
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

        match basic_error {
            BasicError::DuplicateIndexError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
            }
            _ => panic!("Expected DuplicateIndexError, got {}", basic_error),
        }
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

        if let Some(Value::Array(ref mut arr)) = raw_data_contract["documents"]["indexedDocument"]
            .get_mut("indices")
            .unwrap()
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

        assert_eq!(1048, basic_error.code());
        match basic_error {
            BasicError::DuplicateIndexNameError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
                assert_eq!(err.duplicate_index_name(), "index1".to_string())
            }
            _ => panic!("Expected DuplicateIndexNameError, got {}", basic_error),
        }
    }

    #[test]
    fn index_should_be_an_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] =
            platform_value!(["something else"]);
        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn index_should_have_properties_definition() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"] = platform_value!([{}]);
        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0",
            schema_error.instance_path().to_string()
        );

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), "properties");
    }

    #[test]
    fn index_properties_definition_should_be_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"] =
            platform_value!("something else");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn index_properties_definition_should_have_at_least_one_property_defined() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"] =
            platform_value!([]);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!("minItems", schema_error.keyword());
    }

    #[test]
    fn index_properties_definition_should_have_no_more_than_10_property_def() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        for i in 0..10 {
            if let Some(Value::Array(ref mut properties)) = raw_data_contract["documents"]
                ["indexedDocument"]["indices"][0]
                .get_mut("properties")
                .unwrap()
            {
                let field_name = format!("field{}", i);
                properties.push(platform_value!({
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
        assert_eq!("maxItems", schema_error.keyword());
    }

    #[test]
    fn index_property_definition_should_be_an_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"][0] =
            platform_value!("something else");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn index_properties_should_have_at_least_one_property() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"] =
            platform_value!([]);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!("minItems", schema_error.keyword());
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
        property["anotherField"] = platform_value!("something");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 1);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties/0",
            schema_error.instance_path().to_string()
        );
        assert_eq!("maxProperties", schema_error.keyword());
    }

    #[test]
    fn index_properties_should_have_value_asc_desc() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"][0]
            ["$ownerId"] = platform_value!("wrong");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/properties/0/$ownerId",
            schema_error.instance_path().to_string()
        );
        assert_eq!("enum", schema_error.keyword());
    }

    #[test]
    fn index_properties_should_have_unique_flag_to_be_of_boolean_type() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["indices"][0]["unique"] =
            platform_value!(12);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/indices/0/unique",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
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
                .insert(property_name.clone(), platform_value!({ "type" : "string"}))
                .expect("properties should be present");

            if let Some(Value::Array(ref mut indices)) = raw_data_contract["documents"]
                ["indexedDocument"]
                .get_mut("indices")
                .unwrap()
            {
                indices.push(platform_value!({
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
        assert_eq!("maxItems", schema_error.keyword());
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
                    platform_value!({ "type" : "string", "maxLength" : 63 }),
                )
                .expect("properties should be present");

            if let Some(Value::Array(ref mut indices)) = raw_data_contract["documents"]
                ["indexedDocument"]
                .get_mut("indices")
                .unwrap()
            {
                indices.push(platform_value!({
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
        let basic_error = get_basic_error(error);

        assert_eq!(1017, basic_error.code());

        match basic_error {
            BasicError::UniqueIndicesLimitReachedError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
                assert_eq!(err.index_limit(), 3);
                // assert_eq!(err.property_type(), "array".to_string());
            }
            _ => panic!(
                "Expected UniqueIndicesLimitReachedError, got {}",
                basic_error
            ),
        }
    }

    #[test]
    fn index_property_should_not_be_named_id() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let index_definition = platform_value!({
            "name" : "index_1",
            "properties" : [
                { "$id"  : "asc"},
                { "firstName"  : "asc"},
            ]
        });

        if let Some(Value::Array(ref mut indices)) = raw_data_contract["documents"]
            ["indexedDocument"]
            .get_mut("indices")
            .unwrap()
        {
            indices.push(index_definition)
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let basic_error = get_basic_error(error);

        assert_eq!(1015, basic_error.code());
        match basic_error {
            BasicError::SystemPropertyIndexAlreadyPresentError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
                assert_eq!(err.property_name(), "$id".to_string());
            }
            _ => panic!(
                "Expected SystemPropertyIndexAlreadyPresentError, got {}",
                basic_error
            ),
        }
    }

    #[test]
    fn index_should_not_have_undefined_property() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        if let Some(Value::Array(ref mut index_properties)) = raw_data_contract["documents"]
            ["indexedDocument"]["indices"][0]
            .get_mut("properties")
            .unwrap()
        {
            index_properties.push(platform_value!({ "missingProperty"  : "asc"}))
        } else {
            panic!("the index properties are not array")
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let basic_error = get_basic_error(error);

        assert_eq!(1016, basic_error.code());

        match basic_error {
            BasicError::UndefinedIndexPropertyError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
                assert_eq!(err.property_name(), "missingProperty".to_string());
            }
            _ => panic!("Expected UndefinedIndexPropertyError, got {}", basic_error),
        }
    }

    #[test]
    fn index_property_should_not_be_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let object_property = platform_value!({
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
        if let Some(Value::Array(ref mut required)) = raw_data_contract["documents"]
            ["indexedDocument"]
            .get_mut("required")
            .unwrap()
        {
            required.push(platform_value!("objectProperty"))
        }
        if let Some(Value::Array(ref mut properties)) = raw_data_contract["documents"]
            ["indexedDocument"]["indices"][0]
            .get_mut("properties")
            .unwrap()
        {
            properties.push(platform_value!({"objectProperty" : "asc" }))
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let basic_error = get_basic_error(error);

        assert_eq!(1013, basic_error.code());

        match basic_error {
            BasicError::InvalidIndexPropertyTypeError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
                assert_eq!(err.property_name(), "objectProperty".to_string());
                assert_eq!(err.property_type(), "object".to_string());
            }
            _ => panic!(
                "Expected InvalidIndexPropertyTypeError, got {}",
                basic_error
            ),
        }
    }

    #[test]
    fn index_property_should_not_point_to_array() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedArray"] = platform_value!({
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
        let basic_error = get_basic_error(error);

        assert_eq!(1013, basic_error.code());

        match basic_error {
            BasicError::InvalidIndexPropertyTypeError(err) => {
                assert_eq!(err.document_type(), "indexedArray".to_string());
                assert_eq!(err.property_name(), "mentions".to_string());
                assert_eq!(err.property_type(), "array".to_string());
            }
            _ => panic!(
                "Expected InvalidIndexPropertyTypeError, got {}",
                basic_error
            ),
        }
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
        indexed_document_definition["properties"]["arrayProperty"] = platform_value!({
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
            .push(platform_value!("arrayProperty"))
            .expect("array should exist");
        let index_definition = &mut indexed_document_definition["indices"][0];
        index_definition["properties"]
            .push(platform_value!({ "arrayProperty" : "asc"}))
            .expect("properties of index should exist");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let error = result.errors.get(0).expect("the error should be present");
        let index_error = get_basic_error(error);

        assert_eq!(1013, index_error.code());

        match index_error {
            BasicError::InvalidIndexPropertyTypeError(err) => {
                assert_eq!(err.document_type(), "indexedDocument".to_string());
                assert_eq!(err.property_name(), "arrayProperty");
                assert_eq!(err.property_type(), "array".to_string());
            }
            _ => panic!(
                "Expected InvalidIndexPropertyTypeError, got {}",
                index_error
            ),
        }
    }

    #[test]
    fn should_have_valid_property_names() {
        let TestData {
            raw_data_contract,
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
            let mut cloned_data_contract = raw_data_contract.clone();
            cloned_data_contract["documents"]["indexedDocument"]["properties"][property_name] =
                platform_value!({"type" : "string", "maxLength" : 63});

            cloned_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"]
                .push(platform_value!({ property_name : "asc"}))
                .unwrap();

            cloned_data_contract["documents"]["indexedDocument"]["required"]
                .push(Value::Text(property_name.to_string()))
                .unwrap();

            let result = data_contract_validator
                .validate(&cloned_data_contract)
                .expect("validation result");
            assert!(result.is_valid());
        }
    }

    #[test]
    fn should_return_invalid_result_if_property_has_invalid_format() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();
        let invalid_names = ["a.", ".a"];

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
            "type": "object",
            "properties": {
              "a": {
                "type": "object",
                "properties": {
                  "property": {
                    "type": "string",
                    "maxLength": 63,
                  },
                },
                "additionalProperties": false,
              },
            },
            "indices": [
              {
                "name": "index1",
                "properties": [],
                "unique": true,
              },
            ],
            "additionalProperties": false,
        });

        for invalid_name in invalid_names {
            let mut cloned_data_contract = raw_data_contract.clone();
            cloned_data_contract["documents"]["indexedDocument"]["indices"][0]["properties"]
                .push(platform_value!({ invalid_name : "asc"}))
                .unwrap();
            let result = data_contract_validator
                .validate(&cloned_data_contract)
                .expect("should return validation result");

            let index_error = get_basic_error(&result.errors[0]);

            assert_eq!(1016, index_error.code());

            match index_error {
                BasicError::UndefinedIndexPropertyError(err) => {
                    assert_eq!(err.property_name(), invalid_name.to_string());
                }
                _ => panic!("Expected UndefinedIndexPropertyError, got {}", index_error),
            }
        }
    }

    // As https://github.com/dashevo/platform/pull/435 suggests, the `desc` ordering is disabled
    // temporarily until reverse ordering become implemented.
    #[test]
    fn index_with_desc_order_is_disallowed() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        let index_definition = platform_value!({
            "name" : "index_1",
            "properties" : [
                { "$id"  : "desc"},
            ]
        });

        if let Some(Value::Array(ref mut indices)) = raw_data_contract["documents"]
            ["indexedDocument"]
            .get_mut("indices")
            .unwrap()
        {
            indices.push(index_definition)
        }

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/indexedDocument/indices/6/properties/0/$id",
            schema_error.instance_path().to_string()
        );
        assert_eq!("enum", schema_error.keyword());
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
            platform_value!("definitely not a number");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/signatureSecurityLevelRequirement",
            schema_error.instance_path().to_string()
        );
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn signature_level_requirement_should_be_one_of_available_values() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"]["signatureSecurityLevelRequirement"] =
            platform_value!(199);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert_eq!(
            "/documents/indexedDocument/signatureSecurityLevelRequirement",
            schema_error.instance_path().to_string()
        );
        assert_eq!("enum", schema_error.keyword());
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

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn dependent_required_should_be_object() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn dependent_required_should_have_array_value() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn dependent_required_should_have_array_of_strings() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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
        assert_eq!("type", schema_error.keyword());
    }

    #[test]
    fn dependent_required_should_have_array_of_unique_strings() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["indexedDocument"] = platform_value!({
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
        assert_eq!("uniqueItems", schema_error.keyword());
    }
}

#[test]
fn should_return_invalid_result_with_circular_ref_pointer() {
    let TestData {
        mut raw_data_contract,
        data_contract_validator,
        ..
    } = setup_test();

    raw_data_contract["$defs"]["object"] = platform_value!({ "$ref" : "#/$defs/object"});

    let result = data_contract_validator
        .validate(&raw_data_contract)
        .expect("validation result should be returned");
    let validation_error = result
        .errors
        .get(0)
        .expect("the validation error should exist");
    let basic_error = get_basic_error(validation_error);

    assert_eq!(1014, validation_error.code());
    match basic_error {
        BasicError::InvalidJsonSchemaRefError(err) => {
            assert_eq!(
                err.message(),
                "the ref '#/$defs/object' contains cycles".to_string()
            );
        }
        _ => panic!("Expected InvalidJsonSchemaRefError, got {}", basic_error),
    }
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
    let index_error = get_basic_error(validation_error);

    assert_eq!(1012, index_error.code());

    match index_error {
        BasicError::InvalidIndexedPropertyConstraintError(err) => {
            assert_eq!(err.property_name(), "firstName".to_string());
            assert_eq!(err.constraint_name(), "maxLength".to_string());
            assert_eq!(err.reason(), "should be less or equal than 63".to_string());
        }
        _ => panic!(
            "Expected InvalidIndexedPropertyConstraintError, got {}",
            index_error
        ),
    }
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
        let index_error = get_basic_error(validation_error);

        assert_eq!(1012, index_error.code());

        match index_error {
            BasicError::InvalidIndexedPropertyConstraintError(err) => {
                assert_eq!(err.property_name(), "byteArrayField".to_string());
                assert_eq!(err.constraint_name(), "maxItems".to_string());
                assert_eq!(err.reason(), "should be less or equal 255".to_string());
            }
            _ => panic!(
                "Expected InvalidIndexedPropertyConstraintError, got {}",
                index_error
            ),
        }
    }

    #[test]
    fn should_return_invalid_result_if_indexed_byte_array_property_have_to_big_max_items() {
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = setup_test();

        raw_data_contract["documents"]["withByteArrays"]["properties"]["byteArrayField"]
            ["maxItems"] = platform_value!(8192);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        let validation_error = result
            .errors
            .get(0)
            .expect("the validation error should exist");
        let index_error = get_basic_error(validation_error);

        assert_eq!(1012, index_error.code());
        match index_error {
            BasicError::InvalidIndexedPropertyConstraintError(err) => {
                assert_eq!(err.property_name(), "byteArrayField".to_string());
                assert_eq!(err.constraint_name(), "maxItems".to_string());
                assert_eq!(err.reason(), "should be less or equal 255".to_string());
            }
            _ => panic!(
                "Expected InvalidIndexedPropertyConstraintError, got {}",
                index_error
            ),
        }
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
