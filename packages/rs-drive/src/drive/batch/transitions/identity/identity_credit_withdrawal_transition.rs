use std::collections::BTreeMap;

use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{
    DocumentOperation, IdentityOperation, WithdrawalOperation,
};
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use dpp::contracts::withdrawals_contract;
use dpp::dashcore::{consensus, BlockHeader};
use dpp::document::generate_document_id;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, IdentityCreditWithdrawalTransitionAction, Pooling,
};
use dpp::system_data_contracts::load_system_data_contract;
use dpp::util::entropy_generator::generate;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransitionAction {
    fn into_high_level_drive_operations(self, epoch: &Epoch) -> Result<Vec<DriveOperation>, Error> {
        let IdentityCreditWithdrawalTransitionAction {
            prepared_withdrawal_document,
            ..
        } = self;

        let mut drive_operations = vec![];

        drive_operations.push(DocumentOperation(
            DocumentOperationType::AddWithdrawalDocument {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentWithoutSerialization((
                        prepared_withdrawal_document,
                        None,
                    )),
                    owner_id: None,
                },
            },
        ));

        Ok(drive_operations)
    }
}
