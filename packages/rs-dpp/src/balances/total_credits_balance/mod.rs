use crate::state_transition::fee::Credits;

/// The outcome of verifying credits
#[derive(Copy, Clone, Debug)]
pub struct TotalCreditsBalance {
    /// all the credits in platform
    pub total_credits_in_platform: Credits,
    /// all the credits in distribution pools
    pub total_in_pools: SignedCredits,
    /// all the credits in identity balances
    pub total_identity_balances: SignedCredits,
}

impl TotalCreditsBalance {
    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, Error> {
        let TotalCreditsBalance {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
        } = *self;

        if total_in_pools < 0 {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits in distribution pools are less than 0",
            )));
        }

        if total_identity_balances < 0 {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits of identity balances are less than 0",
            )));
        }

        if total_credits_in_platform > MAX_CREDITS {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Total credits in platform more than max credits size",
            )));
        }

        let total_from_trees = (total_in_pools)
            .checked_add(total_identity_balances)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Overflow of total credits",
            )))?;

        Ok(total_credits_in_platform.to_signed()? == total_from_trees)
    }

    /// Get the total in all trees
    pub fn total_in_trees(&self) -> Result<Credits, Error> {
        let TotalCreditsBalance {
            total_in_pools,
            total_identity_balances,
            ..
        } = *self;

        let total_in_trees =
            total_in_pools
                .checked_add(total_identity_balances)
                .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                    "Overflow of total credits",
                )))?;

        Ok(total_in_trees.to_unsigned())
    }
}