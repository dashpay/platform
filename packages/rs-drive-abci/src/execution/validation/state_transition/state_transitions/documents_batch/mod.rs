mod data_triggers;
mod identity_and_signatures;
mod state;
mod structure;

use dpp::identity::PartialIdentity;
use dpp::prelude::*;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition_action::StateTransitionAction;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::documents_batch::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::documents_batch::state::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::documents_batch::structure::v0::StateTransitionStructureValidationV0;

use crate::execution::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for DocumentsBatchTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform, tx),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionValidationV0 for DocumentsBatchTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        protocol_version: u32,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .structure
        {
            0 => self.validate_structure_v0(drive, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        protocol_version: u32,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .identity_signatures
        {
            0 => self.validate_identity_and_signatures_v0(drive, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: validate_identity_and_signatures".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
