use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::IdentityOperation;
use crate::drive::batch::{DriveOperation, IdentityOperationType};
use crate::error::Error;
use crate::state_transition_action::system::bump_identity_nonce_action::{
    BumpIdentityNonceAction, BumpIdentityNonceActionAccessorsV0,
};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BumpIdentityNonceAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let identity_id = self.identity_id();

        let identity_nonce = self.identity_nonce();

        Ok(vec![IdentityOperation(
            IdentityOperationType::UpdateIdentityNonce {
                identity_id: identity_id.into_buffer(),
                nonce: identity_nonce,
            },
        )])
    }
}
