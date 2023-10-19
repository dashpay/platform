use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError,
};
use dpp::consensus::basic::BasicError;

use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::AssetLockProved;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::AssetLockProofStateValidation;
use crate::execution::validation::state_transition::common::asset_lock::transaction::fetch_asset_lock_transaction_output_sync::fetch_asset_lock_transaction_output_sync;

pub(in crate::execution::validation::state_transition::state_transitions::identity_top_up) trait IdentityTopUpStateTransitionStateValidationV0
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityTopUpStateTransitionStateValidationV0 for IdentityTopUpTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        validation_result.merge(self.asset_lock_proof().validate_state(
            platform,
            tx,
            platform_version,
        )?);

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        self.transform_into_action_v0(platform, platform_version)
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let tx_out_validation = fetch_asset_lock_transaction_output_sync(
            platform.core_rpc,
            self.asset_lock_proof(),
            platform_version,
        )?;

        if !tx_out_validation.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                tx_out_validation.errors,
            ));
        }

        let tx_out = tx_out_validation.into_data()?;
        match IdentityTopUpTransitionAction::try_from_borrowed(self, &tx_out) {
            Ok(action) => {
                validation_result.set_data(action.into());
            }
            Err(error) => {
                validation_result.add_error(error);
            }
        }

        Ok(validation_result)
    }
}
