use crate::util::object_size_info::OwnedDocumentInfo;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
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

impl DocumentAndContractInfo<'_> {
    /// The same write without storage flags when the document type declares a `ttl`: such a
    /// document is stored flagless and refunds nothing. Every other write is unchanged. Done
    /// before any element is sized, so sizes, replacements and patches all agree with what is
    /// stored.
    pub fn without_storage_flags_if_expiring(self) -> Self {
        if self.document_type.documents_ttl_seconds().is_none() {
            return self;
        }
        let DocumentAndContractInfo {
            owned_document_info,
            contract,
            document_type,
        } = self;
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: owned_document_info.document_info.without_storage_flags(),
                owner_id: owned_document_info.owner_id,
            },
            contract,
            document_type,
        }
    }
}
