/// Getters introduced by `TokenConfigurationV1`.
///
/// Implemented for every configuration version: a V0 configuration answers `false`, 0 and
/// transferable, since neither the shielded pool opt-in, its threshold nor non-transferable
/// tokens existed before V1.
pub trait TokenConfigurationV1Getters {
    /// Whether the token has its own shielded pool.
    fn has_shielded_pool(&self) -> bool;

    /// The fewest notes the token's shielded pool must hold before tokens may leave it for a
    /// visible destination; 0 when the configuration sets none.
    fn minimum_pool_notes_for_outgoing(&self) -> u64;

    /// Whether holders may move the token to another identity.
    fn is_transferable(&self) -> bool;
}

/// Setters introduced by `TokenConfigurationV1`.
pub trait TokenConfigurationV1Setters {
    /// Sets whether the token has its own shielded pool.
    fn set_has_shielded_pool(&mut self, has_shielded_pool: bool);

    /// Sets whether holders may move the token to another identity.
    fn set_transferable(&mut self, transferable: bool);
}
