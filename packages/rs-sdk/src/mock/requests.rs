use super::MockDashPlatformSdk;
use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dpp::bincode::config::standard;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::group::Group;
use dpp::group::group_action::GroupAction;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::tokens::status::TokenStatus;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::{
    bincode,
    block::{extended_epoch_info::ExtendedEpochInfo, finalized_epoch_info::FinalizedEpochInfo},
    dashcore::{hashes::Hash as CoreHash, ProTxHash},
    document::{serialization_traits::DocumentCborMethodsV0, Document},
    identifier::Identifier,
    identity::IdentityPublicKey,
    platform_serialization::{platform_encode_to_vec, platform_versioned_decode_from_slice},
    prelude::{DataContract, Identity},
    serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    },
    voting::votes::{resource_vote::ResourceVote, Vote},
};
use drive::grovedb::Element;
use drive_proof_verifier::types::evonode_status::EvoNodeStatus;
use drive_proof_verifier::types::groups::GroupActions;
use drive_proof_verifier::types::identity_token_balance::{
    IdentitiesTokenBalances, IdentityTokenBalances,
};
use drive_proof_verifier::types::token_info::{IdentitiesTokenInfos, IdentityTokenInfos};
use drive_proof_verifier::types::token_status::TokenStatuses;
use drive_proof_verifier::types::{
    Contenders, ContestedResources, CurrentQuorumsInfo, ElementFetchRequestItem,
    IdentityBalanceAndRevision, IndexMap, MasternodeProtocolVote, PrefundedSpecializedBalance,
    ProposerBlockCounts, RetrievedValues, TotalCreditsInPlatform, VotePollsGroupedByTimestamp,
    Voters,
};
use std::{collections::BTreeMap, hash::Hash};

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

impl<K: Ord + MockResponse, V: MockResponse> MockResponse for BTreeMap<K, V> {
    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (data, _): (BTreeMap<Vec<u8>, Vec<u8>>, _) =
            bincode::decode_from_slice(buf, BINCODE_CONFIG).expect("decode BTreeMap");

        data.into_iter()
            .map(|(k, v)| (K::mock_deserialize(sdk, &k), V::mock_deserialize(sdk, &v)))
            .collect()
    }

    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let data: BTreeMap<Vec<u8>, Vec<u8>> = self
            .iter()
            .map(|(k, v)| (k.mock_serialize(sdk), v.mock_serialize(sdk)))
            .collect();

        bincode::encode_to_vec(data, BINCODE_CONFIG).expect("encode BTreeMap")
    }
}

impl<K: Hash + Eq + MockResponse, V: MockResponse> MockResponse for IndexMap<K, V> {
    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (data, _): (IndexMap<Vec<u8>, Vec<u8>>, _) =
            bincode::serde::decode_from_slice(buf, BINCODE_CONFIG).expect("decode IndexMap");

        data.into_iter()
            .map(|(k, v)| (K::mock_deserialize(sdk, &k), V::mock_deserialize(sdk, &v)))
            .collect()
    }

    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let data: IndexMap<Vec<u8>, Vec<u8>> = self
            .iter()
            .map(|(k, v)| (k.mock_serialize(sdk), v.mock_serialize(sdk)))
            .collect();

        bincode::serde::encode_to_vec(data, BINCODE_CONFIG).expect("encode IndexMap")
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

impl MockResponse for Element {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // Create a bincode configuration
        let config = standard();

        // Serialize using the specified configuration
        bincode::encode_to_vec(self, config).expect("Failed to serialize Element")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        // Create a bincode configuration
        let config = standard();

        // Deserialize using the specified configuration
        bincode::decode_from_slice(buf, config)
            .expect("Failed to deserialize Element")
            .0
    }
}

impl MockResponse for drive_proof_verifier::types::IdentityNonceFetcher {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
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
        self.0.to_be_bytes().to_vec()
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
impl MockResponse for ProTxHash {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let data = self.as_raw_hash().as_byte_array();
        platform_encode_to_vec(data, BINCODE_CONFIG, sdk.version()).expect("encode ProTxHash")
    }
    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let data = platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
            .expect("decode ProTxHash");
        ProTxHash::from_raw_hash(CoreHash::from_byte_array(data))
    }
}

impl MockResponse for ProposerBlockCounts {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.0.mock_serialize(sdk)
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let data = RetrievedValues::<Identifier, u64>::mock_deserialize(sdk, buf);
        ProposerBlockCounts(data)
    }
}

impl MockResponse for IdentityTokenBalances {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.0.mock_serialize(sdk)
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let map = RetrievedValues::mock_deserialize(sdk, buf);
        Self(map)
    }
}

impl MockResponse for IdentitiesTokenBalances {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.0.mock_serialize(sdk)
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let map = RetrievedValues::mock_deserialize(sdk, buf);
        Self(map)
    }
}

impl MockResponse for IdentityTokenInfos {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // Clone and collect into vector
        let vec: Vec<(Identifier, Option<IdentityTokenInfo>)> =
            self.0.iter().map(|(k, v)| (*k, v.clone())).collect();

        // Serialize vector
        platform_encode_to_vec(vec, BINCODE_CONFIG, sdk.version())
            .expect(concat!("encode ", stringify!($name)))
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        // deserialize vector
        let vec: Vec<(Identifier, Option<IdentityTokenInfo>)> =
            platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
                .expect(concat!("decode ", stringify!($name)));

        Self(RetrievedValues::from_iter(vec))
    }
}

impl MockResponse for IdentitiesTokenInfos {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // Clone and collect into vector
        let vec: Vec<(Identifier, Option<IdentityTokenInfo>)> =
            self.0.iter().map(|(k, v)| (*k, v.clone())).collect();

        // Serialize vector
        platform_encode_to_vec(vec, BINCODE_CONFIG, sdk.version())
            .expect(concat!("encode ", stringify!($name)))
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        // deserialize vector
        let vec: Vec<(Identifier, Option<IdentityTokenInfo>)> =
            platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
                .expect(concat!("decode ", stringify!($name)));

        Self(RetrievedValues::from_iter(vec))
    }
}

impl MockResponse for TokenStatuses {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // Clone and collect into vector
        let vec: Vec<(Identifier, Option<TokenStatus>)> =
            self.iter().map(|(k, v)| (*k, v.clone())).collect();

        // Serialize vector
        platform_encode_to_vec(vec, BINCODE_CONFIG, sdk.version())
            .expect(concat!("encode ", stringify!($name)))
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        // deserialize vector
        let vec: Vec<(Identifier, Option<TokenStatus>)> =
            platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
                .expect(concat!("decode ", stringify!($name)));

        RetrievedValues::from_iter(vec)
    }
}

impl MockResponse for TokenContractInfo {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        platform_encode_to_vec(self, BINCODE_CONFIG, sdk.version())
            .expect("encode TokenContractInfo")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
            .expect("decode TokenContractInfo")
    }
}

impl MockResponse for TotalSingleTokenBalance {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).expect("encode vec of data")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode vec of data")
            .0
    }
}

impl MockResponse for GroupActions {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // Clone and collect into vector
        let vec: Vec<(Identifier, Option<GroupAction>)> =
            self.iter().map(|(k, v)| (*k, v.clone())).collect();

        // Serialize vector
        platform_encode_to_vec(vec, BINCODE_CONFIG, sdk.version())
            .expect(concat!("encode ", stringify!($name)))
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        // deserialize vector
        let vec: Vec<(Identifier, Option<GroupAction>)> =
            platform_versioned_decode_from_slice(buf, BINCODE_CONFIG, sdk.version())
                .expect(concat!("decode ", stringify!($name)));

        RetrievedValues::from_iter(vec)
    }
}

impl_mock_response!(Identity);
impl_mock_response!(IdentityPublicKey);
impl_mock_response!(Identifier);
impl_mock_response!(MasternodeProtocolVote);
impl_mock_response!(ResourceVote);
impl_mock_response!(u8);
impl_mock_response!(u16);
impl_mock_response!(u32);
impl_mock_response!(u64);
impl_mock_response!(Vote);
impl_mock_response!(ExtendedEpochInfo);
impl_mock_response!(FinalizedEpochInfo);
impl_mock_response!(ContestedResources);
impl_mock_response!(IdentityBalanceAndRevision);
impl_mock_response!(Contenders);
impl_mock_response!(Voters);
impl_mock_response!(VotePollsGroupedByTimestamp);
impl_mock_response!(PrefundedSpecializedBalance);
impl_mock_response!(TotalCreditsInPlatform);
impl_mock_response!(ElementFetchRequestItem);
impl_mock_response!(EvoNodeStatus);
impl_mock_response!(CurrentQuorumsInfo);
impl_mock_response!(Group);
impl_mock_response!(TokenPricingSchedule);
impl_mock_response!(RewardDistributionMoment);
impl_mock_response!(TokenContractInfo);
