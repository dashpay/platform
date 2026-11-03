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
    /// all the tokens held in token shielded pools (0 before protocol version 14)
    pub total_token_shielded_pool_balances: SumTokenAmount,
    /// all the tokens the token marketplace holds in escrow for open listings (0 before
    /// protocol version 15)
    pub total_token_order_escrow_balances: SumTokenAmount,
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
            "    total_token_shielded_pool_balances: {},",
            self.total_token_shielded_pool_balances
        )?;
        writeln!(
            f,
            "    total_token_order_escrow_balances: {}",
            self.total_token_order_escrow_balances
        )?;
        write!(f, "}}")
    }
}
impl TotalTokensBalance {
    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, ProtocolError> {
        let TotalTokensBalance {
            total_tokens_in_platform,
            total_identity_token_balances,
            total_token_shielded_pool_balances,
            total_token_order_escrow_balances,
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

        if total_token_shielded_pool_balances < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Tokens in shielded pools are less than 0".to_string(),
            ));
        }

        // Every protocol version runs this check. The escrow term is zero below protocol
        // version 15, where `calculate_total_tokens_balance` 0 and 1 report none, so this
        // refusal and its share of the sum below cannot change their verdict.
        if total_token_order_escrow_balances < 0 {
            return Err(ProtocolError::CriticalCorruptedCreditsCodeExecution(
                "Tokens in order escrow are less than 0".to_string(),
            ));
        }

        Ok(total_tokens_in_platform == self.total_balances()?)
    }

    /// The balance side of the conservation equation: identity balances plus shielded pool
    /// balances plus the tokens held in escrow for open listings. Errors on overflow.
    pub fn total_balances(&self) -> Result<SumTokenAmount, ProtocolError> {
        self.total_identity_token_balances
            .checked_add(self.total_token_shielded_pool_balances)
            .and_then(|balances| balances.checked_add(self.total_token_order_escrow_balances))
            .ok_or_else(|| {
                ProtocolError::CriticalCorruptedCreditsCodeExecution(
                    "Overflow adding identity, shielded pool and order escrow token balances"
                        .to_string(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_count_the_tokens_held_in_order_escrow_on_the_balance_side() {
        let balance = TotalTokensBalance {
            total_tokens_in_platform: 100,
            total_identity_token_balances: 60,
            total_token_shielded_pool_balances: 15,
            total_token_order_escrow_balances: 25,
        };
        assert!(balance.ok().expect("expected no overflow"));
        assert_eq!(balance.total_balances().expect("expected no overflow"), 100);

        // Leaving the escrow out of the balance side is a broken conservation check.
        let without_escrow = TotalTokensBalance {
            total_token_order_escrow_balances: 0,
            ..balance
        };
        assert!(!without_escrow.ok().expect("expected no overflow"));
    }

    #[test]
    fn should_refuse_a_negative_order_escrow_total() {
        let balance = TotalTokensBalance {
            total_tokens_in_platform: 0,
            total_identity_token_balances: 0,
            total_token_shielded_pool_balances: 0,
            total_token_order_escrow_balances: -1,
        };
        assert!(balance.ok().is_err());
    }
}
