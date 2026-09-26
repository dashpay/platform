use crate::balances::credits::{Creditable, MAX_CREDITS};
use crate::fee::{Credits, SignedCredits};
use crate::ProtocolError;
use std::fmt;

/// The outcome of verifying credits
#[derive(Copy, Clone, Debug)]
pub struct TotalCreditsBalance {
    /// all the credits in platform
    pub total_credits_in_platform: Credits,
    /// all the credits in distribution pools
    pub total_in_pools: SignedCredits,
    /// all the credits in identity balances
    pub total_identity_balances: SignedCredits,
    /// all the credits in specialized balances
    pub total_specialized_balances: SignedCredits,
    /// all the credits in addresses
    pub total_in_addresses: SignedCredits,
    /// all the credits inside shielded credit pools
    pub total_in_shielded_balances: SignedCredits,
    /// all the live credits held by contracts in their credit buckets; a
    /// wiped contract's retained credits are excluded by the tree layout
    pub total_in_contract_credits: SignedCredits,
}

impl fmt::Display for TotalCreditsBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TotalCreditsBalance {{")?;
        writeln!(
            f,
            "    total_credits_in_platform: {},",
            self.total_credits_in_platform
        )?;
        writeln!(f, "    total_in_pools: {},", self.total_in_pools)?;
        writeln!(
            f,
            "    total_identity_balances: {},",
            self.total_identity_balances
        )?;
        writeln!(
            f,
            "    total_specialized_balances: {},",
            self.total_specialized_balances
        )?;
        writeln!(
            f,
            "    total_addresses_balances: {},",
            self.total_in_addresses
        )?;
        writeln!(
            f,
            "    total_in_shielded_balances: {},",
            self.total_in_shielded_balances
        )?;
        writeln!(
            f,
            "    total_in_contract_credits: {}",
            self.total_in_contract_credits
        )?;
        write!(f, "}}")
    }
}

impl TotalCreditsBalance {
    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, ProtocolError> {
        let TotalCreditsBalance {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
            total_specialized_balances,
            total_in_addresses,
            total_in_shielded_balances,
            total_in_contract_credits,
        } = *self;

        if total_in_pools < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Credits in distribution pools are less than 0".to_string(),
            ));
        }

        if total_identity_balances < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Credits of identity balances are less than 0".to_string(),
            ));
        }

        if total_specialized_balances < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Credits of specialized balances are less than 0".to_string(),
            ));
        }

        if total_in_addresses < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Credits of addresses are less than 0".to_string(),
            ));
        }

        if total_in_shielded_balances < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Credits inside shielded balances are less than 0".to_string(),
            ));
        }

        if total_in_contract_credits < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Credits held by contracts are less than 0".to_string(),
            ));
        }

        if total_credits_in_platform > MAX_CREDITS {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Total credits in platform more than max credits size".to_string(),
            ));
        }

        let total_from_trees = (total_in_pools)
            .checked_add(total_identity_balances)
            .and_then(|partial_sum| partial_sum.checked_add(total_specialized_balances))
            .and_then(|partial_sum| partial_sum.checked_add(total_in_addresses))
            .and_then(|partial_sum| partial_sum.checked_add(total_in_shielded_balances))
            .and_then(|partial_sum| partial_sum.checked_add(total_in_contract_credits))
            .ok_or(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Overflow of total credits".to_string(),
            ))?;

        Ok(total_credits_in_platform.to_signed()? == total_from_trees)
    }

    /// Get the total in all trees
    pub fn total_in_trees(&self) -> Result<Credits, ProtocolError> {
        let TotalCreditsBalance {
            total_in_pools,
            total_identity_balances,
            total_specialized_balances,
            total_in_addresses,
            total_in_shielded_balances,
            total_in_contract_credits,
            ..
        } = *self;

        let total_in_trees = total_in_pools
            .checked_add(total_identity_balances)
            .and_then(|partial_sum| partial_sum.checked_add(total_specialized_balances))
            .and_then(|partial_sum| partial_sum.checked_add(total_in_addresses))
            .and_then(|partial_sum| partial_sum.checked_add(total_in_shielded_balances))
            .and_then(|partial_sum| partial_sum.checked_add(total_in_contract_credits))
            .ok_or(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Overflow of total credits".to_string(),
            ))?;

        Ok(total_in_trees.to_unsigned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced() -> TotalCreditsBalance {
        TotalCreditsBalance {
            total_credits_in_platform: 600,
            total_in_pools: 100,
            total_identity_balances: 100,
            total_specialized_balances: 100,
            total_in_addresses: 100,
            total_in_shielded_balances: 100,
            total_in_contract_credits: 100,
        }
    }

    #[test]
    fn should_count_contract_credits_as_a_term_of_the_equation() {
        let balance = balanced();
        assert!(balance.ok().expect("no overflow"));
        assert_eq!(balance.total_in_trees().expect("no overflow"), 600);

        let short = TotalCreditsBalance {
            total_in_contract_credits: 0,
            ..balance
        };
        assert!(
            !short.ok().expect("no overflow"),
            "dropping the contract credits term must unbalance the equation"
        );
    }

    #[test]
    fn should_reject_negative_contract_credits() {
        let negative = TotalCreditsBalance {
            total_in_contract_credits: -1,
            total_credits_in_platform: 499,
            ..balanced()
        };
        assert!(matches!(
            negative.ok(),
            Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(_))
        ));
    }

    #[test]
    fn should_report_overflow_when_the_contract_credits_term_overflows_the_sum() {
        let overflowing = TotalCreditsBalance {
            total_in_contract_credits: SignedCredits::MAX,
            ..balanced()
        };
        assert!(matches!(
            overflowing.ok(),
            Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(_))
        ));
        assert!(matches!(
            overflowing.total_in_trees(),
            Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(_))
        ));
    }
}
