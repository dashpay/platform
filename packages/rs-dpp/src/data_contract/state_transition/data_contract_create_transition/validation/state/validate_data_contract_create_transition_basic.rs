use std::sync::Arc;

use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::Value;

use crate::{
    consensus::basic::BasicError,
    data_contract::{generate_data_contract_id, state_transition::property_names},
    data_contract::{
        property_names as data_contract_property_names,
        validation::data_contract_validator::DataContractValidator,
    },
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    util::json_value::JsonValueExt,
    validation::{
        DataValidator, DataValidatorWithContext, JsonSchemaValidator, SimpleValidationResult,
    },
    version::ProtocolVersionValidator,
    ProtocolError,
};

lazy_static! {
    static ref DATA_CONTRACT_CREATE_SCHEMA: Value = serde_json::from_str(include_str!(
        "../../../../../schema/data_contract/stateTransition/dataContractCreate.json"
    ))
    .unwrap();
}

pub struct DataContractCreateTransitionBasicValidator {
    json_schema_validator: JsonSchemaValidator,
    protocol_validator: Arc<ProtocolVersionValidator>,
    data_contract_validator: DataContractValidator,
}

impl DataContractCreateTransitionBasicValidator {
    pub fn new(protocol_validator: Arc<ProtocolVersionValidator>) -> Result<Self, ProtocolError> {
        let data_contract_validator = DataContractValidator::new(protocol_validator.clone());
        let json_schema_validator = JsonSchemaValidator::new(DATA_CONTRACT_CREATE_SCHEMA.clone())
            .map_err(|e| {
            anyhow!("cannot create instance of json validator for Data Contract Create schema: {e}")
        })?;

        Ok(Self {
            protocol_validator,
            data_contract_validator,
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
    ) -> Result<SimpleValidationResult, ProtocolError> {
        validate_data_contract_create_transition_basic(
            &self.json_schema_validator,
            self.protocol_validator.as_ref(),
            &self.data_contract_validator,
            data,
            execution_context,
        )
    }
}

pub fn validate_data_contract_create_transition_basic(
    json_schema_validator: &impl DataValidator<Item = Value>,
    protocol_validator: &impl DataValidator<Item = u32>,
    data_contract_validator: &impl DataValidator<Item = Value>,
    raw_state_transition: &Value,
    _execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleValidationResult, ProtocolError> {
    let result = json_schema_validator.validate(raw_state_transition)?;
    if !result.is_valid() {
        return Ok(result);
    }

    let protocol_version = raw_state_transition.get_u64(property_names::PROTOCOL_VERSION)? as u32;
    let result = protocol_validator.validate(&protocol_version)?;
    if !result.is_valid() {
        return Ok(result);
    }

    let raw_data_contract = raw_state_transition.get_value(property_names::DATA_CONTRACT)?;

    // Validate Data Contract
    let result = data_contract_validator.validate(raw_state_transition)?;
    if !result.is_valid() {
        return Ok(result);
    }

    let owner_id = raw_data_contract.get_bytes(data_contract_property_names::OWNER_ID)?;
    let entropy = raw_data_contract.get_bytes(data_contract_property_names::ENTROPY)?;
    let raw_data_contract_id = raw_data_contract.get_bytes(data_contract_property_names::ID)?;

    // Validate Data Contract ID
    let generated_id = generate_data_contract_id(owner_id, entropy);

    let mut validation_result = SimpleValidationResult::default();
    if generated_id != raw_data_contract_id {
        validation_result.add_error(BasicError::InvalidDataContractId {
            expected_id: generated_id,
            invalid_id: raw_data_contract_id,
        })
    }

    Ok(validation_result)
}
