#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::tokens::token_event::TokenEvent;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    // Internally tagged with `$kind`. The inner `TokenEvent` is itself
    // internally tagged with `$type`, so its fields flatten alongside `$kind`
    // into one object. `$kind` keeps the two discriminators distinct. Wire
    // shape, e.g.:
    //   {"$kind": "tokenEvent", "$type": "mint", "amount": <n>, "recipient": <id>}
    serde(tag = "$kind", rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum GroupActionEvent {
    TokenEvent(TokenEvent),
}

// Manual impl because GroupActionEvent is a flat enum (not versioned V0/V1).
// Its inner type TokenEvent also has a manual impl — see token_event.rs.
#[cfg(feature = "json-conversion")]
impl JsonConvertible for GroupActionEvent {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `GroupActionEvent` uses `tag = "$kind"` (internal). Plain `kind`
    // (no `$` prefix) because the wire level has no other `$`-prefixed
    // fields. Distinct from the inner `TokenEvent`'s `type` discriminator
    // — both keys coexist at the flattened top level without collision.

    #[test]
    fn json_round_trip_token_event_mint() {
        use crate::serialization::JsonConvertible;
        let original = GroupActionEvent::TokenEvent(
            crate::tokens::token_event::json_convertible_tests::mint_fixture(),
        );
        let json = original.to_json().expect("to_json");
        // Outer `kind: "tokenEvent"` from GroupActionEvent. Inner TokenEvent
        // (custom serde) flattens its named fields at the same level.
        assert_eq!(
            json,
            json!({
                "$kind": "tokenEvent",
                "$type": "mint",
                "amount": 5_000,
                "recipient": "Bswb3UyeD1pUTaGiE6WvqwFpJZsQSEY1xhJePCDTHdvp",
                "publicNote": "genesis mint",
            })
        );
        let recovered = GroupActionEvent::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_token_event_mint() {
        use crate::serialization::ValueConvertible;
        let original = GroupActionEvent::TokenEvent(
            crate::tokens::token_event::json_convertible_tests::mint_fixture(),
        );
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$kind": "tokenEvent",
                "$type": "mint",
                "amount": 5_000u64,
                "recipient": platform_value::Identifier::new([0xa1; 32]),
                "publicNote": "genesis mint",
            })
        );
        let recovered = GroupActionEvent::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    // The two tests below pin the deepest custom-serde composition in the
    // crate: GroupActionEvent (`tag = "$kind"`, derive — buffers inner content
    // through serde's ContentDeserializer) → TokenEvent (custom impl) →
    // externally-tagged TokenConfigurationChangeItem /
    // TokenDistributionTypeWithResolvedRecipient → the custom
    // internally-tagged AuthorizedActionTakers /
    // TokenDistributionResolvedRecipient. The Identity-carrying variants
    // specifically exercise the Identifier dual-shape path (HR base58 string
    // vs buffered bytes) under the ContentDeserializer HR-quirk. A one-sided
    // edit to any custom Serialize/Deserialize pair in the chain fails here.

    fn change_item_fixture() -> GroupActionEvent {
        use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
        use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        GroupActionEvent::TokenEvent(TokenEvent::ConfigUpdate(
            TokenConfigurationChangeItem::ConventionsControlGroup(
                AuthorizedActionTakers::Identity(platform_value::Identifier::from([0x42u8; 32])),
            ),
            Some("rotate control".to_string()),
        ))
    }

    fn claim_fixture() -> GroupActionEvent {
        use crate::data_contract::associated_token::token_distribution_key::TokenDistributionTypeWithResolvedRecipient;
        use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
        GroupActionEvent::TokenEvent(TokenEvent::Claim(
            TokenDistributionTypeWithResolvedRecipient::Perpetual(
                TokenDistributionResolvedRecipient::Evonode(platform_value::Identifier::from(
                    [0x42u8; 32],
                )),
            ),
            750,
            Some("payout".to_string()),
        ))
    }

    #[test]
    fn json_round_trip_config_update_with_identity_action_taker() {
        use crate::serialization::JsonConvertible;
        let original = change_item_fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$kind": "tokenEvent",
                "$type": "configUpdate",
                "configurationChange": {
                    "$type": "conventionsControlGroup",
                    "value": {
                        "$type": "identity",
                        "identity": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                    },
                },
                "publicNote": "rotate control",
            })
        );
        let recovered = GroupActionEvent::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_config_update_with_identity_action_taker() {
        use crate::serialization::ValueConvertible;
        let original = change_item_fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$kind": "tokenEvent",
                "$type": "configUpdate",
                "configurationChange": {
                    "$type": "conventionsControlGroup",
                    "value": {
                        "$type": "identity",
                        "identity": platform_value::Identifier::from([0x42u8; 32]),
                    },
                },
                "publicNote": "rotate control",
            })
        );
        let recovered = GroupActionEvent::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_claim_with_resolved_recipient() {
        use crate::serialization::JsonConvertible;
        let original = claim_fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$kind": "tokenEvent",
                "$type": "claim",
                "distributionType": {
                    "$type": "perpetual",
                    "value": {
                        "$type": "evonode",
                        "identity": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                    },
                },
                "amount": 750,
                "publicNote": "payout",
            })
        );
        let recovered = GroupActionEvent::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_claim_with_resolved_recipient() {
        use crate::serialization::ValueConvertible;
        let original = claim_fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$kind": "tokenEvent",
                "$type": "claim",
                "distributionType": {
                    "$type": "perpetual",
                    "value": {
                        "$type": "evonode",
                        "identity": platform_value::Identifier::from([0x42u8; 32]),
                    },
                },
                "amount": 750u64,
                "publicNote": "payout",
            })
        );
        let recovered = GroupActionEvent::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

use std::fmt;

impl fmt::Display for GroupActionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupActionEvent::TokenEvent(event) => write!(f, "Token event: {}", event),
        }
    }
}

impl GroupActionEvent {
    /// Returns a reference to the public note if the variant includes one.
    pub fn public_note(&self) -> Option<&str> {
        match self {
            GroupActionEvent::TokenEvent(token_event) => token_event.public_note(),
        }
    }

    /// Returns a name of the event
    pub fn event_name(&self) -> String {
        match self {
            GroupActionEvent::TokenEvent(token_event) => {
                format!("Token: {}", token_event.associated_document_type_name())
            }
        }
    }
}
