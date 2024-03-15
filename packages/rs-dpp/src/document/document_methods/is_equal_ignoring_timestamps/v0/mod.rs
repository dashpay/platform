use crate::document::document_methods::DocumentGetRawForDocumentTypeV0;
use crate::document::DocumentV0Getters;

pub trait DocumentIsEqualIgnoringTimestampsV0:
    DocumentV0Getters + DocumentGetRawForDocumentTypeV0
{
    /// Return a value given the path to its key and the document type for a contract.
    fn is_equal_ignoring_timestamps_v0(&self, rhs: &Self) -> bool {
        self.id() == rhs.id()
            && self.owner_id() == rhs.owner_id()
            && self.properties() == rhs.properties()
            && self.revision() == rhs.revision()
    }
}
