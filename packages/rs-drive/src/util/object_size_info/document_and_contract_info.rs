use crate::util::object_size_info::OwnedDocumentInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

/// Document and contract info
#[derive(Clone, Debug)]
pub struct DocumentAndContractInfo<'a> {
    /// Document info
    pub owned_document_info: OwnedDocumentInfo<'a>,
    ///DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
}
