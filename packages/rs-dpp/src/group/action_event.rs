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
    // Internal tagging with `kind`. Plain (no `$` prefix) per the rule —
    // the wire-shape level has no other `$`-prefixed fields. Cannot use
    // `type` because the inner `TokenEvent` already uses `type` as its own
    // adjacent-tag discriminator (and is consensus-binary-locked, can't
    // rename). `kind` is distinct, semantically reads naturally ("the kind
    // is tokenEvent"), and lets us drop the `data` wrapper. Wire shape:
    //   {"kind": "tokenEvent", "type": "mint", "data": [...]}
    serde(tag = "kind", rename_all = "camelCase")
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

    // `GroupActionEvent` uses `tag = "kind"` (internal). Plain `kind`
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
        // Outer `kind: "tokenEvent"` from GroupActionEvent. Inner `type:
        // "mint"` and `data: [...]` from TokenEvent's adjacent tagging.
        assert_eq!(
            json,
            json!({
                "kind": "tokenEvent",
                "type": "mint",
                "data": [
                    5_000,
                    "Bswb3UyeD1pUTaGiE6WvqwFpJZsQSEY1xhJePCDTHdvp",
                    "genesis mint"
                ]
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
                "kind": "tokenEvent",
                "type": "mint",
                "data": [
                    5_000u64,
                    platform_value::Identifier::new([0xa1; 32]),
                    "genesis mint"
                ]
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
