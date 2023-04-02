use std::sync::Arc;

use dpp::{
    consensus::basic::{data_contract::InvalidDataContractIdError, BasicError},
    data_contract::validation::data_contract_validator::DataContractValidator,
    validation::SimpleConsensusValidationResult,
};
use dpp::{
    data_contract::{
        generate_data_contract_id,
        state_transition::data_contract_create_transition::{
            validation::state::validate_data_contract_create_transition_basic::DATA_CONTRACT_CREATE_SCHEMA,
            DataContractCreateTransition,
        },
    },
    validation::JsonSchemaValidator,
    Convertible,
};
use dpp::{prelude::ConsensusValidationResult, state_transition::StateTransitionConvert};
use dpp::{state_transition::StateTransitionAction, version::ProtocolVersionValidator};
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;
use crate::validation::state_transition::StateTransitionValidation;

impl StateTransitionValidation for DataContractCreateTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Reuse jsonschema validation on a whole state transition
        let json_schema_validator = JsonSchemaValidator::new(DATA_CONTRACT_CREATE_SCHEMA.clone())
            .expect("unable to compile jsonschema");
        let result = json_schema_validator
            .validate(
                &(self
                    .to_object(true)
                    .expect("data contract is serializable")
                    .try_into_validating_json()
                    .expect("TODO")),
            )
            .expect("TODO: how jsonschema validation will ever fail?");
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate protocol version
        let protocol_version_validator = ProtocolVersionValidator::default();
        let result = protocol_version_validator
            .validate(self.protocol_version)
            .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate data contract
        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator)); // ffs
        let result = data_contract_validator
            .validate(&(self.data_contract.clone().into_object().expect("TODO")))?;
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate data contract id
        let generated_id =
            generate_data_contract_id(self.data_contract.owner_id, self.data_contract.entropy);
        let mut validation_result = SimpleConsensusValidationResult::default();
        if generated_id != self.data_contract.id.as_ref() {
            validation_result.add_error(BasicError::InvalidDataContractIdError(
                InvalidDataContractIdError::new(
                    generated_id,
                    self.data_contract.id.as_ref().to_owned(),
                ), // TODO
            ))
        }

        Ok(validation_result)
    }

    fn validate_signature(&self, drive: &Drive) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
