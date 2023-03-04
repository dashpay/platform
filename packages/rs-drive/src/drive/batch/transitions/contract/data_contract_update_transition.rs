use std::borrow::Cow;
use dpp::data_contract::state_transition::{DataContractUpdateTransition, DataContractUpdateTransitionAction};
use crate::drive::batch::{ContractOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::ContractOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::error::Error;

impl DriveHighLevelOperationConverter for DataContractUpdateTransitionAction {
    fn to_high_level_drive_operations(self) -> Result<Vec<DriveOperation>, Error> {
        let DataContractUpdateTransitionAction {
            data_contract, ..
        } = self;
        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(ContractOperation(ContractOperationType::ApplyContract { contract: Cow::Owned(data_contract), storage_flags: None }));

        Ok(drive_operations)
    }
}