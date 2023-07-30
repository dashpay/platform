use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError,
};
use dpp::consensus::basic::BasicError;

use dpp::consensus::state::identity::IdentityAlreadyExistsError;

use dpp::consensus::ConsensusError;
use dpp::dashcore::OutPoint;

use dpp::identity::state_transition::identity_create_transition::{
    IdentityCreateTransition, IdentityCreateTransitionAction,
};

use dpp::platform_value::Bytes36;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::StateTransitionAction;
use dpp::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use dpp::state_transition_action::StateTransitionAction;
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;
use crate::execution::validation::asset_lock::fetch_tx_out::v0::FetchAssetLockProofTxOutV0;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::v0::validate_unique_identity_public_key_hashes_in_state_v0;

pub(crate) trait StateTransitionStateValidationV0 {
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

impl StateTransitionStateValidationV0 for IdentityCreateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let identity_id = self.identity_id();
        let balance =
            drive.fetch_identity_balance(self.identity_id.to_buffer(), tx, platform_version)?;

        // Balance is here to check if the identity does already exist
        if balance.is_some() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAlreadyExistsError::new(identity_id.to_owned()).into(),
            ));
        }
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
        let asset_lock_already_found =
            drive.has_asset_lock_outpoint(&Bytes36(outpoint), tx, &platform_version.drive)?;

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

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_in_state_v0(
                self.public_keys.as_slice(),
                drive,
                tx,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        self.transform_into_action_v0(platform)
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let tx_out_validation = self
            .asset_lock_proof
            .fetch_asset_lock_transaction_output_sync_v0(platform.core_rpc)?;
        if !tx_out_validation.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                tx_out_validation.errors,
            ));
        }

        let tx_out = tx_out_validation.into_data()?;
        match IdentityCreateTransitionAction::from_borrowed(self, tx_out.value * 1000) {
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
