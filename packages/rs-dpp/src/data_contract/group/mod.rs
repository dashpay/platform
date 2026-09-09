use crate::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use crate::data_contract::group::methods::v0::GroupMethodsV0;
use crate::data_contract::group::v0::GroupV0;
use crate::data_contract::GroupContractPosition;
use crate::errors::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::validation::SimpleConsensusValidationResult;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod accessors;
pub(crate) mod methods;
pub mod v0;

pub type RequiredSigners = u8;

pub type GroupMemberPower = u32;
pub type GroupSumPower = u32;
pub type GroupRequiredPower = u32;
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[platform_serialize(unversioned)]
#[serde(tag = "$formatVersion")]
pub enum Group {
    #[serde(rename = "0")]
    V0(GroupV0),
}

impl GroupV0Getters for Group {
    fn member_power(&self, member_id: Identifier) -> Result<u32, ProtocolError> {
        match self {
            Group::V0(group_v0) => group_v0.member_power(member_id),
        }
    }
    fn members(&self) -> &BTreeMap<Identifier, u32> {
        match self {
            Group::V0(group_v0) => group_v0.members(),
        }
    }

    fn members_mut(&mut self) -> &mut BTreeMap<Identifier, u32> {
        match self {
            Group::V0(group_v0) => group_v0.members_mut(),
        }
    }

    fn required_power(&self) -> GroupRequiredPower {
        match self {
            Group::V0(group_v0) => group_v0.required_power(),
        }
    }
}

impl GroupV0Setters for Group {
    fn set_members(&mut self, members: BTreeMap<Identifier, u32>) {
        match self {
            Group::V0(group_v0) => group_v0.set_members(members),
        }
    }

    fn set_member_power(&mut self, member_id: Identifier, power: u32) {
        match self {
            Group::V0(group_v0) => group_v0.set_member_power(member_id, power),
        }
    }

    fn remove_member(&mut self, member_id: &Identifier) -> bool {
        match self {
            Group::V0(group_v0) => group_v0.remove_member(member_id),
        }
    }

    fn set_required_power(&mut self, required_power: GroupRequiredPower) {
        match self {
            Group::V0(group_v0) => group_v0.set_required_power(required_power),
        }
    }
}

impl GroupMethodsV0 for Group {
    fn validate(
        &self,
        group_contract_position: Option<GroupContractPosition>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            Group::V0(group_v0) => group_v0.validate(group_contract_position, platform_version),
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
    use crate::data_contract::group::v0::GroupV0;
    use platform_value::Identifier;
    use serde_json::json;
    use std::collections::BTreeMap;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    fn fixture() -> Group {
        let mut members = BTreeMap::new();
        members.insert(Identifier::new([0xa0; 32]), 1u32);
        members.insert(Identifier::new([0xb1; 32]), 2u32);
        Group::V0(GroupV0 {
            members,
            required_power: 2,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `members` keys are `Identifier` — JSON renders them as the
        // base58-encoded string (e.g. "Bp2HuBWdciXFKV2CnoC1Z4V44QfCmArCQHdzKpArYJc7"
        // for `[0xa0; 32]`). Member-power values are `u32`; JSON has only one
        // number type, so the U32 distinction is erased on the wire — the
        // value-path assertion below uses `1u32` / `2u32` to lock it in.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "members": {
                    "Bp2HuBWdciXFKV2CnoC1Z4V44QfCmArCQHdzKpArYJc7": 1,
                    "CxeKLJRofna6h2GqCsjdVwc2D7EdDFX8uDy9KGw3Ey68": 2,
                },
                "requiredPower": 2,
            })
        );
        let recovered = Group::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `members` is `BTreeMap<Identifier, u32>`. platform_value renders
        // the BTreeMap as a `Value::Map([(Identifier, U32), ...])` (NOT the
        // textual-keyed `{ ... }` form, because the keys are Identifiers, not
        // strings) and preserves the U32 variant on the values. We construct
        // the expected map directly because `platform_value!{...}` only emits
        // string-keyed Maps.
        use platform_value::Value;
        let expected = Value::Map(vec![
            (
                Value::Text("$formatVersion".to_string()),
                Value::Text("0".to_string()),
            ),
            (
                Value::Text("members".to_string()),
                Value::Map(vec![
                    (Value::Identifier([0xa0; 32]), Value::U32(1)),
                    (Value::Identifier([0xb1; 32]), Value::U32(2)),
                ]),
            ),
            (Value::Text("requiredPower".to_string()), Value::U32(2)),
        ]);
        assert_eq!(value, expected);
        let recovered = Group::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
