use crate::consensus::state::identity::IdentityAlreadyExistsError;
use crate::identity::state_transition::identity_create_transition::{
    IdentityCreateTransition, IdentityCreateTransitionAction,
};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::StateTransitionLike;
use crate::validation::{AsyncDataValidator, ValidationResult};
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
    type ResultItem = IdentityCreateTransitionAction;

    async fn validate(
        &self,
        data: &IdentityCreateTransition,
    ) -> Result<ValidationResult<Self::ResultItem>, ProtocolError> {
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
) -> Result<ValidationResult<IdentityCreateTransitionAction>, ProtocolError> {
    let mut result = ValidationResult::default();

    let identity_id = state_transition.get_identity_id();
    let balance = state_repository
        .fetch_identity_balance(identity_id, Some(state_transition.get_execution_context()))
        .await
        .map_err(|e| {
            NonConsensusError::StateRepositoryFetchError(format!(
                "state repository fetch identity balance error: {}",
                e.to_string()
            ))
        })?;

    if state_transition.get_execution_context().is_dry_run() {
        return Ok(IdentityCreateTransitionAction::from_borrowed(state_transition, 0).into());
    }

    // Balance is here to check if the identity does already exist
    if balance.is_some() {
        result.add_error(IdentityAlreadyExistsError::new(identity_id.to_buffer()));
        Ok(result)
    } else {
        let tx_out = state_transition
            .asset_lock_proof
            .fetch_asset_lock_transaction_output(
                state_repository,
                &state_transition.execution_context,
            )
            .await
            .map_err(Into::<NonConsensusError>::into)?;
        return Ok(
            IdentityCreateTransitionAction::from_borrowed(state_transition, tx_out.value).into(),
        );
    }
}

#[cfg(test)]
mod test {
    use crate::{
        identity::state_transition::identity_create_transition::IdentityCreateTransition,
        state_repository::MockStateRepositoryLike, state_transition::StateTransitionLike,
        tests::fixtures::identity_create_transition_fixture,
    };

    use super::validate_identity_create_transition_state;

    #[tokio::test]
    async fn should_not_verify_signature_on_dry_run() {
        let mut state_repository = MockStateRepositoryLike::new();
        let raw_transition = identity_create_transition_fixture(None);
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
