use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::util::batch::DriveOperation::{DataContractOperation, IdentityOperation};
use crate::util::batch::{DataContractOperationType, DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for DataContractUpdateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .data_contract_update_transition
        {
            0 => {
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DataContractUpdateTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
