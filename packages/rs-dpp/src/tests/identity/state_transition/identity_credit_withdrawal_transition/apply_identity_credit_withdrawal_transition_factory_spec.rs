#[cfg(test)]
mod apply_identity_credit_withdrawal_transition_factory {
    use dashcore::{consensus, BlockHeader};
    use serde_json::json;

    use crate::{
        contracts::withdrawals_contract,
        document::Document,
        identity::state_transition::identity_credit_withdrawal_transition::{
            apply_identity_credit_withdrawal_transition_factory::ApplyIdentityCreditWithdrawalTransition,
            IdentityCreditWithdrawalTransition, Pooling,
        },
        state_repository::MockStateRepositoryLike,
        tests::fixtures::get_data_contract_fixture,
    };
    use mockall::predicate::{always, eq};
    use std::default::Default;

    #[tokio::test]
    async fn should_fail_if_data_contract_was_not_found() {
        let mut state_repository = MockStateRepositoryLike::default();

        let state_transition = IdentityCreditWithdrawalTransition {
            amount: 10,
            ..Default::default()
        };

        state_repository
            .expect_fetch_data_contract()
            .times(1)
            .returning(|_, _| anyhow::Ok(None));

        let applier = ApplyIdentityCreditWithdrawalTransition::new(state_repository);

        match applier
            .apply_identity_credit_withdrawal_transition(&state_transition)
            .await
        {
            Ok(_) => panic!("should not be able to apply state transition"),
            Err(e) => {
                assert_eq!(e.to_string(), "Withdrawals data contract not found");
            }
        };
    }

    #[tokio::test]
    async fn should_create_withdrawal_and_reduce_balance() {
        let block_time_seconds = 1675709306;

        let state_transition = IdentityCreditWithdrawalTransition {
            amount: 10,
            ..Default::default()
        };

        let mut state_repository = MockStateRepositoryLike::default();

        state_repository
            .expect_fetch_documents::<Document>()
            .returning(|_, _, _, _| anyhow::Ok(vec![]));

        state_repository
            .expect_fetch_data_contract()
            .times(1)
            .returning(|_, _| anyhow::Ok(Some(get_data_contract_fixture(None))));

        state_repository
            .expect_fetch_latest_platform_block_header()
            .times(1)
            .returning(move || {
                let header = BlockHeader {
                    time: block_time_seconds,
                    version: 1,
                    prev_blockhash: Default::default(),
                    merkle_root: Default::default(),
                    bits: Default::default(),
                    nonce: Default::default(),
                };

                anyhow::Ok(consensus::serialize(&header))
            });

        state_repository
            .expect_create_document()
            .times(1)
            .withf(move |doc, _| {
                let created_at_match = doc.created_at == Some(block_time_seconds as i64 * 1000);
                let updated_at_match = doc.created_at == Some(block_time_seconds as i64 * 1000);

                let document_data_match = doc.data
                    == json!({
                        "amount": 10,
                        "coreFeePerByte": 0,
                        "pooling": Pooling::Never,
                        "outputScript": [],
                        "status": withdrawals_contract::WithdrawalStatus::QUEUED,
                    });

                created_at_match && updated_at_match && document_data_match
            })
            .returning(|_, _| anyhow::Ok(()));

        state_repository
            .expect_remove_from_identity_balance()
            .times(1)
            // TODO: we need to assert execution context as well
            .with(
                eq(state_transition.identity_id),
                eq(state_transition.amount),
                always(),
            )
            .returning(|_, _, _| anyhow::Ok(()));

        state_repository
            .expect_remove_from_system_credits()
            .times(1)
            .with(eq(state_transition.amount), always())
            .returning(|_, _| anyhow::Ok(()));

        let applier = ApplyIdentityCreditWithdrawalTransition::new(state_repository);

        let result = applier
            .apply_identity_credit_withdrawal_transition(&state_transition)
            .await;

        assert!(result.is_ok())
    }
}
