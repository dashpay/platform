/// Getters introduced by `TokenConfigurationV1`.
///
/// Implemented for every configuration version: a V0 configuration answers `false`, since
/// the shielded pool opt-in did not exist before V1.
pub trait TokenConfigurationV1Getters {
    /// Whether the token has its own shielded pool.
    fn has_shielded_pool(&self) -> bool;
}

/// Setters introduced by `TokenConfigurationV1`.
pub trait TokenConfigurationV1Setters {
    /// Sets whether the token has its own shielded pool.
    fn set_has_shielded_pool(&mut self, has_shielded_pool: bool);
}
