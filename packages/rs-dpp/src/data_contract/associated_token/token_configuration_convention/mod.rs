use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;
pub mod methods;
pub mod v0;

/// Versioned wrapper for token display conventions.
///
/// `TokenConfigurationConvention` provides a flexible, forward-compatible structure
/// for representing human-readable metadata about a token, such as localized names
/// and decimal formatting standards.
///
/// This enum enables evolution of the convention schema over time without breaking
/// compatibility with older tokens. Each variant defines a specific format version.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, From)]
#[serde(tag = "$format_version")]
pub enum TokenConfigurationConvention {
    /// Version 0 of the token convention schema.
    ///
    /// Defines localized names (by ISO 639 language codes) and the number of decimal places
    /// used for displaying token amounts.
    #[serde(rename = "0")]
    V0(TokenConfigurationConventionV0),
}

impl fmt::Display for TokenConfigurationConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfigurationConvention::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
