mod accessors;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct TokenConfigurationLocalizationV0 {
    pub should_capitalize: bool,
    pub singular_form: String,
    pub plural_form: String,
}

impl fmt::Display for TokenConfigurationLocalizationV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Capitalized: {}, Singular: '{}', Plural: '{}'",
            self.should_capitalize, self.singular_form, self.plural_form
        )
    }
}
