use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::drive::object_size_info::OwnedDocumentInfo;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use crate::state_transition_action::document::documents_batch::document_transition::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0};

impl DriveHighLevelDocumentOperationConverter for BumpIdentityDataContractNonceAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let data_contract_id = self.data_contract_id();

        let identity_contract_nonce = self.identity_contract_nonce();

        Ok(vec![IdentityOperation(
            IdentityOperationType::UpdateIdentityContractNonce {
                identity_id: owner_id.into_buffer(),
                contract_id: data_contract_id.into_buffer(),
                nonce: identity_contract_nonce,
            },
        )])
    }
}
