use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::state_repository::StateRepositoryLike;
use crate::validation::{AsyncDataValidator, SimpleValidationResult, ValidationResult};
use crate::{NonConsensusError, ProtocolError};
use async_trait::async_trait;

pub struct IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncDataValidator for IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = IdentityTopUpTransition;

    async fn validate(
        &self,
        data: &IdentityTopUpTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        validate_identity_topup_transition_state(data, &self.state_repository)
            .await
            .map(|result| result.into())
            .map_err(|err| err.into())
    }
}

impl<SR> IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> IdentityTopUpTransitionStateValidator<SR>
    where
        SR: StateRepositoryLike,
    {
        IdentityTopUpTransitionStateValidator { state_repository }
    }
}

/// Validate that identity exists
///
/// Do we need to check that key ids are incremental?
///
/// For later versions:
/// 1. We need to check that outpoint exists (not now)
/// 2. Verify ownership proof signature, as it requires special transaction to be implemented
pub async fn validate_identity_topup_transition_state(
    _state_transition: &IdentityTopUpTransition,
    _state_repository: &impl StateRepositoryLike,
) -> Result<ValidationResult<()>, NonConsensusError> {
    Ok(ValidationResult::default())
}
