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
    /// all the tokens held in token shielded pools (0 before protocol version 15)
    pub total_token_shielded_pool_balances: SumTokenAmount,
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
            "    total_token_shielded_pool_balances: {}",
            self.total_token_shielded_pool_balances
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

        let total_balances = total_identity_token_balances
            .checked_add(total_token_shielded_pool_balances)
            .ok_or_else(|| {
                ProtocolError::CriticalCorruptedCreditsCodeExecution(
                    "Overflow adding identity and shielded pool token balances".to_string(),
                )
            })?;

        Ok(total_tokens_in_platform == total_balances)
    }

    /// The balance side of the conservation equation: identity balances plus shielded pool
    /// balances. Errors on overflow.
    pub fn total_balances(&self) -> Result<SumTokenAmount, ProtocolError> {
        self.total_identity_token_balances
            .checked_add(self.total_token_shielded_pool_balances)
            .ok_or_else(|| {
                ProtocolError::CriticalCorruptedCreditsCodeExecution(
                    "Overflow adding identity and shielded pool token balances".to_string(),
                )
            })
    }
}
