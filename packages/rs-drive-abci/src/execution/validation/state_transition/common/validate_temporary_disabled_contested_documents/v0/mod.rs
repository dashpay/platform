use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::consensus::basic::document::ContestedDocumentsTemporarilyNotAllowedError;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;

pub const TARGET_EPOCH_INDEX: EpochIndex = 2;

pub fn validate_temporary_disabled_contested_documents(
    state_transition: &StateTransition,
    block_info: &BlockInfo,
) -> ConsensusValidationResult<()> {
    if block_info.epoch.index >= TARGET_EPOCH_INDEX {
        return ConsensusValidationResult::new();
    }

    if let StateTransition::DocumentsBatch(transition) = state_transition {
        let is_contested = transition.transitions().iter().any(|transition| {
            if let DocumentTransition::Create(create) = transition {
                create.prefunded_voting_balance().is_some()
            } else {
                false
            }
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
    }

    ConsensusValidationResult::new()
}
