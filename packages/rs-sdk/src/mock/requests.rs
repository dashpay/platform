use super::MockDashPlatformSdk;
use dpp::{
    block::extended_epoch_info::ExtendedEpochInfo,
    document::serialization_traits::DocumentCborMethodsV0,
    document::Document,
    platform_serialization::{
        platform_encode_to_vec, platform_versioned_decode_from_slice, PlatformVersionEncode,
        PlatformVersionedDecode,
    },
    prelude::{DataContract, Identity},
    serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    },
};
use drive_proof_verifier::types::{
    Contenders, ContestedResources, PrefundedSpecializedBalance, VotePollsGroupedByTimestamp,
    Voters,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

static BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

/// Trait implemented by objects that can be used in mock expectation responses.
///
/// ## Panics
///
/// Can panic on errors.
pub trait MockResponse {
    /// Serialize the object to save into expectations
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8>;

    /// Deserialize the object from expectations
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    fn mock_deserialize(mock_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized;
}

impl<T: MockResponse> MockResponse for Option<T> {
    fn mock_deserialize(mock_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        if buf.is_empty() {
            return None;
        }

        Some(T::mock_deserialize(mock_sdk, buf))
    }
    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8> {
        match self {
            Some(item) => item.mock_serialize(mock_sdk),
            None => vec![],
        }
    }
}

impl<T: MockResponse> MockResponse for Vec<T> {
    fn mock_deserialize(mock_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let items: Vec<Vec<u8>> = bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode vec of data")
            .0;
        items
            .into_iter()
            .map(|item| T::mock_deserialize(mock_sdk, &item))
            .collect()
    }

    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let data: Vec<Vec<u8>> = self
            .iter()
            .map(|item| item.mock_serialize(mock_sdk))
            .collect();

        bincode::encode_to_vec(data, BINCODE_CONFIG).expect("encode vec of data")
    }
}

impl<K: Ord + Serialize + for<'de> Deserialize<'de>, V: Serialize + for<'de> Deserialize<'de>>
    MockResponse for BTreeMap<K, V>
{
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        serde_json::from_slice(buf).expect("decode vec of data")
    }

    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        serde_json::to_vec(self).expect("encode vec of data")
    }
}

impl MockResponse for Identity
where
    Self: PlatformVersionedDecode + PlatformVersionEncode,
{
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // self.clone().serialize_to_bytes().expect("serialize data")
        platform_encode_to_vec(self, BINCODE_CONFIG, sdk.version()).expect("serialize data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized + PlatformVersionedDecode,
    {
        // Self::deserialize_from_bytes(buf).expect("deserialize data")
        platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
            .expect("deserialize data")
    }
}

// FIXME: Seems that DataContract doesn't implement PlatformVersionedDecode + PlatformVersionEncode,
// so we just use some methods implemented directly on these objects.
impl MockResponse for DataContract {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.serialize_to_bytes_with_platform_version(sdk.version())
            .expect("encode data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        DataContract::versioned_deserialize(buf, true, sdk.version()).expect("decode data")
    }
}

// FIXME: Seems that Document doesn't implement PlatformVersionedDecode + PlatformVersionEncode,
// so we use cbor.
impl MockResponse for Document {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.to_cbor().expect("encode data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        Self::from_cbor(buf, None, None, sdk.version()).expect("decode data")
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityBalance {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        (*self).to_be_bytes().to_vec()
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        Self::from_be_bytes(buf.try_into().expect("balance should be 8 bytes"))
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityNonceFetcher {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        (self.0).to_be_bytes().to_vec()
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        drive_proof_verifier::types::IdentityNonceFetcher(u64::from_be_bytes(
            buf.try_into()
                .expect("identity contract nonce should be should be 8 bytes"),
        ))
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityContractNonceFetcher {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        (self.0).to_be_bytes().to_vec()
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        drive_proof_verifier::types::IdentityContractNonceFetcher(u64::from_be_bytes(
            buf.try_into()
                .expect("identity contract nonce should be should be 8 bytes"),
        ))
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityBalanceAndRevision {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).expect("encode IdentityBalanceAndRevision")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (item, _len) = bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode IdentityBalanceAndRevision");
        item
    }
}

impl MockResponse for ExtendedEpochInfo {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        platform_encode_to_vec(self, BINCODE_CONFIG, sdk.version())
            .expect("encode ExtendedEpochInfo")
    }
    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
            .expect("decode ExtendedEpochInfo")
    }
}
// TODO: Verify if this serialization is deterministic
impl MockResponse for Contenders {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::serde::encode_to_vec(self, BINCODE_CONFIG).expect("encode Contenders")
    }
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (v, _) =
            bincode::serde::decode_from_slice(buf, BINCODE_CONFIG).expect("decode Contenders");
        v
    }
}

impl MockResponse for Voters {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).expect("encode Voters")
    }
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode Voters")
            .0
    }
}

impl MockResponse for VotePollsGroupedByTimestamp {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).expect("encode VotePollsGroupedByTimestamp")
    }
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode VotePollsGroupedByTimestamp")
            .0
    }
}

impl MockResponse for PrefundedSpecializedBalance {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).expect("encode PrefundedSpecializedBalance")
    }
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode PrefundedSpecializedBalance")
            .0
    }
}

impl MockResponse for ContestedResources {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::serde::encode_to_vec(self, BINCODE_CONFIG).expect("encode ContestedResources")
    }
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::serde::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode ContestedResources")
            .0
    }
}
