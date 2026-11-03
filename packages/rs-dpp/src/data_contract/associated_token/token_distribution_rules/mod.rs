#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub mod accessors;
mod methods;
pub mod v0;
pub mod v1;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From, DecodeUntrusted,
)]
#[serde(tag = "$formatVersion")]
pub enum TokenDistributionRules {
    #[serde(rename = "0")]
    V0(TokenDistributionRulesV0),
    /// Version 0 plus the once-per-identity distribution (protocol version 14).
    #[serde(rename = "1")]
    V1(TokenDistributionRulesV1),
}

use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use crate::data_contract::associated_token::token_distribution_rules::v1::TokenDistributionRulesV1;
use std::fmt;

impl fmt::Display for TokenDistributionRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
            TokenDistributionRules::V1(v1) => {
                write!(f, "{}", v1) //just pass through
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
    fn v1_json_round_trip_with_once_per_identity_distribution() {
        use crate::data_contract::associated_token::token_distribution_rules::accessors::v1::{
            TokenDistributionRulesV1Getters, TokenDistributionRulesV1Setters,
        };
        use crate::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
        use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
        use crate::serialization::JsonConvertible;
        // Setting the once-per-identity distribution on V0 rules upgrades them to V1 in place.
        let mut rules = fixture();
        let distribution =
            TokenOncePerIdentityDistribution::V0(TokenOncePerIdentityDistributionV0 {
                amount: 100,
            });
        rules.set_once_per_identity_distribution(Some(distribution.clone()));
        assert!(matches!(rules, TokenDistributionRules::V1(_)));
        assert_eq!(rules.once_per_identity_distribution(), Some(&distribution));

        let json = rules.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "1",
                "perpetualDistribution": null,
                "perpetualDistributionRules": default_ccr_json(),
                "preProgrammedDistribution": null,
                "newTokensDestinationIdentity": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                "newTokensDestinationIdentityRules": default_ccr_json(),
                "mintingAllowChoosingDestination": true,
                "mintingAllowChoosingDestinationRules": default_ccr_json(),
                "changeDirectPurchasePricingRules": default_ccr_json(),
                "oncePerIdentityDistribution": {
                    "$formatVersion": "0",
                    "amount": 100,
                },
            })
        );
        let recovered = TokenDistributionRules::from_json(json).expect("from_json");
        assert_eq!(rules, recovered);
    }

    #[test]
    fn clearing_once_per_identity_distribution_downgrades_v1_rules_to_v0() {
        use crate::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
        use crate::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
        use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
        // Version 0 stays the wire format whenever the distribution is unset, so clearing it
        // takes the rules back to the exact version 0 they were upgraded from.
        let mut rules = fixture();
        rules.set_once_per_identity_distribution(Some(TokenOncePerIdentityDistribution::V0(
            TokenOncePerIdentityDistributionV0 { amount: 100 },
        )));
        assert!(matches!(rules, TokenDistributionRules::V1(_)));
        rules.set_once_per_identity_distribution(None);
        assert_eq!(rules, fixture());
    }

    #[test]
    fn v0_rules_stay_v0_when_clearing_once_per_identity_distribution() {
        use crate::data_contract::associated_token::token_distribution_rules::accessors::v1::{
            TokenDistributionRulesV1Getters, TokenDistributionRulesV1Setters,
        };
        let mut rules = fixture();
        rules.set_once_per_identity_distribution(None);
        assert_eq!(rules, fixture());
        assert_eq!(rules.once_per_identity_distribution(), None);
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
