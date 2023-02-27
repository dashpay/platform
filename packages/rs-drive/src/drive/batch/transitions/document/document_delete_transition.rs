use dpp::data_contract::DriveContractExt;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{DocumentCreateTransition, DocumentDeleteTransition};
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for DocumentDeleteTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let DocumentDeleteTransition {
            base
        } = self;

        let DocumentBaseTransition {
            id, document_type, data_contract, ..
        } = &base;

        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(DocumentOperation(DocumentOperationType::DeleteDocumentOfNamedTypeForContract {
            document_id: id.to_buffer(),
            contract: data_contract,
            document_type_name: document_type,
            owner_id: None,
        }));

        Ok(drive_operations)
    }
}