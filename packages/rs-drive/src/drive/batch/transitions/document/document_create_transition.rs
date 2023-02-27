use dpp::data_contract::DriveContractExt;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::DocumentCreateTransition;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for DocumentCreateTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let DocumentCreateTransition {
            base, entropy, created_at, updated_at, data
        } = self;

        let DocumentBaseTransition {
            id, document_type, action, data_contract_id, data_contract
        } = &base;

        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(DocumentOperation(DocumentOperationType::AddDocumentForContract { document_and_contract_info: DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo { document_info: (), owner_id: None },
            contract: data_contract,
            document_type: data_contract.document_type_for_name(document_type)?,
        }, override_document: false }));

        Ok(drive_operations)
    }
}