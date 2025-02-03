use crate::balances::credits::SignedTokenAmount;
use crate::ProtocolError;
#[cfg(feature = "fixtures-and-mocks")]
use bincode::Encode;
#[cfg(feature = "fixtures-and-mocks")]
use platform_serialization::de::Decode;
use std::fmt;

/// A structure where the token supply and the aggregated token account balances should always be equal
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "fixtures-and-mocks", derive(Encode, Decode))]
pub struct TotalSingleTokenBalance {
    /// the token supply
    pub token_supply: SignedTokenAmount,
    /// the sum of all user account balances
    pub aggregated_token_account_balances: SignedTokenAmount,
}

impl fmt::Display for TotalSingleTokenBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TotalSingleTokenBalance {{")?;
        writeln!(f, "    token_supply: {},", self.token_supply)?;
        writeln!(
            f,
            "    aggregated_token_account_balances: {}",
            self.aggregated_token_account_balances
        )?;
        write!(f, "}}")
    }
}
impl TotalSingleTokenBalance {
    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, ProtocolError> {
        let TotalSingleTokenBalance {
            token_supply,
            aggregated_token_account_balances,
        } = *self;

        if token_supply < 0 {
            return Err(ProtocolError::Generic(
                "Token in platform are less than 0".to_string(),
            ));
        }

        if aggregated_token_account_balances < 0 {
            return Err(ProtocolError::Generic(
                "Token in aggregated identity balances are less than 0".to_string(),
            ));
        }

        Ok(token_supply == aggregated_token_account_balances)
    }
}
