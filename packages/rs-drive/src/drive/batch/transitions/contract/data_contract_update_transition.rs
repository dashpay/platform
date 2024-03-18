use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{DataContractOperation, IdentityOperation};
use crate::drive::batch::{DataContractOperationType, DriveOperation, IdentityOperationType};
use crate::error::Error;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for DataContractUpdateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        Ok(vec![
            // We must create the contract
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id: self.data_contract_ref().owner_id().into_buffer(),
                contract_id: self.data_contract_ref().id().into_buffer(),
                nonce: self.identity_contract_nonce(),
            }),
            DataContractOperation(DataContractOperationType::ApplyContract {
                contract: Cow::Owned(self.data_contract()),
                storage_flags: None,
            }),
        ])
    }
}
