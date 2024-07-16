use crate::drive::Drive;
use crate::util::batch::drive_op_batch::{
    DocumentOperation, DocumentOperationsForContractDocumentType, UpdateOperationInfo,
};
use crate::util::batch::{DocumentOperationType, DriveOperation};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;

impl Drive {
    /// Add update multiple documents operations
    pub(super) fn add_update_multiple_documents_operations_v0<'a>(
        documents: &'a [Document],
        data_contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        drive_operation_types: &mut Vec<DriveOperation<'a>>,
    ) {
        let operations: Vec<DocumentOperation> = documents
            .iter()
            .map(|document| {
                DocumentOperation::UpdateOperation(UpdateOperationInfo {
                    document,
                    serialized_document: None,
                    owner_id: None,
                    storage_flags: None,
                })
            })
            .collect();

        if !operations.is_empty() {
            drive_operation_types.push(DriveOperation::DocumentOperation(
                DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType {
                    document_operations: DocumentOperationsForContractDocumentType {
                        operations,
                        contract: data_contract,
                        document_type,
                    },
                },
            ));
        }
    }
}
