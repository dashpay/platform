use crate::execution::types::{fees_in_pools, proposer_payouts};
use std::fmt;

/// Holds info relevant fees and a processed block
#[derive(Debug)]
pub struct ProcessedBlockFeesOutcome {
    /// Amount of fees in the storage and processing fee distribution pools
    pub fees_in_pools: fees_in_pools::v0::FeesInPoolsV0,
    /// A struct with the number of proposers to be paid out and the last paid epoch index
    pub payouts: Option<proposer_payouts::v0::ProposersPayouts>,
    /// A number of epochs which had refunded
    pub refunded_epochs_count: Option<u16>,
    /// The credits the applied batch minted into Platform (the epoch Core block rewards, on an
    /// epoch change): the fee-processing share of the block's credit inflow, recorded for the
    /// net daily withdrawal limit
    pub credit_mints: dpp::fee::Credits,
}

impl fmt::Display for ProcessedBlockFeesOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ProcessedBlockFeesOutcome {{")?;
        writeln!(f, "    fees_in_pools: {},", self.fees_in_pools)?;
        writeln!(
            f,
            "    payouts: {},",
            match &self.payouts {
                Some(payouts) => format!("{}", payouts),
                None => "None".to_string(),
            }
        )?;
        writeln!(
            f,
            "    refunded_epochs_count: {},",
            match self.refunded_epochs_count {
                Some(count) => count.to_string(),
                None => "None".to_string(),
            }
        )?;
        writeln!(f, "    credit_mints: {}", self.credit_mints)?;
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::fees_in_pools::v0::FeesInPoolsV0;
    use crate::execution::types::proposer_payouts::v0::ProposersPayouts;

    #[test]
    fn display_with_payouts_and_refunds() {
        let outcome = ProcessedBlockFeesOutcome {
            fees_in_pools: FeesInPoolsV0 {
                processing_fees: 100,
                storage_fees: 200,
            },
            payouts: Some(ProposersPayouts {
                proposers_paid_count: 3,
                paid_epoch_index: 5,
            }),
            refunded_epochs_count: Some(2),
            credit_mints: 5000,
        };
        let output = format!("{}", outcome);
        assert!(output.contains("ProcessedBlockFeesOutcome"));
        assert!(output.contains("fees_in_pools:"));
        assert!(output.contains("proposers_paid_count: 3"));
        assert!(output.contains("refunded_epochs_count: 2"));
        assert!(output.contains("credit_mints: 5000"));
    }

    #[test]
    fn display_with_no_payouts_and_no_refunds() {
        let outcome = ProcessedBlockFeesOutcome {
            fees_in_pools: FeesInPoolsV0 {
                processing_fees: 0,
                storage_fees: 0,
            },
            payouts: None,
            refunded_epochs_count: None,
            credit_mints: 0,
        };
        let output = format!("{}", outcome);
        assert!(output.contains("payouts: None"));
        assert!(output.contains("credit_mints: 0"));
        assert!(output.contains("refunded_epochs_count: None"));
    }
}
