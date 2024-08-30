use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::block::epoch::EpochIndex;
use dpp::consensus::basic::document::ContestedDocumentsTemporarilyNotAllowedError;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::ConsensusValidationResult;

pub const TARGET_EPOCH_INDEX: EpochIndex = 3;

#[inline(always)]
pub fn validate_is_allowed_v0<C>(
    state_transition: &DocumentsBatchTransition,
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

    let is_contested = state_transition.transitions().iter().any(|transition| {
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
