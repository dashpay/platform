//! Whether every property of a contested index is a top-level property of the document. The
//! vote poll of a contest takes each index value from the document's top-level properties by
//! name, so a nested or system property gives the poll a null value, and no contested create
//! can open or join the contest.

use crate::data_contract::document_type::Index;

impl Index {
    /// Whether this index's contest, if it has one, is keyed only by top-level properties of
    /// the document: no property path (`records.identity`) and no system property
    /// (`$ownerId`). A contested create keys its vote poll by looking each index property up
    /// by name among the document's top-level properties ([`Index::extract_values`]), where a
    /// property path or a system property is never found. The poll then holds a null value
    /// where the contender's index entry holds the real one, and Drive fails every contested
    /// create of the type without storing it, so the contest can never start. An index without
    /// a contest is keyed by any of its properties.
    pub fn contest_is_keyed_by_top_level_properties(&self) -> bool {
        self.contested_index.is_none()
            || self
                .properties
                .iter()
                .all(|property| !property.name.starts_with('$') && !property.name.contains('.'))
    }
}
