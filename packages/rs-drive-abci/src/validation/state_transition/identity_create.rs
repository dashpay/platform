use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use dpp::identity::PartialIdentity;
use dpp::{
    identity::state_transition::identity_create_transition::validation::basic::IDENTITY_CREATE_TRANSITION_SCHEMA,
    validation::ConsensusValidationResult,
};
use dpp::{
    identity::state_transition::identity_create_transition::IdentityCreateTransition,
    state_transition::StateTransitionAction, validation::SimpleConsensusValidationResult,
};
use drive::grovedb::TransactionArg;
use drive::{drive::Drive, grovedb::Transaction};

use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::key_validation::{
    validate_identity_public_keys_signatures, validate_identity_public_keys_structure,
    validate_unique_identity_public_key_hashes_state,
};

use crate::asset_lock::fetch_tx_out::FetchAssetLockProofTxOut;

use crate::{
    error::Error,
    validation::state_transition::common::{validate_protocol_version, validate_schema},
};

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityCreateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(IDENTITY_CREATE_TRANSITION_SCHEMA.clone(), self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure(self.public_keys.as_slice())
    }

    fn validate_signatures(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();
        validation_result.add_errors(
            validate_identity_public_keys_signatures(self.public_keys.as_slice())?.errors,
        );
        // We need to set the data, even though we are setting to None,
        // We are really setting to Some(None) internally,
        validation_result.set_data(None);
        Ok(validation_result)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        drive: &Drive,
        core_rpc: &C,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let identity_id = self.get_identity_id();
        let balance = drive.fetch_identity_balance(self.identity_id.to_buffer(), tx)?;

        // Balance is here to check if the identity does already exist
        if balance.is_some() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAlreadyExistsError::new(identity_id.to_buffer()).into(),
            ));
        }

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_state(
                self.public_keys.as_slice(),
                drive,
                tx,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let tx_out_validation = self
            .asset_lock_proof
            .fetch_asset_lock_transaction_output_sync(core_rpc)?;
        if !tx_out_validation.is_valid_with_data() {
            return Ok(ConsensusValidationResult::new_with_errors(
                tx_out_validation.errors,
            ));
        }

        let tx_out = tx_out_validation.into_data()?;
        validation_result.set_data(
            IdentityCreateTransitionAction::from_borrowed(self, tx_out.value * 1000).into(),
        );
        return Ok(validation_result);
    }
}
