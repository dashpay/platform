mod identity_and_signatures;
mod state;
mod structure;

use dpp::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

use dpp::identity::PartialIdentity;
use dpp::{
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_credit_transfer::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::identity_credit_transfer::state::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_credit_transfer::structure::v0::StateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;

impl StateTransitionActionTransformerV0 for IdentityCreditTransferTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0()
    }
}

impl StateTransitionValidationV0 for IdentityCreditTransferTransition {
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
        drive: &Drive,
        _protocol_version: u32,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        //todo: use protocol version to determine validation
        self.validate_identity_and_signatures_v0(drive, tx)
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
