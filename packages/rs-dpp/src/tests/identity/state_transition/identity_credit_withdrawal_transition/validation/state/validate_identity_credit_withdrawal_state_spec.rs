use crate::{
    identity::state_transition::identity_credit_withdrawal_transition::{
        validation::state::validate_identity_credit_withdrawal_transition_state::IdentityCreditWithdrawalTransitionValidator,
        IdentityCreditWithdrawalTransition,
    },
    state_repository::{MockStateRepositoryLike, StateRepositoryLike},
};

use std::sync::Arc;

#[cfg(test)]
pub fn setup_test<SR: StateRepositoryLike>(
    state_repository_mock: SR,
    amount_option: Option<u64>,
) -> (
    IdentityCreditWithdrawalTransition,
    IdentityCreditWithdrawalTransitionValidator<SR>,
) {
    let mut state_transition = IdentityCreditWithdrawalTransition::default();

    if let Some(amount) = amount_option {
        state_transition.amount = amount;
    }

    (
        state_transition,
        IdentityCreditWithdrawalTransitionValidator::new(Arc::new(state_repository_mock)),
    )
}

#[cfg(test)]
mod validate_identity_credit_withdrawal_transition_state_factory {
    use anyhow::Error;

    use crate::assert_consensus_errors;
    use crate::consensus::ConsensusError;
    use crate::prelude::{Identifier, Identity};

    use super::*;

    #[tokio::test]
    async fn should_return_invalid_result_if_identity_not_found() {
        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_identity::<Identity>()
            .times(1)
            .withf(|id, _| *id == Identifier::default())
            .returning(|_, _| anyhow::Ok(None));

        let (state_transition, validator) = setup_test(state_repository, None);

        let result = validator
            .validate_identity_credit_withdrawal_transition_state(&state_transition)
            .await
            .unwrap();

        assert_consensus_errors!(result, ConsensusError::BasicError, 1);

        let error = result.first_error().unwrap();

        assert_eq!(error.code(), 2000);
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_identity_have_not_enough_balance() {
        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_identity::<Identity>()
            .times(1)
            .withf(|id, _| *id == Identifier::default())
            .returning(|_, _| {
                let mut identity = Identity::default();

                identity.set_balance(10);

                anyhow::Ok(Some(identity))
            });

        let (state_transition, validator) = setup_test(state_repository, Some(42));

        let result = validator
            .validate_identity_credit_withdrawal_transition_state(&state_transition)
            .await
            .unwrap();

        assert_consensus_errors!(result, ConsensusError::IdentityInsufficientBalanceError, 1);

        let error = result.first_error().unwrap();

        assert_eq!(error.code(), 4024);
    }

    #[tokio::test]
    async fn should_return_original_error_if_any() {
        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_identity::<Identity>()
            .times(1)
            .withf(|id, _| *id == Identifier::default())
            .returning(|_, _| Err(Error::msg("Some error")));

        let (state_transition, validator) = setup_test(state_repository, Some(5));

        let result = validator
            .validate_identity_credit_withdrawal_transition_state(&state_transition)
            .await;

        match result {
            Ok(_) => assert!(false, "should not return Ok result"),
            Err(e) => assert_eq!(e.to_string(), "Some error"),
        }
    }

    #[tokio::test]
    async fn should_return_valid_result() {
        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_identity::<Identity>()
            .times(1)
            .withf(|id, _| *id == Identifier::default())
            .returning(|_, _| {
                let mut identity = Identity::default();

                identity.set_balance(10);

                anyhow::Ok(Some(identity))
            });

        let (state_transition, validator) = setup_test(state_repository, Some(5));

        let result = validator
            .validate_identity_credit_withdrawal_transition_state(&state_transition)
            .await
            .unwrap();

        assert_eq!(result.is_valid(), true);
    }
}
