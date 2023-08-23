use crate::fee::Credits;
use crate::NonConsensusError;

use super::operations::{Operation, OperationLike};
//
// pub fn calculate_operation_fees(
//     operations: &[Operation],
// ) -> Result<DummyFeesResult, NonConsensusError> {
//     let mut storage_fee: Credits = 0;
//     let mut processing_fee: Credits = 0;
//     let mut fee_refunds: Vec<Refunds> = Vec::new();
//
//     for operation in operations {
//         storage_fee = storage_fee
//             .checked_add(operation.get_storage_cost()?)
//             .ok_or(NonConsensusError::Overflow("storage cost is too big"))?;
//         processing_fee = processing_fee
//             .checked_add(operation.get_processing_cost()?)
//             .ok_or(NonConsensusError::Overflow("processing cost is too big"))?;
//
//         // Merge refunds
//         if let Some(operation_refunds) = operation.get_refunds() {
//             for identity_refunds in operation_refunds {
//                 let mut existing_identity_refunds = fee_refunds
//                     .iter_mut()
//                     .find(|refund| refund.identifier == identity_refunds.identifier);
//
//                 if existing_identity_refunds.is_none() {
//                     fee_refunds.push(identity_refunds.clone());
//                     continue;
//                 }
//
//                 for (epoch_index, credits) in identity_refunds.credits_per_epoch.iter() {
//                     if let Some(ref mut refunds) = existing_identity_refunds {
//                         let epoch = refunds
//                             .credits_per_epoch
//                             .entry(epoch_index.to_string())
//                             .or_default();
//
//                         *epoch = epoch
//                             .checked_add(*credits)
//                             .ok_or(NonConsensusError::Overflow("credits per epoch are too big"))?
//                     }
//                 }
//             }
//         }
//     }
//
//     Ok(DummyFeesResult {
//         storage: storage_fee,
//         processing: processing_fee,
//         fee_refunds,
//     })
// }
