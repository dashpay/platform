use crate::consensus::state::identity::IdentityAlreadyExistsError;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::StateTransitionLike;
use crate::validation::{AsyncDataValidator, SimpleValidationResult, ValidationResult};
use crate::{NonConsensusError, ProtocolError};
use async_trait::async_trait;

pub struct IdentityCreateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncDataValidator for IdentityCreateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = IdentityCreateTransition;

    async fn validate(
        &self,
        data: &IdentityCreateTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        validate_identity_create_transition_state(&self.state_repository, data)
            .await
            .map_err(|err| err.into())
    }
}

impl<SR> IdentityCreateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> IdentityCreateTransitionStateValidator<SR>
    where
        SR: StateRepositoryLike,
    {
        IdentityCreateTransitionStateValidator { state_repository }
    }
}

/// Validate that identity exists
///
/// Do we need to check that key ids are incremental?
///
/// For later versions:
/// 1. We need to check that outpoint exists (not now)
/// 2. Verify ownership proof signature, as it requires special transaction to be implemented
pub async fn validate_identity_create_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &IdentityCreateTransition,
) -> Result<ValidationResult<()>, NonConsensusError> {
    // TODO: refactor to return ProtocolError?
    let mut result = ValidationResult::default();

    let identity_id = state_transition.get_identity_id();
    let balance = state_repository
        .fetch_identity_balance(identity_id, state_transition.get_execution_context())
        .await
        .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

    if state_transition.get_execution_context().is_dry_run() {
        return Ok(result);
    }

    if balance.is_some() {
        result.add_error(IdentityAlreadyExistsError::new(identity_id.to_buffer()));
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::{
        identity::state_transition::identity_create_transition::IdentityCreateTransition,
        state_repository::MockStateRepositoryLike, state_transition::StateTransitionLike,
        tests::fixtures::identity_create_transition_fixture_json,
    };

    use super::validate_identity_create_transition_state;

    #[tokio::test]
    async fn should_not_verify_signature_on_dry_run() {
        let mut state_repository = MockStateRepositoryLike::new();
        let raw_transition = identity_create_transition_fixture_json(None);
        let transition = IdentityCreateTransition::new(raw_transition).unwrap();

        transition.get_execution_context().enable_dry_run();
        state_repository
            .expect_fetch_identity_balance()
            .return_once(|_, _| Ok(Some(1)));

        let result =
            validate_identity_create_transition_state(&state_repository, &transition).await;
        assert!(result.is_ok());
    }
}
