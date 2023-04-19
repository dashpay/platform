use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::ContractOperation;
use crate::drive::batch::{ContractOperationType, DriveOperation};
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for DataContractUpdateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let DataContractUpdateTransitionAction { data_contract, .. } = self;
        let mut drive_operations = vec![];
        // We must create the contract
        drive_operations.push(ContractOperation(ContractOperationType::ApplyContract {
            contract: Cow::Owned(data_contract),
            storage_flags: None,
        }));

        Ok(drive_operations)
    }
}
