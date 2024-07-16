mod v0;

use crate::data_contract::v0::DataContractV0;
use crate::prelude::DataContract;
use crate::util::cbor_value::CborCanonicalMap;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Identifier;
pub use v0::*;

impl DataContractCborConversionMethodsV0 for DataContract {
    fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(DataContractV0::from_cbor_with_id(
                cbor_bytes,
                contract_id,
                full_validation,
                platform_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_cbor_with_id".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn from_cbor(
        cbor_bytes: impl AsRef<[u8]>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(
                DataContractV0::from_cbor(cbor_bytes, full_validation, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_cbor".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn to_cbor(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_cbor(platform_version),
        }
    }

    fn to_cbor_canonical_map(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<CborCanonicalMap, ProtocolError> {
        todo!()
    }
}
