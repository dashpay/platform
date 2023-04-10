use std::sync::Arc;

use dpp::identity::PartialIdentity;
use dpp::platform_value::string_encoding::Encoding::Base58;
use dpp::prelude::ConsensusValidationResult;
use dpp::{
    consensus::basic::{data_contract::InvalidDataContractIdError, BasicError},
    data_contract::{
        state_transition::data_contract_create_transition::DataContractCreateTransitionAction,
        validation::data_contract_validator::DataContractValidator,
    },
    validation::SimpleConsensusValidationResult,
    StateError,
};
use dpp::{
    data_contract::{
        generate_data_contract_id,
        state_transition::data_contract_create_transition::{
            validation::state::validate_data_contract_create_transition_basic::DATA_CONTRACT_CREATE_SCHEMA,
            DataContractCreateTransition,
        },
    },
    Convertible,
};
use dpp::{state_transition::StateTransitionAction, version::ProtocolVersionValidator};
use dpp::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DATA_CONTRACT_CREATE_SCHEMA_VALIDATOR;
use dpp::identity::SecurityLevel::{CRITICAL, HIGH};
use drive::grovedb::TransactionArg;
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;
use crate::validation::state_transition::StateTransitionValidation;

use super::common::validate_schema;

impl StateTransitionValidation for DataContractCreateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&DATA_CONTRACT_CREATE_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        //todo: re-enable version validation
        // // Validate protocol version
        // let protocol_version_validator = ProtocolVersionValidator::default();
        // let result = protocol_version_validator
        //     .validate(self.protocol_version)
        //     .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        // if !result.is_valid() {
        //     return Ok(result);
        // }
        //
        // // Validate data contract
        // let data_contract_validator =
        //     DataContractValidator::new(Arc::new(protocol_version_validator)); // ffs
        // let result = data_contract_validator
        //     .validate(&(self.data_contract.to_cleaned_object().expect("TODO")))?;
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        // Validate data contract id
        let generated_id =
            generate_data_contract_id(self.data_contract.owner_id, self.data_contract.entropy);
        let mut validation_result = SimpleConsensusValidationResult::default();
        if generated_id != self.data_contract.id.as_ref() {
            validation_result.add_error(BasicError::InvalidDataContractIdError(
                InvalidDataContractIdError::new(
                    generated_id,
                    self.data_contract.id.as_ref().to_owned(),
                ),
            ))
        }

        Ok(validation_result)
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        Ok(
            validate_state_transition_identity_signature(drive, self, false, transaction)?
                .map(|partial_identity| Some(partial_identity)),
        )
    }

    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        platform: &'a PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        // Data contract shouldn't exist
        if drive
            .get_contract_with_fetch_info(self.data_contract.id.to_buffer(), None, false, tx)?
            .1
            .is_some()
        {
            Ok(ConsensusValidationResult::new_with_errors(vec![
                StateError::DataContractAlreadyPresentError {
                    data_contract_id: self.data_contract.id.to_owned(),
                }
                .into(),
            ]))
        } else {
            let action: StateTransitionAction =
                Into::<DataContractCreateTransitionAction>::into(self).into();
            Ok(action.into())
        }
    }
}
