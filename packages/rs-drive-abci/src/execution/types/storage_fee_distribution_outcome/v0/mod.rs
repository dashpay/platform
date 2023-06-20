use drive::fee::credits::Credits;

/// Result of storage fee distribution
pub struct StorageFeeDistributionOutcome {
    /// Leftovers in result of divisions and rounding after storage fee distribution to epochs
    pub leftovers: Credits,
    /// A number of epochs which had refunded
    pub refunded_epochs_count: u16,
}