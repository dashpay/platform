use dpp::block::epoch::EpochIndex;

/// Struct containing the number of proposers to be paid and the index of the epoch
/// they're to be paid from.
#[derive(PartialEq, Eq, Debug)]
pub struct ProposersPayouts {
    /// Number of proposers to be paid
    pub proposers_paid_count: u16,
    /// Index of last epoch marked as paid
    pub paid_epoch_index: EpochIndex,
}