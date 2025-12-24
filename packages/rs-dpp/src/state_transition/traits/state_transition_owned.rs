use platform_value::Identifier;

pub trait StateTransitionOwned {
    /// Get owner ID
    fn owner_id(&self) -> Identifier;
}
