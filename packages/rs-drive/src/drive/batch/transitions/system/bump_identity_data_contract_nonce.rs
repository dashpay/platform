use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::IdentityOperation;
use crate::drive::batch::{DriveOperation, IdentityOperationType};
use crate::error::Error;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
    BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0,
};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BumpIdentityDataContractNonceAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
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
}
