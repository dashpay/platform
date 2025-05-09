use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
use dpp::serialization::PlatformDeserializable;

use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_proofs_v0(
        &self,
        GetProofsRequest {
            state_transition: state_transition_bytes,
        }: GetProofsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponse>, Error> {
        let state_transition =
            match StateTransition::deserialize_from_bytes(&state_transition_bytes) {
                Ok(state_transition) => state_transition,
                Err(e) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Protocol(
                        e,
                    )))
                }
            };

        let result =
            self.drive
                .prove_state_transition(&state_transition, None, platform_version)?;

        if !result.is_valid() {
            return Ok(QueryValidationResult::new_with_errors(
                result.errors.into_iter().map(QueryError::Proof).collect(),
            ));
        }

        let proof = result.into_data()?;

        let response = GetProofsResponse {
            proof: Some(self.response_proof_v0(platform_state, proof)),
            metadata: Some(self.response_metadata_v0(platform_state)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
