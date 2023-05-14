use crate::asset_lock::fetch_tx_out::FetchAssetLockProofTxOut;
use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::identity::state_transition::identity_topup_transition::validation::basic::IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use dpp::identity::PartialIdentity;
use dpp::{
    identity::state_transition::identity_topup_transition::IdentityTopUpTransition,
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityTopUpTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        active_protocol_version: u32,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_protocol_version(self.protocol_version))
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();

        let maybe_partial_identity =
            drive.fetch_identity_with_balance(self.identity_id.to_buffer(), tx)?;

        let partial_identity = match maybe_partial_identity {
            None => {
                //slightly weird to have a signature error, maybe should be changed
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(self.identity_id),
                ));
                return Ok(validation_result);
            }
            Some(pk) => pk,
        };

        validation_result.set_data(Some(partial_identity));
        Ok(validation_result)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        self.transform_into_action(platform, tx)
    }

    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let tx_out_validation = self
            .asset_lock_proof
            .fetch_asset_lock_transaction_output_sync(platform.core_rpc)?;
        if !tx_out_validation.is_valid_with_data() {
            return Ok(ConsensusValidationResult::new_with_errors(
                tx_out_validation.errors,
            ));
        }

        let tx_out = tx_out_validation.into_data()?;
        validation_result.set_data(
            IdentityTopUpTransitionAction::from_borrowed(self, tx_out.value * 1000).into(),
        );
        Ok(validation_result)
    }
}
