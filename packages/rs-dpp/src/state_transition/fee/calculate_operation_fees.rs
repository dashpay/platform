use super::{
    operations::{Operation, OperationLike},
    DummyFeesResult, Refunds,
};

pub fn calculate_operation_fees(operations: &[Operation]) -> DummyFeesResult {
    let mut storage_fee = 0;
    let mut processing_fee = 0;
    let mut fee_refunds: Vec<Refunds> = Vec::new();

    for operation in operations {
        storage_fee += operation.get_storage_cost();
        processing_fee += operation.get_processing_cost();

        // Merge refunds
        if let Some(operation_refunds) = operation.get_refunds() {
            for identity_refunds in operation_refunds {
                let mut existing_identity_refunds = fee_refunds
                    .iter_mut()
                    .find(|refund| refund.identifier == identity_refunds.identifier);

                if existing_identity_refunds.is_none() {
                    fee_refunds.push(identity_refunds.clone());
                    continue;
                }

                for (epoch_index, credits) in identity_refunds.credits_per_epoch.iter() {
                    if let Some(ref mut refunds) = existing_identity_refunds {
                        let epoch = refunds
                            .credits_per_epoch
                            .entry(epoch_index.to_string())
                            .or_default();
                        *epoch += credits
                    }
                }
            }
        }
    }

    DummyFeesResult {
        storage: storage_fee,
        processing: processing_fee,
        fee_refunds,
    }
}
