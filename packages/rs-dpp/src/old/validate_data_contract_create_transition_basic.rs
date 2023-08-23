use std::sync::Arc;

use anyhow::anyhow;
use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::consensus::basic::data_contract::InvalidDataContractIdError;
use crate::consensus::basic::decode::ProtocolVersionParsingError;
use crate::{
    consensus::basic::BasicError,
    data_contract::property_names as data_contract_property_names,
    data_contract::{generate_data_contract_id, state_transition::property_names},
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    validation::{
        DataValidator, DataValidatorWithContext, JsonSchemaValidator,
        SimpleConsensusValidationResult,
    },
    version::ProtocolVersionValidator,
    ProtocolError,
};
use crate::mocks::JsonSchemaValidator;

lazy_static! {
    pub static ref DATA_CONTRACT_CREATE_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../../schema/data_contract/v0/stateTransition/dataContractCreate.json"
    ))
    .unwrap();
    pub static ref DATA_CONTRACT_CREATE_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(DATA_CONTRACT_CREATE_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

pub struct DataContractCreateTransitionBasicValidator {
    json_schema_validator: JsonSchemaValidator,
    protocol_validator: Arc<ProtocolVersionValidator>,
}

impl DataContractCreateTransitionBasicValidator {
    pub fn new(protocol_validator: Arc<ProtocolVersionValidator>) -> Result<Self, ProtocolError> {
        let json_schema_validator = JsonSchemaValidator::new(DATA_CONTRACT_CREATE_SCHEMA.clone())
            .map_err(|e| {
            anyhow!("cannot create instance of json validator for Data Contract Create schema: {e}")
        })?;

        Ok(Self {
            protocol_validator,
            json_schema_validator,
        })
    }
}

impl DataValidatorWithContext for DataContractCreateTransitionBasicValidator {
    type Item = Value;
    fn validate(
        &self,
        data: &Self::Item,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        validate_data_contract_create_transition_basic(
            &self.json_schema_validator,
            self.protocol_validator.as_ref(),
            &self.data_contract_validator,
            data,
            execution_context,
        )
    }
}

fn validate_data_contract_create_transition_basic(
    json_schema_validator: &impl DataValidator<Item = JsonValue>,
    protocol_validator: &impl DataValidator<Item = u32>,
    data_contract_validator: &impl DataValidator<Item = Value>,
    raw_state_transition: &Value,
    _execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let result = json_schema_validator.validate(
        &raw_state_transition
            .try_to_validating_json()
            .map_err(ProtocolError::ValueError)?,
    )?;
    if !result.is_valid() {
        return Ok(result);
    }

    let state_transition_version = match raw_state_transition
        .get_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
        .map_err(ProtocolError::ValueError)
    {
        Ok(v) => v,
        Err(parsing_error) => {
            return Ok(SimpleConsensusValidationResult::new_with_errors(vec![
                ProtocolVersionParsingError::new(parsing_error.to_string()).into(),
            ]))
        }
    };

    let result = protocol_validator.validate(&state_transition_version)?;
    if !result.is_valid() {
        return Ok(result);
    }

    let raw_data_contract = raw_state_transition.get_value(property_names::DATA_CONTRACT)?;

    // Validate Data Contract
    let result = data_contract_validator.validate(raw_data_contract)?;
    if !result.is_valid() {
        return Ok(result);
    }
    let owner_id = raw_data_contract.get_bytes(data_contract_property_names::OWNER_ID)?;
    let entropy = raw_state_transition.get_bytes(property_names::ENTROPY)?;
    let raw_data_contract_id = raw_data_contract.get_bytes(data_contract_property_names::ID)?;

    // Validate Data Contract ID
    let generated_id = generate_data_contract_id(owner_id, entropy);

    let mut validation_result = SimpleConsensusValidationResult::default();
    if generated_id.as_slice() != raw_data_contract_id {
        validation_result.add_error(BasicError::InvalidDataContractIdError(
            InvalidDataContractIdError::new(generated_id.to_vec(), raw_data_contract_id),
        ))
    }

    Ok(validation_result)
}
