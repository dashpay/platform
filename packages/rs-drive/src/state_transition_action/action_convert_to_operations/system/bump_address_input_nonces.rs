use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNonceActionAccessorsV0, BumpAddressInputNoncesAction,
};
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::AddressFundsOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BumpAddressInputNoncesAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .bump_identity_nonce
        {
            0 => {
                // For bump address input nonces, we need to set the balance for each input address
                // The nonce is already updated in the input
                let operations: Vec<DriveOperation<'b>> = self
                    .inputs_with_remaining_balance()
                    .iter()
                    .map(|(address, (nonce, balance))| {
                        AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                            address: address.clone(),
                            nonce: *nonce,
                            balance: *balance,
                        })
                    })
                    .collect();

                Ok(operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BumpAddressInputNoncesAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
