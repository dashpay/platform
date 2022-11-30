#[cfg(test)]
mod apply_identity_credit_withdrawal_transition_factory {
    use std::convert::TryInto;

    use serde_json::json;

    use crate::{
        contracts::withdrawals_contract,
        data_contract::DataContract,
        identity::state_transition::identity_credit_withdrawal_transition::{
            apply_identity_credit_withdrawal_transition_factory::ApplyIdentityCreditWithdrawalTransition,
            IdentityCreditWithdrawalTransition,
        },
        prelude::{Identifier, Identity},
        state_repository::MockStateRepositoryLike,
        state_transition::StateTransitionConvert,
        tests::fixtures::get_data_contract_fixture,
    };

    #[tokio::test]
    async fn should_fail_if_data_contract_was_not_found() {
        let state_transition = IdentityCreditWithdrawalTransition::default();

        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_data_contract::<Option<DataContract>>()
            .times(1)
            .returning(|_, _| anyhow::Ok(None));

        let applier = ApplyIdentityCreditWithdrawalTransition::new(state_repository);

        match applier
            .apply_identity_credit_withdrawal_transition(&state_transition)
            .await
        {
            Ok(_) => assert!(false, "should not be able to apply state transition"),
            Err(e) => {
                assert_eq!(e.to_string(), "Withdrawals data contract not found");
            }
        };
    }

    #[tokio::test]
    async fn should_call_state_repository_methods() {
        let block_time_seconds = 1669260925;

        let mut state_transition = IdentityCreditWithdrawalTransition::default();

        state_transition.amount = 10;

        let st_hash: [u8; 32] = state_transition.hash(true).unwrap().try_into().unwrap();

        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_data_contract::<Option<DataContract>>()
            .times(1)
            .returning(|_, _| anyhow::Ok(Some(get_data_contract_fixture(None))));

        state_repository
            .expect_fetch_latest_platform_block_header()
            .times(1)
            .returning(move || anyhow::Ok(json!({"time": {"seconds": block_time_seconds}})));

        state_repository
            .expect_create_document()
            .times(1)
            .withf(move |doc, _| {
                let id_match = doc.id == Identifier::from_bytes(&st_hash).unwrap();

                let created_at_match = doc.created_at == Some(block_time_seconds * 1000);
                let updated_at_match = doc.created_at == Some(block_time_seconds * 1000);

                let document_data_match = doc.data
                    == json!({
                        "amount": 10,
                        "coreFeePerByte": 0,
                        "pooling": 0,
                        "outputScript": [],
                        "status": withdrawals_contract::statuses::QUEUED,
                    });

                id_match && created_at_match && updated_at_match && document_data_match
            })
            .returning(|_, _| anyhow::Ok(()));

        state_repository
            .expect_fetch_identity::<Identity>()
            .times(1)
            .withf(|id, _| *id == Identifier::default())
            .returning(|_, _| {
                let mut identity = Identity::default();

                identity.set_balance(42);

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
