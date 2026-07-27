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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_correctly() {
        let payouts = ProposersPayouts {
            proposers_paid_count: 5,
            paid_epoch_index: 42,
        };
        let output = format!("{}", payouts);
        assert!(output.contains("proposers_paid_count: 5"));
        assert!(output.contains("paid_epoch_index: 42"));
        assert!(output.contains("ProposersPayouts"));
    }

    #[test]
    fn display_with_zero_values() {
        let payouts = ProposersPayouts {
            proposers_paid_count: 0,
            paid_epoch_index: 0,
        };
        let output = format!("{}", payouts);
        assert!(output.contains("proposers_paid_count: 0"));
        assert!(output.contains("paid_epoch_index: 0"));
    }

    #[test]
    fn equality() {
        let a = ProposersPayouts {
            proposers_paid_count: 3,
            paid_epoch_index: 7,
        };
        let b = ProposersPayouts {
            proposers_paid_count: 3,
            paid_epoch_index: 7,
        };
        let c = ProposersPayouts {
            proposers_paid_count: 4,
            paid_epoch_index: 7,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
