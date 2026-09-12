use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Proves a state transition, version 1: the transition is decoded under the size cap and
    /// the decode budget of the family its wire prefix names, with the active protocol
    /// version, so a proof request for a contract-code envelope the block accepted decodes
    /// like the block did. Ordinary transitions decode exactly as in version 0.
    pub(super) fn query_proofs_v1(
        &self,
        GetProofsRequest {
            state_transition: state_transition_bytes,
        }: GetProofsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponse>, Error> {
        let kind = StateTransition::peek_envelope_kind(&state_transition_bytes);
        let max_size = StateTransition::family_max_size(kind, platform_version);
        if state_transition_bytes.len() as u64 > max_size {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "state transition size {} exceeds the maximum of {max_size} bytes for its family",
                    state_transition_bytes.len()
                )),
            ));
        }

        let state_transition = match StateTransition::deserialize_from_bytes_in_version_bounded(
            &state_transition_bytes,
            platform_version,
        ) {
            Ok(state_transition) => state_transition,
            Err(e) => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Protocol(
                    e,
                )))
            }
        };

        let result = self
            .drive
            .prove_state_transition(&state_transition, None, platform_version)
            .inspect_err(|e| {
                tracing::warn!(
                    state_transition_type = %state_transition.state_transition_type(),
                    error = %e,
                    "Error while proving state transition: {}",
                    e
                )
            })?;

        if !result.is_valid() {
            return Ok(QueryValidationResult::new_with_errors(
                result.errors.into_iter().map(QueryError::Proof).collect(),
            ));
        }

        let proof = result.into_data()?;

        let (grovedb_used, proof) =
            self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

        let response = GetProofsResponse {
            proof: Some(proof),
            metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::ProtocolError;

    /// A synthetic envelope with the contract-code capable prefix of a contract create
    /// transition (outer index 0, inner index 1).
    fn contract_code_capable_envelope(len: usize) -> Vec<u8> {
        // The filler is not zero because bincode decodes trailing zeroes as valid empty fields.
        let mut envelope = vec![0xFFu8; len];
        envelope[0] = 0;
        envelope[1] = 1;
        envelope
    }

    #[test]
    fn should_reach_the_decode_step_for_an_envelope_at_the_family_cap() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_state = platform.state.load();
        let family_cap = platform_version
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes")
            as usize;

        let result = platform
            .query_proofs_v1(
                GetProofsRequest {
                    state_transition: contract_code_capable_envelope(family_cap),
                },
                &platform_state,
                platform_version,
            )
            .expect("the query handler must not fail");
        // Zeroed bytes are garbage, never over the family cap and never over the budget.
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Protocol(
                ProtocolError::PlatformDeserializationError(_)
            )]
        ));
    }

    #[test]
    fn should_reject_an_envelope_above_the_family_cap_before_decoding() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_state = platform.state.load();
        let family_cap = platform_version
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes")
            as usize;

        let result = platform
            .query_proofs_v1(
                GetProofsRequest {
                    state_transition: contract_code_capable_envelope(family_cap + 1),
                },
                &platform_state,
                platform_version,
            )
            .expect("the query handler must not fail");
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(message)] if message.contains("exceeds the maximum")
        ));
    }

    #[test]
    fn should_keep_the_ordinary_cap_for_an_ordinary_transition() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_state = platform.state.load();
        let ordinary_cap = platform_version.system_limits.max_state_transition_size as usize;

        // Outer index 2 is Batch.
        let mut oversized = vec![0xFFu8; ordinary_cap + 1];
        oversized[0] = 2;
        let result = platform
            .query_proofs_v1(
                GetProofsRequest {
                    state_transition: oversized,
                },
                &platform_state,
                platform_version,
            )
            .expect("the query handler must not fail");
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(message)] if message.contains("exceeds the maximum")
        ));
    }
}
