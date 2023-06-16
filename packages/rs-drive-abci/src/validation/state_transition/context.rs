use dpp::dashcore::TxOut;

/// Validation context to share data required for validation between validation functions
/// It allows to avoid fetching the same data multiple times
#[derive(Debug, Default)]
pub struct ValidationDataShareContext {
    /// Asset Lock Output
    pub asset_lock_output: Option<TxOut>,
}
