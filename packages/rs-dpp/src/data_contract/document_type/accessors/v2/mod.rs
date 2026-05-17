/// Trait providing getters for DocumentTypeV2-specific fields.
pub trait DocumentTypeV2Getters {
    /// Returns whether documents of this type are countable.
    /// When true, the primary key tree uses a CountTree enabling O(1) total document count queries.
    fn documents_countable(&self) -> bool;

    /// Returns whether this document type supports range countable.
    /// When true, the primary key tree uses a ProvableCountTree.
    /// Implies documents_countable = true.
    fn range_countable(&self) -> bool;
}

/// Trait providing setters for DocumentTypeV2-specific fields.
pub trait DocumentTypeV2Setters {
    /// Sets whether documents of this type are countable.
    fn set_documents_countable(&mut self, countable: bool);

    /// Sets whether this document type supports range countable.
    fn set_range_countable(&mut self, range_countable: bool);
}
