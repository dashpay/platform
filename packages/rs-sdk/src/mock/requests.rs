use std::collections::BTreeMap;

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

use rs_dapi_client::mock::Key;
use serde::{Deserialize, Serialize};

use super::MockDashPlatformSdk;

/// Trait implemented by objects that can be used as requests in mock expectations.
pub trait MockRequest {
    /// Format the object as a key that will be used to match the request with the expectation.
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    fn mock_key(&self) -> Key;
}

impl<T: serde::Serialize> MockRequest for T {
    fn mock_key(&self) -> Key {
        Key::new(self)
    }
}

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
        let items: Vec<Vec<u8>> = bincode::decode_from_slice(buf, bincode::config::standard())
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

        bincode::encode_to_vec(data, bincode::config::standard()).expect("encode vec of data")
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
        platform_encode_to_vec(self, bincode::config::standard(), sdk.version())
            .expect("serialize data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized + PlatformVersionedDecode,
    {
        // Self::deserialize_from_bytes(buf).expect("deserialize data")
        platform_versioned_decode_from_slice(buf, bincode::config::standard(), sdk.version())
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
        (*self).to_le_bytes().to_vec()
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        Self::from_le_bytes(buf.try_into().expect("balance should be 8 bytes"))
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityContractNonce {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        (self.0).to_le_bytes().to_vec()
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
        where
            Self: Sized,
    {
        drive_proof_verifier::types::IdentityContractNonce(u64::from_le_bytes(buf.try_into().expect("identity contract nonce should be should be 8 bytes")))
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityBalanceAndRevision {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, bincode::config::standard())
            .expect("encode IdentityBalanceAndRevision")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (item, _len) = bincode::decode_from_slice(buf, bincode::config::standard())
            .expect("decode IdentityBalanceAndRevision");
        item
    }
}

impl MockResponse for ExtendedEpochInfo {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        platform_encode_to_vec(self, bincode::config::standard(), sdk.version())
            .expect("encode ExtendedEpochInfo")
    }
    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        platform_versioned_decode_from_slice(buf, bincode::config::standard(), sdk.version())
            .expect("decode ExtendedEpochInfo")
    }
}
