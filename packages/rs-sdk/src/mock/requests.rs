use super::MockDashPlatformSdk;
use dpp::{
    block::extended_epoch_info::ExtendedEpochInfo,
    document::{serialization_traits::DocumentCborMethodsV0, Document},
    platform_serialization::{platform_encode_to_vec, platform_versioned_decode_from_slice},
    prelude::{DataContract, Identity},
    serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    },
    voting::votes::Vote,
};
use drive_proof_verifier::types::{
    Contenders, ContestedResources, IdentityBalanceAndRevision, PrefundedSpecializedBalance,
    VotePollsGroupedByTimestamp, Voters,
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
        bincode::serde::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode vec of data")
            .0
    }

    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::serde::encode_to_vec(self, BINCODE_CONFIG).expect("encode vec of data")
    }
}

/// Serialize and deserialize the object for mocking using bincode.
///
/// Use this macro when the object implements platform serialization.
macro_rules! impl_mock_response {
    ($name:ident) => {
        impl MockResponse for $name {
            fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
                platform_encode_to_vec(self, BINCODE_CONFIG, sdk.version())
                    .expect(concat!("encode ", stringify!($name)))
            }
            fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
            where
                Self: Sized,
            {
                platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
                    .expect(concat!("decode ", stringify!($name)))
            }
        }
    };
}

/// Serialize and deserialize the object for mocking using bincode.
///
/// Use this macro when the object does not implement platform serialization, but it implements
/// Encode and Decode from bincode.
macro_rules! impl_mock_response_bincode {
    ($name:ident) => {
        impl MockResponse for $name {
            fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
                bincode::encode_to_vec(self, BINCODE_CONFIG)
                    .expect(concat!("encode ", stringify!($name)))
            }
            fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
            where
                Self: Sized,
            {
                bincode::decode_from_slice(buf, BINCODE_CONFIG)
                    .expect(concat!("decode ", stringify!($name)))
                    .0
            }
        }
    };
}

// TODO: Verify if this serialization is deterministic
/// Serialize and deserialize the object for mocking using bincode::serde.
///
/// Use this macro when the object does not implement bincode nor platform serialization, but it implements
/// Serialize and Deserialize from serde.
macro_rules! impl_mock_response_serde {
    ($name:ident) => {
        impl MockResponse for $name {
            fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
                bincode::serde::encode_to_vec(self, BINCODE_CONFIG)
                    .expect(concat!("encode ", stringify!($name)))
            }
            fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
            where
                Self: Sized,
            {
                bincode::serde::decode_from_slice(buf, BINCODE_CONFIG)
                    .expect(concat!("decode ", stringify!($name)))
                    .0
            }
        }
    };
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

impl_mock_response!(Identity);

impl_mock_response_serde!(ExtendedEpochInfo);
impl_mock_response_serde!(Contenders);
impl_mock_response_serde!(ContestedResources);
impl_mock_response_serde!(Vote);

impl_mock_response_bincode!(IdentityBalanceAndRevision);
impl_mock_response_bincode!(Voters);
impl_mock_response_bincode!(VotePollsGroupedByTimestamp);
impl_mock_response_bincode!(PrefundedSpecializedBalance);
