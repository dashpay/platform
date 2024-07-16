use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::bump_identity_nonce_action::{
    BumpIdentityNonceAction, BumpIdentityNonceActionAccessorsV0,
};
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BumpIdentityNonceAction {
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
                let identity_id = self.identity_id();

                let identity_nonce = self.identity_nonce();

                Ok(vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce: identity_nonce,
                    },
                )])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BumpIdentityNonceAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
