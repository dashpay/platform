#[cfg(test)]
mod apply_identity_credit_withdrawal_transition_factory {
    use crate::{
        identity::state_transition::identity_credit_withdrawal_transition::{
            apply_identity_credit_withdrawal_transition_factory::ApplyIdentityCreditWithdrawalTransition,
            IdentityCreditWithdrawalTransition,
        },
        prelude::{Identifier, Identity},
        state_repository::MockStateRepositoryLike,
    };

    #[tokio::test]
    async fn should_call_state_repository_methods() {
        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_identity::<Identity>()
            .times(1)
            .withf(|id, _| *id == Identifier::default())
            .returning(|_, _| {
                let mut identity = Identity::default();

                identity = identity.set_balance(42);

                anyhow::Ok(Some(identity))
            });

        state_repository
            .expect_update_identity()
            .times(1)
            .withf(|identity, _| {
                let id_match = *identity.get_id() == Identifier::default();
                let balance_match = identity.get_balance() == (42 - 10);

                id_match && balance_match
            })
            .returning(|_, _| anyhow::Ok(()));

        let applier = ApplyIdentityCreditWithdrawalTransition::new(state_repository);

        let mut state_transition = IdentityCreditWithdrawalTransition::default();

        state_transition.amount = 10;

        match applier
            .apply_identity_credit_withdrawal_transition(&state_transition)
            .await
        {
            Ok(_) => assert!(true),
            Err(_) => {
                assert!(false, "should be able to apply the state transition");
            }
        };
    }
}
