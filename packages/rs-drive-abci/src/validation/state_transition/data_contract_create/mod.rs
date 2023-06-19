mod structure;
mod state;
mod identity_and_signatures;

use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::{
    consensus::basic::{data_contract::InvalidDataContractIdError, BasicError},
    data_contract::{
        state_transition::data_contract_create_transition::DataContractCreateTransitionAction,
    },
    validation::SimpleConsensusValidationResult,
};
use dpp::{
    data_contract::{
        generate_data_contract_id,
        state_transition::data_contract_create_transition::{
            DataContractCreateTransition,
        },
    },
};
use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::state_transition::StateTransitionAction;
use dpp::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DATA_CONTRACT_CREATE_SCHEMA_VALIDATOR;
use drive::grovedb::TransactionArg;
use drive::drive::Drive;

use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::data_contract_create::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::validation::state_transition::data_contract_create::state::v0::StateTransitionStateValidationV0;
use crate::validation::state_transition::data_contract_create::structure::v0::StateTransitionStructureValidationV0;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;
use crate::validation::state_transition::StateTransitionValidationV0;

use super::common::validate_schema;

impl StateTransitionValidationV0 for DataContractCreateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        protocol_version: u32,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        //todo: use protocol version to determine validation
        self.validate_structure_v0()
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        protocol_version: u32,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        //todo: use protocol version to determine validation
        self.validate_identity_and_signatures_v0(drive, transaction)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.validate_state_v0(platform, tx)
    }

    fn transform_into_action<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0::<C>()
    }
}
