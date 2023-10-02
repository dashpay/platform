#[cfg(test)]
mod apply_identity_credit_withdrawal_transition_factory {
    use dashcore::{consensus, Header as BlockHeader};

    use std::collections::BTreeMap;

    use crate::document::ExtendedDocument;
    use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::{
        identity::state_transition::identity_credit_withdrawal_transition::{
            apply_identity_credit_withdrawal_transition_factory::ApplyIdentityCreditWithdrawalTransition,
            IdentityCreditWithdrawalTransition, Pooling,
        },
        state_repository::MockStateRepositoryLike,
        tests::fixtures::get_data_contract_fixture,
    };
    use data_contracts::withdrawals_contract::document_types::withdrawal::properties::{
        AMOUNT, CORE_FEE_PER_BYTE, OUTPUT_SCRIPT, POOLING, STATUS,
    };
    use mockall::predicate::{always, eq};
    use platform_value::Value;
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

        let execution_context = StateTransitionExecutionContext::default();

        match applier
            .apply_identity_credit_withdrawal_transition(&state_transition, &execution_context)
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
            .expect_fetch_documents()
            .returning(|_, _, _, _| anyhow::Ok(vec![]));

        state_repository
            .expect_fetch_data_contract()
            .times(1)
            .returning(|_, _| anyhow::Ok(Some(get_data_contract_fixture(None).data_contract)));

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
            .withf(move |extended_document: &ExtendedDocument, _| {
                let document = &extended_document.document;
                let created_at_match =
                    document.created_at == Some(block_time_seconds as u64 * 1000);
                let updated_at_match =
                    document.updated_at == Some(block_time_seconds as u64 * 1000);

                let document_expected_properties = BTreeMap::from([
                    (AMOUNT.to_string(), Value::U64(10)),
                    (CORE_FEE_PER_BYTE.to_string(), Value::U32(0)),
                    (POOLING.to_string(), Value::U8(Pooling::Never as u8)),
                    (OUTPUT_SCRIPT.to_string(), Value::Bytes(vec![])),
                    (
                        STATUS.to_string(),
                        Value::U8(withdrawals_contract::WithdrawalStatus::QUEUED as u8),
                    ),
                ]);

                let document_data_match = document.properties == document_expected_properties;

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

        let execution_context = StateTransitionExecutionContext::default();

        let result = applier
            .apply_identity_credit_withdrawal_transition(&state_transition, &execution_context)
            .await;

        assert!(result.is_ok())
    }
}
