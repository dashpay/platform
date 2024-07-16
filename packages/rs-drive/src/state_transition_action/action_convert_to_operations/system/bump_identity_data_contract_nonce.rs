use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
    BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0,
};
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
impl DriveHighLevelOperationConverter for BumpIdentityDataContractNonceAction {
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
            .bump_identity_data_contract_nonce
        {
            0 => {
                let identity_id = self.identity_id();
                let data_contract_id = self.data_contract_id();

                let identity_contract_nonce = self.identity_contract_nonce();

                Ok(vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: identity_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BumpIdentityDataContractNonceAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
