use dpp::data_contract::DriveContractExt;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::DocumentReplaceTransition;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for DocumentReplaceTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let DocumentReplaceTransition {
            base, revision, updated_at, data
        } = self;

        let DocumentBaseTransition {
            id, document_type_name, action, data_contract_id, data_contract
        } = &base;

        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(DocumentOperation(DocumentOperationType::UpdateDocumentForContract {
            document: &DocumentStub {},
            serialized_document: &[],
            contract: &Default::default(),
            document_type_name,
            owner_id: None,
            storage_flags: None,
        }));

        Ok(drive_operations)
    }
}