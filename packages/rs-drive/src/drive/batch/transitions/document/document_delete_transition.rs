use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;

use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::{DocumentOperationType, DriveOperation};

use crate::error::Error;
use dpp::block::epoch::Epoch;

use dpp::identifier::Identifier;
use std::borrow::Cow;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use dpp::version::PlatformVersion;

impl DriveHighLevelDocumentOperationConverter for DocumentDeleteTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        _owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let base = self.base_owned();

        let mut drive_operations = vec![];
        drive_operations.push(DocumentOperation(
            DocumentOperationType::DeleteDocumentOfNamedTypeForContractId {
                document_id: base.id().to_buffer(),
                contract_id: base.data_contract_id().to_buffer(),
                document_type_name: Cow::Owned(base.document_type_name_owned()),
            },
        ));

        Ok(drive_operations)
    }
}
