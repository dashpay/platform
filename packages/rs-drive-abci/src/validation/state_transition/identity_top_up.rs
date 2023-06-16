use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::identity::state_transition::identity_topup_transition::validation::basic::IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use dpp::identity::PartialIdentity;
use dpp::{
    identity::state_transition::identity_topup_transition::IdentityTopUpTransition,
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::grovedb::TransactionArg;

use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::asset_lock::{
    fetch_asset_lock_transaction_output_sync, validate_asset_lock_proof_state,
    validate_asset_lock_proof_structure, validate_state_transition_signature_with_asset_lock_proof,
};
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::context::ValidationDataShareContext;

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityTopUpTransition {
    fn validate_structure<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _context: &mut ValidationDataShareContext,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_asset_lock_proof_structure(&self.asset_lock_proof)
    }

    fn validate_identity_and_signatures<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        context: &mut ValidationDataShareContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();

        let maybe_partial_identity = platform
            .drive
            .fetch_identity_with_balance(self.identity_id.to_buffer(), tx)?;

        let partial_identity = match maybe_partial_identity {
            None => {
                //slightly weird to have a signature error, maybe should be changed
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(self.identity_id),
                ));
                return Ok(validation_result);
            }
            Some(partial_identity) => partial_identity,
        };

        validation_result.set_data(Some(partial_identity));

        let signature_validation =
            validate_state_transition_signature_with_asset_lock_proof(self, platform)?;

        if !signature_validation.is_valid() {
            validation_result.merge(signature_validation);

            return Ok(validation_result);
        }

        context.asset_lock_output = Some(signature_validation.into_data()?);

        Ok(validation_result)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        context: &mut ValidationDataShareContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::default();

        let asset_lock_validation_result =
            validate_asset_lock_proof_state(&self.asset_lock_proof, platform, tx)?;

        if !asset_lock_validation_result.is_valid() {
            validation_result.merge(asset_lock_validation_result);

            return Ok(validation_result);
        }

        self.transform_into_action(platform, context, tx)
    }

    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        context: &ValidationDataShareContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let asset_lock_validation_result = context
            .asset_lock_output
            .as_ref()
            .map(|tx_out| Ok(ConsensusValidationResult::new_with_data(tx_out.clone())))
            .unwrap_or_else(|| {
                fetch_asset_lock_transaction_output_sync(platform.core_rpc, &self.asset_lock_proof)
            })?;

        if !asset_lock_validation_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                asset_lock_validation_result.errors,
            ));
        }

        let tx_out = asset_lock_validation_result.into_data()?;
        match IdentityTopUpTransitionAction::from_borrowed(self, tx_out.value * 1000) {
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
