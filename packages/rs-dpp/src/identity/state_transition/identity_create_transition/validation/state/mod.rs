use crate::consensus::state::identity::IdentityAlreadyExistsError;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::prelude::Identity;
use crate::state_repository::StateRepositoryLike;
use crate::validation::ValidationResult;
use crate::NonConsensusError;

/// Validate that identity exists
///
/// Do we need to check that key ids are incremental?
///
/// For later versions:
/// 1. We need to check that outpoint exists (not now)
/// 2. Verify ownership proof signature, as it requires special transaction to be implemented
pub async fn validate_identity_create_transition_state(
    state_transition: IdentityCreateTransition,
    state_repository: impl StateRepositoryLike,
) -> Result<ValidationResult<()>, NonConsensusError> {
    let mut result = ValidationResult::default();

    let identity_id = state_transition.get_identity_id();
    let maybe_identity = state_repository
        .fetch_identity::<Identity>(identity_id)
        .await
        .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

    if let Some(_identity) = maybe_identity {
        result.add_error(IdentityAlreadyExistsError::new(identity_id.to_buffer()));
    }

    Ok(result)
}
