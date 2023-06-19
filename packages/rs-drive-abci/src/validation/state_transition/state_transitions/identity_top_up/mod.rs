mod identity_and_signatures;
mod state;
mod structure;

use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::consensus::ConsensusError;
use dpp::dashcore::OutPoint;
use dpp::identity::state_transition::identity_topup_transition::validation::basic::IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use dpp::identity::PartialIdentity;
use dpp::platform_value::Bytes36;
use dpp::{
    identity::state_transition::identity_topup_transition::IdentityTopUpTransition,
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;
use crate::execution::asset_lock::fetch_tx_out::FetchAssetLockProofTxOut;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::identity_top_up::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::validation::state_transition::identity_top_up::state::v0::StateTransitionStateValidationV0;
use crate::validation::state_transition::identity_top_up::structure::v0::StateTransitionStructureValidationV0;
use crate::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::validation::state_transition::transformer::StateTransitionActionTransformerV0;

impl StateTransitionActionTransformerV0 for IdentityTopUpTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0(platform)
    }
}

impl StateTransitionValidationV0 for IdentityTopUpTransition {
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
