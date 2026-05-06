use crate::data_contract::TokenContractPosition;
use crate::tokens::contract_info::v0::TokenContractInfoV0;
use crate::ProtocolError;
use bincode::Encode;
use derive_more::From;
use platform_serialization::de::Decode;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;

mod methods;
pub mod v0;

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenContractInfo {
    #[cfg_attr(
        any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
        serde(rename = "0")
    )]
    V0(TokenContractInfoV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenContractInfo {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenContractInfo {}

impl TokenContractInfo {
    pub fn new(
        contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .token_contract_info_default_structure_version
        {
            0 => Ok(TokenContractInfo::V0(TokenContractInfoV0 {
                contract_id,
                token_contract_position,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenContractInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    fn fixture() -> TokenContractInfo {
        TokenContractInfo::V0(crate::tokens::contract_info::v0::TokenContractInfoV0 {
            contract_id: Identifier::new([0xab; 32]),
            token_contract_position: 7,
        })
    }

    // `TokenContractInfo` uses the standard `tag = "$formatVersion"` convention.

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `Identifier` renders as base58 in JSON HR. `tokenContractPosition` is
        // a `u16` (TokenContractPosition alias); JSON has only one number type
        // so the U16 distinction is erased — the Value-path assertion below
        // uses `7u16` to lock in the sized variant.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "contractId": "CZ8YUVdk7znjrUmnb5n7kgySk9yRAsQDYmyCxzfSky9t",
                "tokenContractPosition": 7,
            })
        );
        let recovered = TokenContractInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let contract_id = Identifier::new([0xab; 32]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "contractId": contract_id,
                "tokenContractPosition": 7u16,
            })
        );
        let recovered = TokenContractInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
