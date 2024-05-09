use crate::document::document_methods::DocumentGetRawForDocumentTypeV0;
use crate::document::DocumentV0Getters;

pub trait DocumentIsEqualIgnoringTimestampsV0:
    DocumentV0Getters + DocumentGetRawForDocumentTypeV0
{
    /// Checks to see if a document is equal without time based fields.
    /// Since these fields are set on the network this function can be useful to make sure that
    /// fields that were supplied have not changed, while ignoring those that are set network side.
    /// Time based fields that are ignored are
    ///     created_at/updated_at
    ///     created_at_block_height/updated_at_block_height
    ///     created_at_core_block_height/updated_at_core_block_height
    fn is_equal_ignoring_time_based_fields_v0(&self, rhs: &Self) -> bool {
        self.id() == rhs.id()
            && self.owner_id() == rhs.owner_id()
            && self.properties() == rhs.properties()
            && self.revision() == rhs.revision()
    }
}
