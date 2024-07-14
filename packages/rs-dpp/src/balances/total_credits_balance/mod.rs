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
            "    total_specialized_balances: {}",
            self.total_specialized_balances
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

        if total_credits_in_platform > MAX_CREDITS {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Total credits in platform more than max credits size".to_string(),
            ));
        }

        let total_from_trees = (total_in_pools)
            .checked_add(total_identity_balances)
            .and_then(|partial_sum| partial_sum.checked_add(total_specialized_balances))
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
            ..
        } = *self;

        let total_in_trees = total_in_pools
            .checked_add(total_identity_balances)
            .and_then(|partial_sum| partial_sum.checked_add(total_specialized_balances))
            .ok_or(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Overflow of total credits".to_string(),
            ))?;

        Ok(total_in_trees.to_unsigned())
    }
}
