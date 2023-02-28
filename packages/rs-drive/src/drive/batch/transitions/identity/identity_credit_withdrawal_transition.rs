use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::prelude::Identity;
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::batch::drive_op_batch::WithdrawalOperationType;
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation, WithdrawalOperation};
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let IdentityCreditWithdrawalTransition {
            protocol_version, transition_type, identity_id, amount, core_fee_per_byte, pooling, output_script, revision, signature_public_key_id, signature, execution_context
        } = self;

        let mut drive_operations = vec![];
        /// We must create the contract
        //todo:
        drive_operations.push(WithdrawalOperation(WithdrawalOperationType::UpdateIndexCounter {});

        Ok(drive_operations)
    }
}