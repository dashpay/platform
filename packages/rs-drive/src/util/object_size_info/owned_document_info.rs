use crate::util::object_size_info::document_info::DocumentInfo;

/// Document and contract info
#[derive(Clone, Debug)]
pub struct OwnedDocumentInfo<'a> {
    /// Document info
    pub document_info: DocumentInfo<'a>,
    /// Owner ID
    pub owner_id: Option<[u8; 32]>,
}
