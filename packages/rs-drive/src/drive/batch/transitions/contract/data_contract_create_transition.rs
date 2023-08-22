use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::DataContractOperation;
use crate::drive::batch::{DataContractOperationType, DriveOperation};
use crate::error::Error;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for DataContractCreateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let mut drive_operations = vec![];
        // We must create the contract
        drive_operations.push(DataContractOperation(
            DataContractOperationType::ApplyContract {
                contract: Cow::Owned(self.data_contract()),
                storage_flags: None,
            },
        ));

        Ok(drive_operations)
    }
}
