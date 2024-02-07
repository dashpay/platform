use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityContractNonceError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identity::identity_contract_nonce::{
    MergeIdentityContractNonceResult, IDENTITY_CONTRACT_NONCE_VALUE_FILTER,
    IDENTITY_CONTRACT_NONCE_VALUE_FILTER_MAX_BYTES, MISSING_IDENTITY_CONTRACT_REVISIONS_FILTER,
    MISSING_IDENTITY_CONTRACT_REVISIONS_MAX_BYTES,
};
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;

use dpp::validation::SimpleConsensusValidationResult;

use crate::platform_types::platform::PlatformStateRef;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionIdentityContractNonceV0
{
    fn validate_identity_contract_nonces_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchStateTransitionIdentityContractNonceV0 for DocumentsBatchTransition {
    fn validate_identity_contract_nonces_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // We should validate that all newly created documents have valid ids
        for transition in self.transitions() {
            let revision_nonce = transition.identity_contract_nonce();
            let identity_id = self.owner_id();
            let (existing_nonce, fees) = platform.drive.fetch_identity_contract_nonce_with_fees(
                identity_id.to_buffer(),
                transition.data_contract_id().to_buffer(),
                block_info,
                true,
                tx,
                platform_version,
            )?;

            if let Some(existing_nonce) = existing_nonce {
                let actual_existing_revision =
                    existing_nonce & IDENTITY_CONTRACT_NONCE_VALUE_FILTER;
                if actual_existing_revision == revision_nonce {
                    // we were not able to update the revision as it is the same as we already had
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::InvalidIdentityContractNonceError(
                            InvalidIdentityContractNonceError {
                                identity_id,
                                current_identity_contract_nonce: Some(existing_nonce),
                                setting_identity_contract_nonce: revision_nonce,
                                error: MergeIdentityContractNonceResult::NonceAlreadyPresentAtTip,
                            },
                        )),
                    ));
                } else if actual_existing_revision < revision_nonce {
                    if revision_nonce - actual_existing_revision
                        >= MISSING_IDENTITY_CONTRACT_REVISIONS_MAX_BYTES
                    {
                        // we are too far away from the actual revision
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(
                                StateError::InvalidIdentityContractNonceError(
                                    InvalidIdentityContractNonceError {
                                        identity_id,
                                        current_identity_contract_nonce: Some(existing_nonce),
                                        setting_identity_contract_nonce: revision_nonce,
                                        error:
                                            MergeIdentityContractNonceResult::NonceTooFarInFuture,
                                    },
                                ),
                            ),
                        ));
                    }
                } else {
                    let previous_revision_position_from_top =
                        actual_existing_revision - revision_nonce;
                    if previous_revision_position_from_top
                        >= MISSING_IDENTITY_CONTRACT_REVISIONS_MAX_BYTES
                    {
                        // we are too far away from the actual revision
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(
                                StateError::InvalidIdentityContractNonceError(
                                    InvalidIdentityContractNonceError {
                                        identity_id,
                                        current_identity_contract_nonce: Some(existing_nonce),
                                        setting_identity_contract_nonce: revision_nonce,
                                        error: MergeIdentityContractNonceResult::NonceTooFarInPast,
                                    },
                                ),
                            ),
                        ));
                    } else {
                        let old_missing_revisions =
                            (existing_nonce & MISSING_IDENTITY_CONTRACT_REVISIONS_FILTER);
                        let byte_to_set = 1
                            << (previous_revision_position_from_top
                                + IDENTITY_CONTRACT_NONCE_VALUE_FILTER_MAX_BYTES);
                        let old_revision_already_set = (old_missing_revisions & byte_to_set) > 0;
                        if old_revision_already_set {
                            return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::InvalidIdentityContractNonceError(InvalidIdentityContractNonceError{
                                identity_id,
                                current_identity_contract_nonce: Some(existing_nonce),
                                setting_identity_contract_nonce: revision_nonce,
                                error: MergeIdentityContractNonceResult::NonceAlreadyPresentInPast(previous_revision_position_from_top),
                            }))));
                        }
                    }
                }
            } else if revision_nonce >= MISSING_IDENTITY_CONTRACT_REVISIONS_MAX_BYTES {
                // we are too far away from the actual revision
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::InvalidIdentityContractNonceError(
                        InvalidIdentityContractNonceError {
                            identity_id,
                            current_identity_contract_nonce: existing_nonce,
                            setting_identity_contract_nonce: revision_nonce,
                            error: MergeIdentityContractNonceResult::NonceTooFarInPast,
                        },
                    )),
                ));
            };
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
