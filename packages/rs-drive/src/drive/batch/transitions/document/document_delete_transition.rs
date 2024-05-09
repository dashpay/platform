use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;

use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;

use dpp::identifier::Identifier;
use std::borrow::Cow;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;

impl DriveHighLevelDocumentOperationConverter for DocumentDeleteTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let base = self.base_owned();

        let data_contract_id = base.data_contract_id();

        let identity_contract_nonce = base.identity_contract_nonce();

        Ok(vec![
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
                identity_id: owner_id.into_buffer(),
                contract_id: data_contract_id.into_buffer(),
                nonce: identity_contract_nonce,
            }),
            DocumentOperation(
                DocumentOperationType::DeleteDocumentOfNamedTypeForContractId {
                    document_id: base.id().to_buffer(),
                    contract_id: base.data_contract_id().to_buffer(),
                    document_type_name: Cow::Owned(base.document_type_name_owned()),
                },
            ),
        ])
    }
}
