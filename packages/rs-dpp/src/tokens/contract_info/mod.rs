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
pub enum TokenContractInfo {
    V0(TokenContractInfoV0),
}

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
