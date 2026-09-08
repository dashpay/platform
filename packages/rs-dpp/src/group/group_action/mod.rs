pub mod v0;

use crate::data_contract::TokenContractPosition;
use crate::group::action_event::GroupActionEvent;
use crate::group::group_action::v0::GroupActionV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    PartialEq,
    PartialOrd,
    Clone,
    Eq,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
// Stored group actions are decoded from GroveDB proof elements on the client
// before the quorum signature is checked, so the byte budget must be enforced
// by the decoder itself. Every payload (notes, config change, pricing
// schedule) is copied out of the state transition that proposed the action,
// and `StateTransition` is capped at the same 100,000 bytes, so no valid
// stored action can exceed this.
#[platform_serialize(limit = 100000, unversioned)] //versioned directly, no need to use platform_version
pub enum GroupAction {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(GroupActionV0),
}

pub trait GroupActionAccessors {
    fn contract_id(&self) -> Identifier;

    fn proposer_id(&self) -> Identifier;
    fn token_contract_position(&self) -> TokenContractPosition;
    fn event(&self) -> &GroupActionEvent;
}
impl GroupActionAccessors for GroupAction {
    fn contract_id(&self) -> Identifier {
        match self {
            GroupAction::V0(inner) => inner.contract_id(),
        }
    }

    fn proposer_id(&self) -> Identifier {
        match self {
            GroupAction::V0(inner) => inner.proposer_id(),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            GroupAction::V0(inner) => inner.token_contract_position(),
        }
    }

    fn event(&self) -> &GroupActionEvent {
        match self {
            GroupAction::V0(inner) => inner.event(),
        }
    }
}

// TODO(unification pass 2): add round-trip tests for GroupAction once we have an
// explicit fixture (GroupActionV0 has no Default — its `event: GroupActionEvent`
// field is itself a versioned enum without Default).

#[cfg(test)]
mod deserialize_limit_tests {
    use super::*;
    use crate::serialization::PlatformDeserializable;

    /// A proof element is untrusted input: a note length prefix must be
    /// rejected against the byte budget before it sizes an allocation.
    #[test]
    fn rejects_note_length_prefix_beyond_budget_without_allocating() {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let mut buf = Vec::new();
        // GroupAction::V0, then GroupActionV0 { contract_id, proposer_id, position, event }
        buf.extend_from_slice(&bincode::encode_to_vec(0u32, config).unwrap());
        buf.extend_from_slice(&[0u8; 32]);
        buf.extend_from_slice(&[0u8; 32]);
        buf.extend_from_slice(&bincode::encode_to_vec(0u16, config).unwrap());
        // GroupActionEvent::TokenEvent(TokenEvent::Freeze(id, Some(note)))
        buf.extend_from_slice(&bincode::encode_to_vec(0u32, config).unwrap());
        buf.extend_from_slice(&bincode::encode_to_vec(2u32, config).unwrap());
        buf.extend_from_slice(&[0u8; 32]);
        buf.push(1);
        // note length prefix claiming 8 GB, with no bytes following it
        buf.extend_from_slice(&bincode::encode_to_vec(8_000_000_000u64, config).unwrap());

        let err = GroupAction::deserialize_from_bytes(&buf)
            .expect_err("oversized length prefix must be rejected");
        assert!(
            matches!(err, ProtocolError::MaxEncodedBytesReachedError { .. }),
            "unexpected error: {err}"
        );
    }
}
