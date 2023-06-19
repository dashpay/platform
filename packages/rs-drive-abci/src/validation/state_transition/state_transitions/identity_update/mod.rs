mod identity_and_signatures;
mod state;
mod structure;

use dpp::identity::PartialIdentity;
use dpp::{identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition, ProtocolError, state_transition::StateTransitionAction, validation::{ConsensusValidationResult, SimpleConsensusValidationResult}};
use dpp::block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window;
use dpp::consensus::state::identity::identity_public_key_disabled_at_window_violation_error::IdentityPublicKeyDisabledAtWindowViolationError;
use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use dpp::identity::state_transition::identity_update_transition::validate_identity_update_transition_basic::IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR;
use dpp::serialization_traits::PlatformMessageSignable;
use dpp::serialization_traits::Signable;
use drive::grovedb::TransactionArg;
use drive::drive::Drive;

use crate::error::execution::ExecutionError;
use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::identity_update::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::validation::state_transition::identity_update::state::v0::StateTransitionStateValidationV0;
use crate::validation::state_transition::identity_update::structure::v0::StateTransitionStructureValidationV0;
use crate::validation::state_transition::key_validation::{
    validate_identity_public_key_ids_dont_exist_in_state,
    validate_identity_public_key_ids_exist_in_state, validate_identity_public_keys_structure,
    validate_state_transition_identity_signature_v0,
    validate_unique_identity_public_key_hashes_state,
};
use crate::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::validation::state_transition::transformer::StateTransitionActionTransformerV0;

impl StateTransitionActionTransformerV0 for IdentityUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0()
    }
}

impl StateTransitionValidationV0 for IdentityUpdateTransition {
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
}
