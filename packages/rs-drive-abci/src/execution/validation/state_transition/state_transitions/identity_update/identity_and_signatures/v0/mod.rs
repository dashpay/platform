use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;

use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;

use crate::execution::validation::state_transition::common::validate_state_transition_identity_signature::v0::validate_state_transition_identity_signature_v0;
use dpp::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization_traits::{PlatformMessageSignable, Signable};
use dpp::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(crate) trait StateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;
}

impl StateTransitionIdentityAndSignaturesValidationV0 for IdentityUpdateTransition {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut result = ConsensusValidationResult::<Option<PartialIdentity>>::default();

        let bytes: Vec<u8> = self.signable_bytes()?;
        for key in self.add_public_keys.iter() {
            let validation_result = bytes.as_slice().verify_signature(
                key.key_type,
                key.data.as_slice(),
                key.signature.as_slice(),
            )?;
            if !validation_result.is_valid() {
                result.add_errors(validation_result.errors);
            }
        }

        let validation_result = validate_state_transition_identity_signature_v0(
            drive,
            self,
            true,
            transaction,
            platform_version,
        )?;

        if !validation_result.is_valid() {
            result.merge(validation_result);
            return Ok(result);
        }

        let partial_identity = validation_result.into_data()?;

        let Some(revision) = partial_identity.revision else {
            return Err(Error::Execution(CorruptedCodeExecution("revision should exist")));
        };

        // Check revision
        if revision + 1 != self.revision {
            result.add_error(StateError::InvalidIdentityRevisionError(
                InvalidIdentityRevisionError::new(self.identity_id, revision),
            ));
            return Ok(result);
        }

        result.set_data(Some(partial_identity));

        Ok(result)
    }
}
