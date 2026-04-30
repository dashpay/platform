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
    serde(untagged)
)]
pub enum TokenContractInfo {
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn fixture() -> TokenContractInfo {
        TokenContractInfo::V0(crate::tokens::contract_info::v0::TokenContractInfoV0 {
            contract_id: platform_value::Identifier::default(),
            token_contract_position: 0,
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenContractInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenContractInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
