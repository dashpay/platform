use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError,
};
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::consensus::ConsensusError;

use dpp::dashcore::OutPoint;

use dpp::identity::state_transition::identity_create_transition::validation::basic::IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use dpp::identity::PartialIdentity;
use dpp::platform_value::Bytes36;
use dpp::serialization_traits::{PlatformMessageSignable, Signable};
use dpp::validation::ConsensusValidationResult;
use dpp::{
    identity::state_transition::identity_create_transition::IdentityCreateTransition,
    state_transition::StateTransitionAction, validation::SimpleConsensusValidationResult,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::key_validation::{
    validate_identity_public_keys_structure, validate_unique_identity_public_key_hashes_state,
};

use crate::asset_lock::fetch_tx_out::FetchAssetLockProofTxOut;

use crate::platform::PlatformRef;
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
        let result = validate_schema(&IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure(self.public_keys.as_slice())
    }

    fn validate_identity_and_signatures(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();
        let bytes: Vec<u8> = self.signable_bytes()?;
        for key in self.public_keys.iter() {
            let result = bytes.as_slice().verify_signature(
                key.key_type,
                key.data.as_slice(),
                key.signature.as_slice(),
            )?;
            if !result.is_valid() {
                validation_result.add_errors(result.errors);
            }
        }

        // We should validate that the identity id is created from the asset lock proof

        let identifier_from_outpoint = match self.get_asset_lock_proof().create_identifier() {
            Ok(identifier) => identifier,
            Err(_) => {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(
                                self.asset_lock_proof.instant_lock_output_index().unwrap(),
                            ),
                        ),
                    ),
                ))
            }
        };

        if identifier_from_outpoint != self.identity_id {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidIdentifierError(
                    InvalidIdentifierError::new(
                        "identity_id".to_string(),
                        "does not match created identifier from asset lock".to_string(),
                    ),
                )),
            ));
        }

        // We need to set the data, even though we are setting to None,
        // We are really setting to Some(None) internally,
        validation_result.set_data(None);
        Ok(validation_result)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        let identity_id = self.get_identity_id();
        let balance = drive.fetch_identity_balance(self.identity_id.to_buffer(), tx)?;

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
        let asset_lock_already_found = drive.has_asset_lock_outpoint(&Bytes36(outpoint), tx)?;

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

        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_state(
                self.public_keys.as_slice(),
                drive,
                tx,
            )?
            .errors,
        );

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
