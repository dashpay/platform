use crate::{prelude::Identifier, NonConsensusError};

use super::{
    calculate_operation_fees::calculate_operation_fees, constants::DEFAULT_USER_TIP,
    operations::Operation, DummyFeesResult, FeeResult,
};

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

    let required_amount = (storage_fee - total_refunds) + DEFAULT_USER_TIP;
    let desired_amount = (storage_fee + processing_fee - total_refunds) + DEFAULT_USER_TIP;

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
    use std::collections::HashMap;

    use crate::{
        state_transition::fee::{
            operations::Operation, Credits, DummyFeesResult, FeeResult, Refunds,
        },
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
}
