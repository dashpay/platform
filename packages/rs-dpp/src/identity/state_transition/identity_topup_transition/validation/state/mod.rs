use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
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
    _state_transition: IdentityTopUpTransition,
    _state_repository: impl StateRepositoryLike,
) -> Result<ValidationResult<()>, NonConsensusError> {
    Ok(ValidationResult::default())
}
