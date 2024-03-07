use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::identity::identity_nonce::{validate_identity_nonce_update, validate_new_identity_nonce};
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

use dpp::validation::SimpleConsensusValidationResult;

use crate::platform_types::platform::PlatformStateRef;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions) trait IdentityCreditTransferTransitionIdentityNonceV0
{
    fn validate_nonce_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferTransitionIdentityNonceV0 for IdentityCreditTransferTransition {
    fn validate_nonce_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let revision_nonce = self.nonce();
        let identity_id = self.identity_id();
        let (existing_nonce, _unused_fees) = platform.drive.fetch_identity_nonce_with_fees(
            identity_id.to_buffer(),
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
