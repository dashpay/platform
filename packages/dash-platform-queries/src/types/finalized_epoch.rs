//! Finalized epoch related types and helpers
use dpp::block::epoch::EpochIndex;

/// Query used to fetch multiple finalized epochs from Platform.
#[derive(Clone, Debug)]
pub struct FinalizedEpochQuery {
    /// Starting epoch index.
    pub start_epoch_index: EpochIndex,
    /// Whether to include the start epoch.
    pub start_epoch_index_included: bool,
    /// Ending epoch index.
    pub end_epoch_index: EpochIndex,
    /// Whether to include the end epoch.
    pub end_epoch_index_included: bool,
}

impl Default for FinalizedEpochQuery {
    fn default() -> Self {
        Self {
            start_epoch_index: 0,
            start_epoch_index_included: true,
            end_epoch_index: 0,
            end_epoch_index_included: true,
        }
    }
}

impl From<(EpochIndex, EpochIndex)> for FinalizedEpochQuery {
    fn from((start, end): (EpochIndex, EpochIndex)) -> Self {
        Self {
            start_epoch_index: start,
            start_epoch_index_included: true,
            end_epoch_index: end,
            end_epoch_index_included: true,
        }
    }
}
