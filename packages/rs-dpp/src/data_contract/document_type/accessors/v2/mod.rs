use crate::data_contract::GroupContractPosition;

/// Trait providing getters for DocumentType V2-specific fields.
pub trait DocumentTypeV2Getters {
    /// Returns the creation restriction group position, if applicable.
    fn creation_restriction_group(&self) -> Option<GroupContractPosition>;
}

/// Trait providing setters for DocumentType V2-specific fields.
pub trait DocumentTypeV2Setters {
    /// Sets the creation restriction group position.
    fn set_creation_restriction_group(&mut self, group: Option<GroupContractPosition>);
}
