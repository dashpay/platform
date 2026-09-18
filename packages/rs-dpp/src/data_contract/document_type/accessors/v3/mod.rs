/// Getters introduced with document type version 3: the keep-history document
/// lifecycle keywords of meta-schema v4.
pub trait DocumentTypeV3Getters {
    /// Returns whether a deleted document of this type may have its retained
    /// revisions purged by an erase transition. Always false for a document
    /// type parsed by an earlier grammar, which has no such keyword.
    fn documents_can_be_erased(&self) -> bool;
}
