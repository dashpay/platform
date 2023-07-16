mod v0;

use crate::data_contract::v0::DataContractV0;
use crate::prelude::DataContract;
use crate::util::cbor_value::CborCanonicalMap;
use crate::version::PlatformVersion;
use crate::{Convertible, ProtocolError};
use platform_value::Identifier;
pub use v0::*;

impl DataContractCborConversionMethodsV0 for DataContract {
    fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(
                DataContractV0::from_cbor_with_id(cbor_bytes, contract_id, platform_version)?
                    .into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_cbor_with_id".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn from_cbor(
        cbor_bytes: impl AsRef<[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(DataContractV0::from_cbor(cbor_bytes, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_cbor".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_cbor(),
        }
    }

    fn to_cbor_canonical_map(&self) -> Result<CborCanonicalMap, ProtocolError> {
        todo!()
    }
}
