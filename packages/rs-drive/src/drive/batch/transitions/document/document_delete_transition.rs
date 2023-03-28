use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;

use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::{DocumentOperationType, DriveOperation};

use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

use dpp::document::document_transition::{
    DocumentBaseTransitionAction, DocumentDeleteTransitionAction,
};
use dpp::identifier::Identifier;
use std::borrow::Cow;

impl DriveHighLevelDocumentOperationConverter for DocumentDeleteTransitionAction {
    fn into_high_level_document_drive_operations(
        self,
        _epoch: &Epoch,
        _owner_id: Identifier,
    ) -> Result<Vec<DriveOperation>, Error> {
        let DocumentDeleteTransitionAction { base } = self;

        let DocumentBaseTransitionAction {
            id,
            document_type_name,
            data_contract_id,
            ..
        } = base;

        let mut drive_operations = vec![];
        drive_operations.push(DocumentOperation(
            DocumentOperationType::DeleteDocumentOfNamedTypeForContractId {
                document_id: id.to_buffer(),
                contract_id: data_contract_id.to_buffer(),
                document_type_name: Cow::Owned(document_type_name),
                owner_id: None,
            },
        ));

        Ok(drive_operations)
    }
}
