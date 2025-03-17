/// Accessor trait for getters of `TokenKeepsHistoryRulesV0`
pub trait TokenKeepsHistoryRulesV0Getters {
    /// Returns whether transfer history is kept.
    fn keeps_transfer_history(&self) -> bool;

    /// Returns whether freezing history is kept.
    fn keeps_freezing_history(&self) -> bool;

    /// Returns whether minting history is kept.
    fn keeps_minting_history(&self) -> bool;

    /// Returns whether burning history is kept.
    fn keeps_burning_history(&self) -> bool;
}

/// Accessor trait for setters of `TokenKeepsHistoryRulesV0`
pub trait TokenKeepsHistoryRulesV0Setters {
    /// Sets whether transfer history is kept.
    fn set_keeps_transfer_history(&mut self, value: bool);

    /// Sets whether freezing history is kept.
    fn set_keeps_freezing_history(&mut self, value: bool);

    /// Sets whether minting history is kept.
    fn set_keeps_minting_history(&mut self, value: bool);

    /// Sets whether burning history is kept.
    fn set_keeps_burning_history(&mut self, value: bool);
}
