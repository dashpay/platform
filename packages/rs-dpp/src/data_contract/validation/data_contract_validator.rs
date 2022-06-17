use crate::{
    consensus::basic::{BasicError, IndexError},
    data_contract::{
        enrich_data_contract_with_base_schema::enrich_data_contract_with_base_schema,
        enrich_data_contract_with_base_schema::PREFIX_BYTE_0,
        get_property_definition_by_path::get_property_definition_by_path, DataContract,
    },
    util::{
        json_schema::{Index, JsonSchemaExt},
        json_value::JsonValueExt,
    },
    validation::{JsonSchemaValidator, ValidationResult},
    version::ProtocolVersionValidator,
    ProtocolError,
};
use anyhow::anyhow;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::trace;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, sync::Arc};

use super::{
    validate_data_contract_max_depth::validate_data_contract_max_depth,
    validate_data_contract_patterns::validate_data_contract_patterns,
};

pub const MAX_INDEXED_STRING_PROPERTY_LENGTH: usize = 63;
pub const UNIQUE_INDEX_LIMIT: usize = 3;
pub const NOT_ALLOWED_SYSTEM_PROPERTIES: [&str; 1] = ["$id"];
pub const ALLOWED_INDEX_SYSTEM_PROPERTIES: [&str; 3] = ["$ownerId", "$createdAt", "$updatedAt"];
pub const MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH: usize = 255;
pub const MAX_INDEXED_ARRAY_ITEMS: usize = 1024;

lazy_static! {
        // TODO  the base_document_schema should be declared in one place
    static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../schema/document/documentBase.json")).unwrap();
}

pub struct DataContractValidator {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
}

impl DataContractValidator {
    pub fn new(protocol_version_validator: Arc<ProtocolVersionValidator>) -> DataContractValidator {
        Self {
            protocol_version_validator,
        }
    }

    pub fn validate(
        &self,
        raw_data_contract: &JsonValue,
    ) -> Result<ValidationResult, ProtocolError> {
        let mut result = ValidationResult::default();

        trace!("validating against data contract meta validator");
        result.merge(JsonSchemaValidator::validate_data_contract_schema(
            raw_data_contract,
        )?);
        if !result.is_valid() {
            return Ok(result);
        }

        trace!("validating by protocol protocol version validator");
        result.merge(
            self.protocol_version_validator.validate(
                raw_data_contract
                    .get_u64("protocolVersion")
                    .map_err(|_| anyhow!("protocolVersion isn't unsigned integer"))?
                    as u32,
            )?,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        trace!("validating data contract max depth");
        result.merge(validate_data_contract_max_depth(raw_data_contract));
        if !result.is_valid() {
            return Ok(result);
        }

        trace!("validating data contract patterns");
        result.merge(validate_data_contract_patterns(raw_data_contract));
        if !result.is_valid() {
            return Ok(result);
        }

        let data_contract = DataContract::from_raw_object(raw_data_contract.clone())?;
        let enriched_data_contract = enrich_data_contract_with_base_schema(
            &data_contract,
            &BASE_DOCUMENT_SCHEMA,
            PREFIX_BYTE_0,
            &[],
        )?;

        trace!("validating the documents");
        for (document_type, document_schema) in enriched_data_contract.documents.iter() {
            trace!("validating document schema '{}'", document_type);
            let json_schema_validation_result =
                JsonSchemaValidator::validate_schema(document_schema);
            result.merge(json_schema_validation_result);
        }
        if !result.is_valid() {
            return Ok(result);
        }

        trace!("indices validation");
        for (document_type, document_schema) in enriched_data_contract
            .documents
            .iter()
            .filter(|(_, value)| value.get("indices").is_some())
        {
            trace!("validating indices in {}", document_type);
            let mut indices_fingerprints: Vec<String> = vec![];
            let indices = document_schema.get_indices()?;

            trace!("\t validating duplicates");
            let validation_result = validate_index_duplicates(&indices, document_type);
            result.merge(validation_result);

            trace!("\t validating uniqueness");
            let validation_result = validate_max_unique_indices(&indices, document_type);
            result.merge(validation_result);

            trace!("\t validating definitions");
            for index_definition in indices.iter() {
                let validation_result = validate_no_system_indices(index_definition, document_type);
                result.merge(validation_result);

                let user_defined_properties = index_definition
                    .properties
                    .iter()
                    .flatten()
                    .map(|property| property.0)
                    .filter(|property_name| {
                        ALLOWED_INDEX_SYSTEM_PROPERTIES.contains(&property_name.as_str())
                    });

                let property_definition_entities: HashMap<&String, Option<&JsonValue>> =
                    user_defined_properties
                        .map(|property_name| {
                            (
                                property_name,
                                get_property_definition_by_path(document_schema, property_name)
                                    .ok(),
                            )
                        })
                        .collect();

                let validation_result = validate_not_defined_properties(
                    &property_definition_entities,
                    index_definition,
                    document_type,
                );
                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    // Skip further validation if there are undefined properties
                    return Ok(result);
                }

                // Validation of property defs
                for (property_name, maybe_property_definition) in property_definition_entities {
                    // we are allowed to use unwrap as we return if some of the properties definitions is None
                    let property_definition = maybe_property_definition.unwrap();
                    let is_byte_array = property_definition.is_type_of_byte_array();
                    let mut invalid_property_type: String = "".to_string();

                    if property_definition.is_type_of_object() {
                        invalid_property_type = "object".to_string()
                    }

                    // Validate arrays contain scalar values or have the same types
                    // https://github.com/dashevo/platform/blob/ab6391f4b47a970c733e7b81115b44329fbdf993/packages/js-dpp/lib/dataContract/validation/validateDataContractFactory.js#L210
                    if property_definition.is_type_of_array() && !is_byte_array {
                        // const isInvalidPrefixItems = prefixItems
                        //   && (
                        // prefixItems.some((prefixItem) =>
                        // prefixItem.type === 'object' || prefixItem.type === 'array')
                        //     || !prefixItems.every((prefixItem) => prefixItem.type === prefixItems[0].type)
                        //   );
                        //
                        // const isInvalidItemTypes = items.type === 'object' || items.type === 'array';
                        //
                        // if (isInvalidPrefixItems || isInvalidItemTypes) {
                        //   invalidPropertyType = 'array';
                        // }
                    }

                    if !invalid_property_type.is_empty() {
                        result.add_error(BasicError::IndexError(
                            IndexError::InvalidIndexPropertyTypError {
                                document_type: document_type.clone(),
                                index_definition: index_definition.clone(),
                                property_name: property_name.clone(),
                                property_type: invalid_property_type.clone(),
                            },
                        ));
                    }

                    // https://github.com/dashevo/platform/blob/ab6391f4b47a970c733e7b81115b44329fbdf993/packages/js-dpp/lib/dataContract/validation/validateDataContractFactory.js#L236
                    // Validate sting length inside arrays
                    // if (!invalidPropertyType && propertyType === 'array' && !isByteArray) {
                    //   const isInvalidPrefixItems = prefixItems && prefixItems.some((prefixItem) => (
                    //     prefixItem.type === 'string'
                    //     && (
                    // !prefixItem.maxLength || prefixItem.maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH
                    //     )
                    //   ));
                    //
                    //   const isInvalidItemTypes = items.type === 'string' && (
                    //     !items.maxLength || items.maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH
                    //   );
                    //
                    //   if (isInvalidPrefixItems || isInvalidItemTypes) {
                    //     result.addError(
                    //       new InvalidIndexedPropertyConstraintError(
                    //         documentType,
                    //         indexDefinition,
                    //         propertyName,
                    //         'maxLength',
                    //         `should be less or equal ${MAX_INDEXED_STRING_PROPERTY_LENGTH}`,
                    //       ),
                    //     );
                    //   }
                    // }
                    //

                    if invalid_property_type.is_empty() && property_definition.is_type_of_array() {
                        let max_items = property_definition.get_u64("maxItems").ok();
                        let max_limit = if is_byte_array {
                            MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH
                        } else {
                            MAX_INDEXED_ARRAY_ITEMS
                        };

                        if max_items.is_none() || max_items.unwrap() > max_limit as u64 {
                            result.add_error(BasicError::IndexError(
                                IndexError::InvalidIndexedPropertyConstraintError {
                                    document_type: document_type.clone(),
                                    index_definition: index_definition.clone(),
                                    property_name: property_name.clone(),
                                    constraint_name: String::from("maxLength"),
                                    reason: format!("should be less or equal {}", max_limit),
                                },
                            ));
                        }

                        if property_definition.is_type_of_string() {
                            let max_length = property_definition.get_u64("maxLength").ok();

                            if max_length.is_none()
                                || max_length.unwrap() > MAX_INDEXED_STRING_PROPERTY_LENGTH as u64
                            {
                                result.add_error(BasicError::IndexError(
                                    IndexError::InvalidIndexedPropertyConstraintError {
                                        document_type: document_type.clone(),
                                        index_definition: index_definition.clone(),
                                        property_name: property_name.clone(),
                                        constraint_name: String::from("maxLength"),
                                        reason: format!(
                                            "should be less or equal {}",
                                            MAX_INDEXED_STRING_PROPERTY_LENGTH
                                        ),
                                    },
                                ))
                            }
                        }

                        // Make sure that compound unique indices contain all fields
                        if index_definition.properties.len() > 1 {
                            let required_fields = document_schema
                                .get_schema_required_fields()
                                .unwrap_or_default();
                            let all_are_required = index_definition
                                .properties
                                .iter()
                                .flatten()
                                .map(|(field, _)| field)
                                .all(|field| required_fields.contains(&field.as_str()));

                            let all_are_not_required = index_definition
                                .properties
                                .iter()
                                .flatten()
                                .map(|(field, _)| field)
                                .all(|field| !required_fields.contains(&field.as_str()));

                            if !all_are_required && !all_are_not_required {
                                result.add_error(BasicError::IndexError(
                                    IndexError::InvalidCompoundIndexError {
                                        document_type: document_type.clone(),
                                        index_definition: index_definition.clone(),
                                    },
                                ));
                            }

                            // Ensure index definition uniqueness
                            let indices_fingerprint =
                                serde_json::to_string(&index_definition.properties)?;
                            if indices_fingerprints.contains(&indices_fingerprint) {
                                result.add_error(BasicError::IndexError(
                                    IndexError::DuplicateIndexError {
                                        document_type: document_type.clone(),
                                        index_definition: index_definition.clone(),
                                    },
                                ));
                            }
                            indices_fingerprints.push(indices_fingerprint)
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// checks if properties defined in indices are existing in the contract
fn validate_not_defined_properties(
    properties: &HashMap<&String, Option<&JsonValue>>,
    index_definition: &Index,
    document_type: &str,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    for (property_name, definition) in properties {
        if definition.is_none() {
            result.add_error(BasicError::IndexError(
                IndexError::UndefinedIndexPropertyError {
                    document_type: document_type.to_owned(),
                    index_definition: index_definition.clone(),
                    property_name: property_name.to_owned().to_owned(),
                },
            ))
        }
    }
    result
}

/// checks if names of indices are not duplicated
fn validate_index_duplicates(indices: &[Index], document_type: &str) -> ValidationResult {
    let mut result = ValidationResult::default();
    for duplicate_index in indices.iter().map(|i| &i.name).duplicates() {
        result.add_error(BasicError::DuplicateIndexNameError {
            document_type: document_type.to_owned(),
            duplicate_index_name: duplicate_index.to_owned(),
        })
    }
    result
}

/// checks the limit of unique indexes defined in the data contract
fn validate_max_unique_indices(indices: &[Index], document_type: &str) -> ValidationResult {
    let mut result = ValidationResult::default();
    if indices.iter().filter(|i| i.unique).count() > UNIQUE_INDEX_LIMIT {
        result.add_error(BasicError::IndexError(
            IndexError::UniqueIndicesLimitReachedError {
                document_type: document_type.to_owned(),
                index_limit: UNIQUE_INDEX_LIMIT,
            },
        ))
    }

    result
}

/// checks if the system properties are not included in index definition
fn validate_no_system_indices(index_definition: &Index, document_type: &str) -> ValidationResult {
    let mut result = ValidationResult::default();

    for (property_name, _) in index_definition.properties.iter().flatten() {
        if NOT_ALLOWED_SYSTEM_PROPERTIES.contains(&property_name.as_str()) {
            result.add_error(BasicError::IndexError(
                IndexError::SystemPropertyIndexAlreadyPresentError {
                    property_name: property_name.to_owned(),
                    document_type: document_type.to_owned(),
                    index_definition: index_definition.clone(),
                },
            ));
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        consensus::basic::JsonSchemaError,
        tests::fixtures::get_data_contract_fixture,
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
        Convertible,
    };
    use jsonschema::error::{TypeKind, ValidationErrorKind};
    use serde_json::json;
    use test_case::test_case;

    struct TestData {
        data_contract_validator: DataContractValidator,
        data_contract: DataContract,
        raw_data_contract: JsonValue,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None);
        let raw_data_contract = data_contract.to_object().unwrap();

        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );

        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));

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

    #[test_case("protocolVersion")]
    #[test_case("$schema")]
    #[test_case("$id")]
    #[test_case("documents")]
    #[test_case("ownerId")]
    // #[test_case("$defs")]
    fn property_should_be_present(property: &str) {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        raw_data_contract
            .remove(property)
            .unwrap_or_else(|_| panic!("the {} should exist and be removed", "protocolVersion"));

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == property
        ));
    }

    #[test]
    fn protocol_version_should_be_integer() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        raw_data_contract["protocolVersion"] = json!("1");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/protocolVersion", schema_error.instance_path().to_string());
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn protocol_version_should_be_valid() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        raw_data_contract["protocolVersion"] = json!(-1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect_err("protocol error should be returned");
        trace!("The validation result is: {:#?}", result);

        assert!(matches!(result, ProtocolError::Error(..)))
    }

    #[test]
    fn schema_should_be_string() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        raw_data_contract["$schema"] = json!(1);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$schema", schema_error.instance_path().to_string());
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test_case("ownerId")]
    #[test_case("$id")]
    fn owner_id_should_be_byte_array(property_name: &str) {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        let array = ["string"; 32];
        raw_data_contract[property_name] = json!(array);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        let array = [0u8; 31];
        raw_data_contract[property_name] = json!(array);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        let mut too_long_id = Vec::new();
        too_long_id.resize(33, 0u8);
        raw_data_contract[property_name] = json!(too_long_id);

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxItems"), schema_error.keyword(),);
    }

    #[test]
    fn schema_should_be_url() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        raw_data_contract["$schema"] = json!("wrong");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/$schema", schema_error.instance_path().to_string());
        assert_eq!(Some("const"), schema_error.keyword(),);
    }

    #[test]
    fn documents_should_be_object() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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
    fn indices_should_be_array() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

        raw_data_contract["documents"]["indexedDocument"]["indices"] =
            json!("definitely not an array");

        let result = data_contract_validator
            .validate(&raw_data_contract)
            .expect("validation result should be returned");
        trace!("The validation result is: {:#?}", result);

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            "/documents/indexedDocument/indices",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn documents_schema_with_invalid_property_names_should_return_error() {
        init();
        let TestData {
            mut raw_data_contract,
            data_contract_validator,
            ..
        } = get_test_data();

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

        // assert_eq!(3, result.errors.len());
        assert_eq!(
            "/documents/niceDocument/properties",
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("pattern"), schema_error.keyword(),);
        println!("the reuslt is {:#?}", result);
    }

    fn get_schema_error(result: &ValidationResult, number: usize) -> &JsonSchemaError {
        result
            .errors
            .get(number)
            .expect("the error should be returned in validation result")
            .json_schema_error()
            .expect("the error should be json schema error")
    }
}
