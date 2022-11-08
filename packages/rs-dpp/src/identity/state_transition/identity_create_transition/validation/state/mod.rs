use crate::consensus::state::identity::IdentityAlreadyExistsError;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::prelude::Identity;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::StateTransitionLike;
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
    state_repository: &impl StateRepositoryLike,
    state_transition: IdentityCreateTransition,
) -> Result<ValidationResult<()>, NonConsensusError> {
    let mut result = ValidationResult::default();

    let identity_id = state_transition.get_identity_id();
    let maybe_identity = state_repository
        .fetch_identity::<Identity>(identity_id, state_transition.get_execution_context())
        .await
        .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

    if state_transition.get_execution_context().is_dry_run() {
        return Ok(result);
    }

    if let Some(_identity) = maybe_identity {
        result.add_error(IdentityAlreadyExistsError::new(identity_id.to_buffer()));
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::{
        identity::state_transition::identity_create_transition::IdentityCreateTransition,
        prelude::Identity, state_repository::MockStateRepositoryLike,
        state_transition::StateTransitionLike,
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
            .expect_fetch_identity()
            .return_once(|_, _| Ok(Some(Identity::default())));

        let result = validate_identity_create_transition_state(&state_repository, transition).await;
        assert!(result.is_ok());
    }
}
