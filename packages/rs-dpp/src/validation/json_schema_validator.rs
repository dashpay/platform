use crate::consensus::ConsensusError;
use crate::validation::ValidationResult;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};
use jsonschema::{JSONSchema, KeywordDefinition};
use lazy_static::lazy_static;
use serde_json::{json, Value};

lazy_static! {
    static ref DRAFT202012: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/schema.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_CORE: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/meta/core.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_APPLICATOR: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/meta/applicator.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_UNEVALUATED: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/meta/unevaluated.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_VALIDATION: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/meta/validation.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_META_DATA: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/meta/meta-data.json"
    ))
    .expect("Valid schema!");
    static ref DRAFT202012_FORMAT_ANNOTATION: serde_json::Value = serde_json::from_str(
        include_str!("../../schema/meta_schemas/draft2020-12/meta/format-annotation.json")
    )
    .expect("Valid schema!");
    static ref DRAFT202012_CONTENT: serde_json::Value = serde_json::from_str(include_str!(
        "../../schema/meta_schemas/draft2020-12/meta/content.json"
    ))
    .expect("Valid schema!");
    static ref DATA_CONTRACT: Value = serde_json::from_str::<Value>(include_str!(
        "../schema/data_contract/dataContractMeta.json"
    ))
    .unwrap();

    // Compiled version of data contract meta schema
    static ref DATA_CONTRACT_META_SCHEMA: JSONSchema = JSONSchema::options()
        .add_keyword(
                "byteArray",
                KeywordDefinition::Schema(json!({
                    "items": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 255,
                    },
                })),
            )
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
        .compile(&DATA_CONTRACT)
        .expect("Invalid data contract schema");
}

pub struct JsonSchemaValidator {
    raw_schema_json: Value,
    schema: Option<JSONSchema>,
}

impl JsonSchemaValidator {
    pub fn new(schema_json: Value) -> Result<Self, DashPlatformProtocolInitError> {
        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        // BYTE_ARRAY META SCHEMA
        // let schema_clone = &json_schema_validator.raw_schema_json.clone();
        // let res = byte_array_meta::validate(&schema_clone);
        //
        // match res {
        //     Ok(_) => {}
        //     Err(mut errors) => {
        //         return Err(DashPlatformProtocolInitError::from(errors.remove(0)));
        //     }
        // }
        // BYTE_ARRAY META SCHEMA END

        let json_schema = JSONSchema::options()
            .add_keyword(
                "byteArray",
                KeywordDefinition::Schema(json!({
                    "items": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 255,
                    },
                })),
            )
            .compile(&json_schema_validator.raw_schema_json)?;

        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }

    pub fn validate(&self, object: &Value) -> Result<ValidationResult, NonConsensusError> {
        // TODO: create better error messages
        let res = self
            .schema
            .as_ref()
            .ok_or_else(|| SerdeParsingError::new("Expected identity schema to be initialized"))?
            .validate(object);

        let mut validation_result = ValidationResult::new(None);

        match res {
            Ok(_) => Ok(validation_result),
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();
                validation_result.add_errors(errors);
                Ok(validation_result)
            }
        }
    }

    /// Uses predefined meta-schemas to validate data contract schema
    pub fn validate_data_contract_schema(
        data_contract_schema: &Value,
    ) -> Result<ValidationResult, NonConsensusError> {
        let mut validation_result = ValidationResult::new(None);
        let res = DATA_CONTRACT_META_SCHEMA.validate(data_contract_schema);

        match res {
            Ok(_) => Ok(validation_result),
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();
                validation_result.add_errors(errors);
                Ok(validation_result)
            }
        }
    }
}
