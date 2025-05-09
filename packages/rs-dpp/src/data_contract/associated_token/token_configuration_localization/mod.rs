use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
use bincode::Encode;
use derive_more::From;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;
pub mod v0;

/// Versioned wrapper for token name localization data.
///
/// `TokenConfigurationLocalization` allows extensibility for future schema upgrades
/// while preserving backward compatibility. Each variant represents a specific format
/// version for localization information.
///
/// This structure is used to map language codes to localized token names in a flexible,
/// forward-compatible manner.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, From)]
#[serde(tag = "$format_version")]
pub enum TokenConfigurationLocalization {
    /// Version 0 of the token localization schema.
    ///
    /// Defines basic capitalization preference, singular form, and plural form
    /// for displaying token names.
    #[serde(rename = "0")]
    V0(TokenConfigurationLocalizationV0),
}

impl fmt::Display for TokenConfigurationLocalization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfigurationLocalization::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
