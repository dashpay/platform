use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use dpp::data_contract::DriveContractExt;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{DocumentCreateTransition, DocumentDeleteTransition};
use dpp::identifier::Identifier;

impl DriveHighLevelDocumentOperationConverter for DocumentDeleteTransition {
    fn into_high_level_document_drive_operations(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
    ) -> Result<Vec<DriveOperation>, Error> {
        let DocumentDeleteTransition { base } = self;

        let DocumentBaseTransition {
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
                document_type_name,
                owner_id: None,
            },
        ));

        Ok(drive_operations)
    }
}
