use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use dpp::consensus::basic::document::ContestedDocumentsTemporarilyNotAllowedError;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV1;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::validation::ConsensusValidationResult;

use crate::execution::validation::state_transition::batch::is_allowed::v0::TARGET_EPOCH_INDEX;

/// Generation 1 reads a batch of any wire format, including format 2, whose
/// document shell carries the erase kind. The rule is unchanged: contested
/// document creation is not allowed before the target epoch.
#[inline(always)]
pub fn validate_is_allowed_v1<C>(
    state_transition: &BatchTransition,
    platform: &PlatformRef<C>,
) -> ConsensusValidationResult<()> {
    #[cfg(feature = "testing-config")]
    if platform
        .config
        .testing_configs
        .disable_contested_documents_is_allowed_validation
    {
        return ConsensusValidationResult::new();
    }

    let block_info = platform.state.last_block_info();

    if block_info.epoch.index >= TARGET_EPOCH_INDEX {
        return ConsensusValidationResult::new();
    }

    let is_contested = state_transition.transitions_iter_v1().any(|transition| {
        transition
            .as_transition_create()
            .and_then(|create| create.prefunded_voting_balance().as_ref())
            .is_some()
    });

    if is_contested {
        return ConsensusValidationResult::new_with_errors(vec![
            ContestedDocumentsTemporarilyNotAllowedError::new(
                block_info.epoch.index,
                TARGET_EPOCH_INDEX,
            )
            .into(),
        ]);
    }

    ConsensusValidationResult::new()
}
