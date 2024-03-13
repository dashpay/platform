use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{DataContractOperation, IdentityOperation};
use crate::drive::batch::{DataContractOperationType, DriveOperation, IdentityOperationType};
use crate::error::Error;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for DataContractCreateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let mut operations = vec![];

        operations.push(IdentityOperation(
            IdentityOperationType::UpdateIdentityNonce {
                identity_id: self.data_contract_ref().owner_id().into_buffer(),
                nonce: self.identity_nonce(),
            },
        ));

        if !self.data_contract_ref().config().readonly() {
            // This means we can update the data contract, so we should add an identity contract
            // nonce now to make it so there are no additional bytes used later for bumping the
            // identity data contract nonce
            operations.push(IdentityOperation(
                IdentityOperationType::UpdateIdentityContractNonce {
                    identity_id: self.data_contract_ref().owner_id().into_buffer(),
                    contract_id: self.data_contract_ref().id().into_buffer(),
                    nonce: 0,
                },
            ));
        }

        operations.push(DataContractOperation(
            DataContractOperationType::ApplyContract {
                contract: Cow::Owned(self.data_contract()),
                storage_flags: None,
            },
        ));

        Ok(operations)
    }
}
