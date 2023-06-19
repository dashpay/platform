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

use super::StateTransitionValidationV0;

impl StateTransitionValidationV0 for IdentityTopUpTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        protocol_version: u32,
        _tx: TransactionArg,
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
        protocol_version: u32,
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
            Some(partial_identity) => partial_identity,
        };

        validation_result.set_data(Some(partial_identity));
        Ok(validation_result)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let outpoint = match self.asset_lock_proof.out_point() {
            None => {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(
                                self.asset_lock_proof.instant_lock_output_index().unwrap(),
                            ),
                        ),
                    ),
                ));
            }
            Some(outpoint) => outpoint,
        };

        // Now we should check that we aren't using an asset lock again
        let asset_lock_already_found = platform
            .drive
            .has_asset_lock_outpoint(&Bytes36(outpoint), tx)?;

        if asset_lock_already_found {
            let outpoint = OutPoint::from(outpoint);
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutPointAlreadyExistsError(
                        IdentityAssetLockTransactionOutPointAlreadyExistsError::new(
                            outpoint.txid,
                            outpoint.vout as usize,
                        ),
                    ),
                ),
            ));
        }

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
        if !tx_out_validation.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                tx_out_validation.errors,
            ));
        }

        let tx_out = tx_out_validation.into_data()?;
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
