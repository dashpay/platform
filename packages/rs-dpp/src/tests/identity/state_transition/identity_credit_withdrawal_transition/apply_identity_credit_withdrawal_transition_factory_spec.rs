#[cfg(test)]
mod apply_identity_credit_withdrawal_transition_factory {
    use crate::{
        identity::state_transition::identity_credit_withdrawal_transition::{
            apply_identity_credit_withdrawal_transition_factory::ApplyIdentityCreditWithdrawalTransition,
            IdentityCreditWithdrawalTransition,
        },
        state_repository::MockStateRepositoryLike,
    };
    use mockall::predicate::{always, eq};
    use std::default::Default;

    #[tokio::test]
    async fn should_call_state_repository_methods() {
        let mut state_repository = MockStateRepositoryLike::default();

        let state_transition = IdentityCreditWithdrawalTransition {
            amount: 10,
            ..Default::default()
        };

        let IdentityCreditWithdrawalTransition {
            identity_id,
            amount,
            ..
        } = state_transition.clone();

        state_repository
            .expect_fetch_latest_withdrawal_transaction_index()
            .times(1)
            // trying to use values other than default to check they are actually set
            .returning(|| anyhow::Ok(42));

        state_repository
            .expect_enqueue_withdrawal_transaction()
            .withf(|index, _| *index == 42)
            .returning(|_, _| anyhow::Ok(()));

        state_repository
            .expect_remove_from_identity_balance()
            .times(1)
            // TODO: we need to assert execution context as well
            .with(eq(identity_id), eq(amount), always())
            .returning(|_, _, _| anyhow::Ok(()));

        state_repository
            .expect_remove_from_system_credits()
            .times(1)
            .with(eq(amount), always())
            .returning(|_, _| anyhow::Ok(()));

        let applier = ApplyIdentityCreditWithdrawalTransition::new(state_repository);

        let result = applier
            .apply_identity_credit_withdrawal_transition(&state_transition)
            .await;

        assert!(result.is_ok())
    }
}
