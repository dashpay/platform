#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum TokenDistributionRules {
    #[serde(rename = "0")]
    V0(TokenDistributionRulesV0),
}

use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use std::fmt;

impl fmt::Display for TokenDistributionRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use platform_value::{platform_value, Identifier, Value};
    use serde_json::json;

    /// Non-default values per inner field (set destination_identity to a
    /// specific identifier and `minting_allow_choosing_destination` to true)
    /// so the wire-shape assertion catches silent zero-out / flip on round-trip.
    fn fixture() -> TokenDistributionRules {
        let ccr = || ChangeControlRules::V0(ChangeControlRulesV0::default());
        TokenDistributionRules::V0(TokenDistributionRulesV0 {
            perpetual_distribution: None,
            perpetual_distribution_rules: ccr(),
            pre_programmed_distribution: None,
            new_tokens_destination_identity: Some(Identifier::new([0x42; 32])),
            new_tokens_destination_identity_rules: ccr(),
            minting_allow_choosing_destination: true,
            minting_allow_choosing_destination_rules: ccr(),
            change_direct_purchase_pricing_rules: ccr(),
        })
    }

    fn default_ccr_json() -> serde_json::Value {
        json!({
            "$formatVersion": "0",
            "authorizedToMakeChange": {"$type": "noOne"},
            "adminActionTakers": {"$type": "noOne"},
            "changingAuthorizedActionTakersToNoOneAllowed": false,
            "changingAdminActionTakersToNoOneAllowed": false,
            "selfChangingAdminActionTakersAllowed": false,
        })
    }

    fn default_ccr_value() -> Value {
        platform_value!({
            "$formatVersion": "0",
            "authorizedToMakeChange": {"$type": "noOne"},
            "adminActionTakers": {"$type": "noOne"},
            "changingAuthorizedActionTakersToNoOneAllowed": false,
            "changingAdminActionTakersToNoOneAllowed": false,
            "selfChangingAdminActionTakersAllowed": false,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `Identifier` renders as base58 string in JSON. None Options become
        // `null`. Inner `ChangeControlRules` round-trips its own envelope.
        // No sized integers in this fixture.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "perpetualDistribution": null,
                "perpetualDistributionRules": default_ccr_json(),
                "preProgrammedDistribution": null,
                "newTokensDestinationIdentity": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                "newTokensDestinationIdentityRules": default_ccr_json(),
                "mintingAllowChoosingDestination": true,
                "mintingAllowChoosingDestinationRules": default_ccr_json(),
                "changeDirectPurchasePricingRules": default_ccr_json(),
            })
        );
        let recovered = TokenDistributionRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `Identifier`'s Serialize emits `Value::Identifier`; interpolating the
        // Identifier through `platform_value!{...}` runs Serialize and produces
        // the typed variant. None becomes `Value::Null`.
        let id = Identifier::new([0x42; 32]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "perpetualDistribution": Value::Null,
                "perpetualDistributionRules": default_ccr_value(),
                "preProgrammedDistribution": Value::Null,
                "newTokensDestinationIdentity": id,
                "newTokensDestinationIdentityRules": default_ccr_value(),
                "mintingAllowChoosingDestination": true,
                "mintingAllowChoosingDestinationRules": default_ccr_value(),
                "changeDirectPurchasePricingRules": default_ccr_value(),
            })
        );
        let recovered = TokenDistributionRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
