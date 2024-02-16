use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identity::identity_nonce::{
    validate_identity_nonce_update, validate_new_identity_nonce, MergeIdentityNonceResult,
    IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES,
    MISSING_IDENTITY_REVISIONS_FILTER, MISSING_IDENTITY_REVISIONS_MAX_BYTES,
};
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;

use dpp::validation::SimpleConsensusValidationResult;

use crate::platform_types::platform::PlatformStateRef;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions) trait DataContractUpdateStateTransitionIdentityContractNonceV0
{
    fn validate_identity_contract_nonce_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionIdentityContractNonceV0 for DataContractUpdateTransition {
    fn validate_identity_contract_nonce_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let revision_nonce = self.identity_contract_nonce();
        let identity_id = self.data_contract().owner_id();
        let contract_id = self.data_contract().id();
        let (existing_nonce, fees) = platform.drive.fetch_identity_contract_nonce_with_fees(
            identity_id.to_buffer(),
            contract_id.to_buffer(),
            block_info,
            true,
            tx,
            platform_version,
        )?;

        let result = if let Some(existing_nonce) = existing_nonce {
            validate_identity_nonce_update(existing_nonce, revision_nonce, identity_id)
        } else {
            validate_new_identity_nonce(revision_nonce, identity_id)
        };

        Ok(result)
    }
}
