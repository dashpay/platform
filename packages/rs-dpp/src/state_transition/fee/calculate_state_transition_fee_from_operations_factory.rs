use crate::fee::fee_result::FeeResult;
use crate::state_transition::fee::calculate_operation_fees::calculate_operation_fees;
use crate::state_transition::fee::operations::Operation;
use crate::{prelude::Identifier, NonConsensusError};

pub fn calculate_state_transition_fee_from_operations(
    operations: &[Operation],
    identity_id: &Identifier,
) -> Result<FeeResult, NonConsensusError> {
    calculate_state_transition_fee_from_operations_with_custom_calculator(
        operations,
        identity_id,
        calculate_operation_fees,
    )
}

fn calculate_state_transition_fee_from_operations_with_custom_calculator(
    operations: &[Operation],
    identity_id: &Identifier,
    calculate_operation_fees_fn: impl FnOnce(&[Operation]) -> Result<DummyFeesResult, NonConsensusError>,
) -> Result<FeeResult, NonConsensusError> {
    let calculated_fees = calculate_operation_fees_fn(operations)?;

    let storage_fee = calculated_fees.storage;
    let processing_fee = calculated_fees.processing;
    let fee_refunds = calculated_fees.fee_refunds;

    let mut total_refunds = 0;

    let owner_refunds = fee_refunds
        .iter()
        .find(|refunds| identity_id == &refunds.identifier);

    if let Some(owner_refunds) = owner_refunds {
        total_refunds = owner_refunds
            .credits_per_epoch
            .iter()
            .fold(0, |sum, (_, credits)| sum + credits);
    }

    let required_amount = if storage_fee > total_refunds {
        (storage_fee - total_refunds) + DEFAULT_USER_TIP
    } else {
        0
    };

    let fee_sum = storage_fee + processing_fee;

    let desired_amount = if fee_sum > total_refunds {
        (fee_sum - total_refunds) + DEFAULT_USER_TIP
    } else {
        0
    };

    Ok(FeeResult {
        storage_fee,
        processing_fee,
        fee_refunds,
        total_refunds,
        required_amount,
        desired_amount,
    })
}

#[cfg(test)]
mod test {

    use platform_value::Identifier;
    use std::collections::HashMap;

    use crate::fee::Credits;
    use crate::identity::KeyType::ECDSA_SECP256K1;
    use crate::state_transition::fee::calculate_operation_fees::calculate_operation_fees;
    use crate::state_transition::fee::operations::{
        PreCalculatedOperation, SignatureVerificationOperation,
    };
    use crate::{
        state_transition::fee::{operations::Operation, DummyFeesResult, FeeResult, Refunds},
        tests::utils::generate_random_identifier_struct,
        NonConsensusError,
    };

    use super::calculate_state_transition_fee_from_operations_with_custom_calculator;

    #[test]
    fn should_calculate_fee_based_on_executed_operations() {
        let identifier = generate_random_identifier_struct();
        let storage_fee = 10000;
        let processing_fee = 1000;
        let total_refunds = 1000 + 500;
        let required_amount = storage_fee - total_refunds;
        let desired_amount = storage_fee + processing_fee - total_refunds;

        let mut credits_per_epoch: HashMap<String, Credits> = Default::default();
        credits_per_epoch.insert("0".to_string(), 1000);
        credits_per_epoch.insert("1".to_string(), 500);

        let refunds = Refunds {
            identifier,
            credits_per_epoch,
        };

        let mock = |_operations: &[Operation]| -> Result<DummyFeesResult, NonConsensusError> {
            Ok(DummyFeesResult {
                storage: storage_fee,
                processing: processing_fee,
                fee_refunds: vec![refunds.clone()],
            })
        };

        let result = calculate_state_transition_fee_from_operations_with_custom_calculator(
            &[],
            &identifier,
            mock,
        )
        .expect("result should be returned");
        let expected = FeeResult {
            storage_fee,
            processing_fee,
            desired_amount,
            required_amount,
            fee_refunds: vec![refunds],
            total_refunds: 1500,
        };
        assert_eq!(expected, result);
    }

    #[test]
    fn test_with_negative_credits() {
        // Set of operations that produced by Document Remove Transition
        let operations = vec![
            Operation::PreCalculated(PreCalculatedOperation {
                storage_cost: 0,
                processing_cost: 551320,
                fee_refunds: vec![],
            }),
            Operation::SignatureVerification(SignatureVerificationOperation {
                signature_type: ECDSA_SECP256K1,
            }),
            Operation::PreCalculated(PreCalculatedOperation {
                storage_cost: 0,
                processing_cost: 551320,
                fee_refunds: vec![],
            }),
            Operation::PreCalculated(PreCalculatedOperation {
                storage_cost: 0,
                processing_cost: 191260,
                fee_refunds: vec![],
            }),
            Operation::PreCalculated(PreCalculatedOperation {
                storage_cost: 0,
                processing_cost: 16870910,
                fee_refunds: vec![Refunds {
                    identifier: Identifier::new([
                        130, 188, 56, 7, 78, 143, 58, 212, 133, 162, 145, 56, 186, 219, 191, 75,
                        64, 112, 236, 226, 135, 75, 132, 170, 135, 243, 180, 110, 103, 161, 153,
                        252,
                    ]),
                    credits_per_epoch: [("0".to_string(), 114301030)].iter().cloned().collect(),
                }],
            }),
        ];

        let identifier = Identifier::new([
            130, 188, 56, 7, 78, 143, 58, 212, 133, 162, 145, 56, 186, 219, 191, 75, 64, 112, 236,
            226, 135, 75, 132, 170, 135, 243, 180, 110, 103, 161, 153, 252,
        ]);

        // Panics because of negative credits
        let result = calculate_state_transition_fee_from_operations_with_custom_calculator(
            &operations,
            &identifier,
            calculate_operation_fees,
        );

        assert!(matches!(result, Ok(FeeResult {
            desired_amount,
            required_amount,
            ..
        }) if desired_amount == 0 && required_amount == 0));
    }
}
