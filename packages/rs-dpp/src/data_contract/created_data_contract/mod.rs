mod fields;
pub mod v0;

use crate::data_contract::created_data_contract::v0::{
    CreatedDataContractInSerializationFormatV0, CreatedDataContractV0,
};
use crate::prelude::{DataContract, IdentityNonce};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use crate::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use platform_version::TryIntoPlatformVersioned;

/// The created data contract is a intermediate structure that can be consumed by a
/// contract create state transition.
///
///

#[derive(Clone, Debug, PartialEq, From)]
pub enum CreatedDataContract {
    V0(CreatedDataContractV0),
}

#[derive(Clone, Debug, Encode, Decode, From)]
pub enum CreatedDataContractInSerializationFormat {
    V0(CreatedDataContractInSerializationFormatV0),
}

impl PlatformSerializableWithPlatformVersion for CreatedDataContract {
    type Error = ProtocolError;

    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.clone()
            .serialize_consume_to_bytes_with_platform_version(platform_version)
    }

    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let (data_contract, identity_nonce) = self.data_contract_and_identity_nonce();
        let data_contract_serialization_format: DataContractInSerializationFormat =
            data_contract.try_into_platform_versioned(platform_version)?;
        let created_data_contract_in_serialization_format = match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(CreatedDataContractInSerializationFormat::V0(
                CreatedDataContractInSerializationFormatV0 {
                    data_contract: data_contract_serialization_format,
                    identity_nonce,
                },
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::serialize_to_bytes_with_platform_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }?;
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(created_data_contract_in_serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize CreatedDataContract: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for CreatedDataContract {
    fn versioned_deserialize(
        data: &[u8],
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let created_data_contract_in_serialization_format: CreatedDataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!(
                        "unable to deserialize DataContract: {}",
                        e
                    ))
                })?
                .0;
        let (data_contract_in_serialization_format, identity_nonce) =
            created_data_contract_in_serialization_format.data_contract_and_identity_nonce_owned();
        let data_contract = DataContract::try_from_platform_versioned(
            data_contract_in_serialization_format,
            full_validation,
            &mut vec![],
            platform_version,
        )?;
        match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(CreatedDataContract::V0(CreatedDataContractV0 {
                data_contract,
                identity_nonce,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::versioned_deserialize".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl From<CreatedDataContract> for DataContract {
    fn from(value: CreatedDataContract) -> Self {
        match value {
            CreatedDataContract::V0(created_data_contract) => created_data_contract.data_contract,
        }
    }
}

impl CreatedDataContract {
    pub fn data_contract_owned(self) -> DataContract {
        match self {
            CreatedDataContract::V0(v0) => v0.data_contract,
        }
    }

    pub fn data_contract_and_identity_nonce(self) -> (DataContract, IdentityNonce) {
        match self {
            CreatedDataContract::V0(v0) => (v0.data_contract, v0.identity_nonce),
        }
    }

    pub fn data_contract(&self) -> &DataContract {
        match self {
            CreatedDataContract::V0(v0) => &v0.data_contract,
        }
    }

    pub fn data_contract_mut(&mut self) -> &mut DataContract {
        match self {
            CreatedDataContract::V0(v0) => &mut v0.data_contract,
        }
    }

    pub fn identity_nonce(&self) -> IdentityNonce {
        match self {
            CreatedDataContract::V0(v0) => v0.identity_nonce,
        }
    }

    #[cfg(test)]
    pub fn set_identity_nonce(&mut self, identity_nonce: IdentityNonce) {
        match self {
            CreatedDataContract::V0(v0) => v0.identity_nonce = identity_nonce,
        }
    }

    pub fn from_contract_and_identity_nonce(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<CreatedDataContract, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(CreatedDataContractV0 {
                data_contract,
                identity_nonce,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::from_contract_and_entropy".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl CreatedDataContractInSerializationFormat {
    pub fn data_contract_and_identity_nonce_owned(
        self,
    ) -> (DataContractInSerializationFormat, IdentityNonce) {
        match self {
            CreatedDataContractInSerializationFormat::V0(v0) => {
                (v0.data_contract, v0.identity_nonce)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::PlatformVersion;

    fn sample_created() -> CreatedDataContract {
        get_data_contract_fixture(None, 42, 1)
    }

    // -----------------------------------------------------------------------
    // Constructor / getter coverage
    // -----------------------------------------------------------------------

    #[test]
    fn from_contract_and_identity_nonce_wraps_v0() {
        let platform_version = PlatformVersion::latest();
        let created = sample_created();
        let contract = created.data_contract().clone();
        let wrapped = CreatedDataContract::from_contract_and_identity_nonce(
            contract.clone(),
            99,
            platform_version,
        )
        .expect("should wrap successfully on latest version");
        assert_eq!(wrapped.identity_nonce(), 99);
        assert_eq!(wrapped.data_contract().id(), contract.id());
        // from_contract_and_identity_nonce always produces the V0 variant today.
        assert!(matches!(wrapped, CreatedDataContract::V0(_)));
    }

    #[test]
    fn identity_nonce_getter_returns_underlying_value() {
        let created = sample_created();
        assert_eq!(created.identity_nonce(), 42);
    }

    #[test]
    fn data_contract_getter_returns_same_id() {
        let created = sample_created();
        let via_getter = created.data_contract();
        let expected_id = via_getter.id();
        // Also verify the owned getter yields the same id.
        let owned = created.clone().data_contract_owned();
        assert_eq!(owned.id(), expected_id);
    }

    #[test]
    fn data_contract_mut_allows_mutation() {
        let mut created = sample_created();
        let original_id = created.data_contract().id();
        let new_id = platform_value::Identifier::from([7u8; 32]);
        // Mutate via the mutable accessor to prove it hands out a real mut ref.
        created.data_contract_mut().set_id(new_id);
        assert_ne!(created.data_contract().id(), original_id);
        assert_eq!(created.data_contract().id(), new_id);
    }

    #[test]
    fn data_contract_and_identity_nonce_extracts_both() {
        let created = sample_created();
        let (contract, nonce) = created.clone().data_contract_and_identity_nonce();
        assert_eq!(nonce, 42);
        assert_eq!(contract.id(), created.data_contract().id());
    }

    #[test]
    fn set_identity_nonce_updates_value() {
        let mut created = sample_created();
        created.set_identity_nonce(777);
        assert_eq!(created.identity_nonce(), 777);
    }

    // -----------------------------------------------------------------------
    // From<CreatedDataContract> for DataContract
    // -----------------------------------------------------------------------

    #[test]
    fn from_created_to_data_contract() {
        let created = sample_created();
        let expected_id = created.data_contract().id();
        let dc: DataContract = created.into();
        assert_eq!(dc.id(), expected_id);
    }

    // -----------------------------------------------------------------------
    // Bincode serialize / deserialize round-trip via
    // PlatformSerializableWithPlatformVersion +
    // PlatformDeserializableWithPotentialValidationFromVersionedStructure.
    // -----------------------------------------------------------------------

    #[test]
    fn serialize_roundtrip_via_platform_version() {
        let platform_version = PlatformVersion::latest();
        let created = sample_created();
        let bytes = created
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialize should succeed");
        assert!(!bytes.is_empty());

        let restored = CreatedDataContract::versioned_deserialize(&bytes, false, platform_version)
            .expect("deserialize should succeed");
        assert_eq!(restored.identity_nonce(), created.identity_nonce());
        assert_eq!(restored.data_contract().id(), created.data_contract().id());
    }

    #[test]
    fn serialize_consume_to_bytes_matches_clone_path() {
        let platform_version = PlatformVersion::latest();
        let created = sample_created();
        let via_ref = created
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("ref serialize should succeed");
        let via_consume = created
            .clone()
            .serialize_consume_to_bytes_with_platform_version(platform_version)
            .expect("consume serialize should succeed");
        // The ref path internally clones and delegates to the consume path;
        // both must produce byte-identical output.
        assert_eq!(via_ref, via_consume);
    }

    #[test]
    fn versioned_deserialize_rejects_garbage() {
        let platform_version = PlatformVersion::latest();
        let garbage = vec![0xFFu8; 16];
        let err = CreatedDataContract::versioned_deserialize(&garbage, false, platform_version)
            .expect_err("random bytes should not deserialize");
        match err {
            ProtocolError::PlatformDeserializationError(_) => {}
            other => panic!("expected PlatformDeserializationError, got {other:?}"),
        }
    }

    #[test]
    fn versioned_deserialize_rejects_empty_input() {
        let platform_version = PlatformVersion::latest();
        let err = CreatedDataContract::versioned_deserialize(&[], false, platform_version)
            .expect_err("empty input should not deserialize");
        assert!(matches!(
            err,
            ProtocolError::PlatformDeserializationError(_)
        ));
    }

    // -----------------------------------------------------------------------
    // CreatedDataContractInSerializationFormat helpers
    // -----------------------------------------------------------------------

    #[test]
    fn in_serialization_format_data_contract_and_identity_nonce_owned() {
        let platform_version = PlatformVersion::latest();
        let created = sample_created();
        // Re-serialize and re-decode to obtain a raw in-serialization-format value
        // without reaching into private fields.
        let bytes = created
            .clone()
            .serialize_consume_to_bytes_with_platform_version(platform_version)
            .expect("serialize");
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let (decoded, _consumed) = bincode::borrow_decode_from_slice::<
            CreatedDataContractInSerializationFormat,
            _,
        >(&bytes, config)
        .expect("raw bincode decode should succeed");
        let (_contract_fmt, nonce) = decoded.data_contract_and_identity_nonce_owned();
        assert_eq!(nonce, created.identity_nonce());
    }

    // -----------------------------------------------------------------------
    // Clone / PartialEq smoke — these derives are real (not generated per
    // variant) and are used elsewhere in the codebase via matches on equality.
    // -----------------------------------------------------------------------

    #[test]
    fn clone_and_equality() {
        let a = sample_created();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn different_nonce_breaks_equality() {
        let a = sample_created();
        let mut b = a.clone();
        b.set_identity_nonce(a.identity_nonce().wrapping_add(1));
        assert_ne!(a, b);
    }
}
