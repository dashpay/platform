use crate::balances::credits::SumTokenAmount;
use crate::ProtocolError;
use std::fmt;

/// The outcome of verifying token balances
#[derive(Copy, Clone, Debug)]
pub struct TotalTokensBalance {
    /// all the tokens in platform
    pub total_tokens_in_platform: SumTokenAmount,
    /// all the tokens in identity token balances
    pub total_identity_token_balances: SumTokenAmount,
    /// the issued supply of destroyed issuers whose leaves are still stored; part of both raw
    /// totals above and excluded from the active supply. Zero before the destroyed supply ledger
    /// exists.
    pub total_destroyed_supply: SumTokenAmount,
}

impl fmt::Display for TotalTokensBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TotalTokensBalance {{")?;
        writeln!(
            f,
            "    total_tokens_in_platform: {},",
            self.total_tokens_in_platform
        )?;
        writeln!(
            f,
            "    total_identity_token_balances: {},",
            self.total_identity_token_balances
        )?;
        writeln!(
            f,
            "    total_destroyed_supply: {},",
            self.total_destroyed_supply
        )?;
        writeln!(
            f,
            "    active_supply: {}",
            self.total_tokens_in_platform
                .checked_sub(self.total_destroyed_supply)
                .map(|active| active.to_string())
                .unwrap_or_else(|| "overflow".to_string())
        )?;
        write!(f, "}}")
    }
}
impl TotalTokensBalance {
    /// The supply that is still usable: the raw supply minus the destroyed supply.
    /// Errors when the two do not fit together.
    pub fn active_supply(&self) -> Result<SumTokenAmount, ProtocolError> {
        self.total_tokens_in_platform
            .checked_sub(self.total_destroyed_supply)
            .ok_or_else(|| {
                ProtocolError::CriticalCorruptedCreditsCodeExecution(
                    "Active token supply overflowed".to_string(),
                )
            })
    }

    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, ProtocolError> {
        let TotalTokensBalance {
            total_tokens_in_platform,
            total_identity_token_balances,
            total_destroyed_supply,
        } = *self;

        if total_tokens_in_platform < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Tokens in platform are less than 0".to_string(),
            ));
        }

        if total_identity_token_balances < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Tokens in identity balances are less than 0".to_string(),
            ));
        }

        if total_destroyed_supply < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Destroyed token supply is less than 0".to_string(),
            ));
        }

        // The destroyed supply is a subset of the raw supply: the leaves it excludes are still
        // stored and counted by both raw totals.
        Ok(total_tokens_in_platform == total_identity_token_balances
            && total_destroyed_supply <= total_tokens_in_platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pass_when_raw_totals_match_and_the_destroyed_supply_fits() {
        let balance = TotalTokensBalance {
            total_tokens_in_platform: 1_000,
            total_identity_token_balances: 1_000,
            total_destroyed_supply: 400,
        };

        assert!(balance.ok().expect("expected a verdict"));
        assert_eq!(balance.active_supply().expect("expected a total"), 600);
    }

    #[test]
    fn should_fail_when_the_destroyed_supply_exceeds_the_raw_supply() {
        let balance = TotalTokensBalance {
            total_tokens_in_platform: 1_000,
            total_identity_token_balances: 1_000,
            total_destroyed_supply: 1_001,
        };

        assert!(!balance.ok().expect("expected a verdict"));
    }

    #[test]
    fn should_keep_failing_on_a_raw_mismatch_whatever_the_destroyed_supply() {
        let balance = TotalTokensBalance {
            total_tokens_in_platform: 1_000,
            total_identity_token_balances: 999,
            total_destroyed_supply: 0,
        };

        assert!(!balance.ok().expect("expected a verdict"));
    }

    #[test]
    fn should_error_on_a_negative_destroyed_supply() {
        let balance = TotalTokensBalance {
            total_tokens_in_platform: 1_000,
            total_identity_token_balances: 1_000,
            total_destroyed_supply: -1,
        };

        assert!(balance.ok().is_err());
    }
}
