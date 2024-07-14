use dpp::block::epoch::EpochIndex;
use std::fmt;

/// Struct containing the number of proposers to be paid and the index of the epoch
/// they're to be paid from.
#[derive(PartialEq, Eq, Debug)]
pub struct ProposersPayouts {
    /// Number of proposers to be paid
    pub proposers_paid_count: u16,
    /// Index of last epoch marked as paid
    pub paid_epoch_index: EpochIndex,
}

impl fmt::Display for ProposersPayouts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ProposersPayouts {{")?;
        writeln!(
            f,
            "    proposers_paid_count: {},",
            self.proposers_paid_count
        )?;
        writeln!(f, "    paid_epoch_index: {}", self.paid_epoch_index)?;
        write!(f, "}}")
    }
}
