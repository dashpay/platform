mod identity_and_signatures;
mod state;
mod structure;

use crate::error::Error;

use crate::execution::validation::state_transition::identity_create::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::identity_create::state::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_create::structure::v0::StateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransitionAction;
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

impl StateTransitionActionTransformerV0 for IdentityCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0(platform)
    }
}

impl StateTransitionValidationV0 for IdentityCreateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _protocol_version: u32,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        //todo: use protocol version to determine validation
        self.validate_structure_v0()
    }

    fn validate_identity_and_signatures(
        &self,
        _drive: &Drive,
        _protocol_version: u32,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        //todo: use protocol version to determine validation
        self.validate_identity_and_signatures_v0()
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
