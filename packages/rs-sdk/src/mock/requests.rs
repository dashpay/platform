use super::MockDashPlatformSdk;
use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dpp::bincode::config::standard;
use dpp::address_funds::PlatformAddress;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::group::Group;
use dpp::group::group_action::GroupAction;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::tokens::status::TokenStatus;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::{
    bincode,
    block::{extended_epoch_info::ExtendedEpochInfo, finalized_epoch_info::FinalizedEpochInfo},
    dashcore::{hashes::Hash as CoreHash, ProTxHash},
    document::{serialization_traits::DocumentCborMethodsV0, Document},
    identifier::Identifier,
    identity::{identities_contract_keys::IdentitiesContractKeys, IdentityPublicKey},
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
use drive::grovedb::GroveTrunkQueryResult;
use drive_proof_verifier::types::{
    AddressInfo, Contenders, ContestedResources, CurrentQuorumsInfo, ElementFetchRequestItem,
    IdentityBalanceAndRevision, IndexMap, MasternodeProtocolVote, MostRecentShieldedAnchor,
    PlatformAddressTrunkState, PrefundedSpecializedBalance, ProposerBlockCounts,
    RecentAddressBalanceChanges, RecentCompactedAddressBalanceChanges, RetrievedValues,
    ShieldedAnchors, ShieldedEncryptedNote, ShieldedEncryptedNotes, ShieldedNotesCount,
    ShieldedNullifierStatus, ShieldedNullifierStatuses, ShieldedPoolState,
    TokenPreProgrammedDistributions, TotalCreditsInPlatform, VotePollsGroupedByTimestamp, Voters,
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

// FIXME: Seems that DataContract doesn't implement PlatformVersionedDecode + PlatformVersionEncode,
// so we just use some methods implemented directly on these objects.
impl MockResponse for (DataContract, Vec<u8>) {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.1.clone()
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        (
            DataContract::versioned_deserialize(buf, true, sdk.version()).expect("decode data"),
            buf.to_vec(),
        )
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

impl MockResponse for IdentitiesContractKeys {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).expect("encode IdentitiesContractKeys")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::decode_from_slice(buf, BINCODE_CONFIG)
            .expect("decode IdentitiesContractKeys")
            .0
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
impl_mock_response!(TokenPreProgrammedDistributions);
impl_mock_response!(PlatformAddress);
impl_mock_response!(AddressInfo);
impl_mock_response!(RecentAddressBalanceChanges);
impl_mock_response!(RecentCompactedAddressBalanceChanges);
impl_mock_response!(ShieldedPoolState);
impl_mock_response!(ShieldedNotesCount);
impl_mock_response!(ShieldedAnchors);
impl_mock_response!(MostRecentShieldedAnchor);
impl_mock_response!(ShieldedEncryptedNotes);
impl_mock_response!(ShieldedEncryptedNote);
impl_mock_response!(ShieldedNullifierStatuses);
impl_mock_response!(ShieldedNullifierStatus);

/// MockResponse for GroveTrunkQueryResult - panics when called because the Tree type
/// doesn't support serialization. Address sync operations should not be mocked.
impl MockResponse for GroveTrunkQueryResult {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        unimplemented!("GroveTrunkQueryResult does not support mock serialization - the Tree type is not serializable")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, _buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        unimplemented!("GroveTrunkQueryResult does not support mock deserialization - the Tree type is not serializable")
    }
}

/// MockResponse for PlatformAddressTrunkState - panics when called because the underlying
/// Tree type doesn't support serialization. Address sync operations should not be mocked.
impl MockResponse for PlatformAddressTrunkState {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        unimplemented!("PlatformAddressTrunkState does not support mock serialization - the Tree type is not serializable")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, _buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        unimplemented!("PlatformAddressTrunkState does not support mock deserialization - the Tree type is not serializable")
    }
}

impl MockResponse for drive_proof_verifier::DocumentCount {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        bincode::encode_to_vec(self.0, bincode_config).expect("encode DocumentCount")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let (count, _): (u64, _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentCount");
        drive_proof_verifier::DocumentCount(count)
    }
}

/// Wire shape for `DocumentSplitCounts` mock round-trip:
/// `(in_key, key, count)` triples preserving the In dimension
/// AND the verified-vs-absent count distinction. Shared by
/// `mock_serialize`/`mock_deserialize` below — single source of
/// truth so the encode/decode generics align by construction,
/// and clippy's `type_complexity` lint (CI runs with
/// `-D warnings`) doesn't fire on the inline form.
type DocumentSplitCountTriples = Vec<(Option<Vec<u8>>, Vec<u8>, Option<u64>)>;

impl MockResponse for drive_proof_verifier::DocumentSplitCounts {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        // Serialize as `(in_key, key, count)` triples so the In
        // dimension AND the verified-vs-absent count distinction
        // both survive the mock roundtrip. Required for compound
        // (`In + range + distinct`) test fixtures to keep their
        // `in_key` values, and for GroupByIn-absent-branch
        // fixtures to keep their `None` counts.
        let triples: DocumentSplitCountTriples = self
            .0
            .iter()
            .map(|e| (e.in_key.clone(), e.key.clone(), e.count))
            .collect();
        bincode::encode_to_vec(triples, bincode_config).expect("encode DocumentSplitCounts")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let (triples, _): (DocumentSplitCountTriples, _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentSplitCounts");
        let entries: Vec<drive_proof_verifier::SplitCountEntry> = triples
            .into_iter()
            .map(
                |(in_key, key, count)| drive_proof_verifier::SplitCountEntry { in_key, key, count },
            )
            .collect();
        drive_proof_verifier::DocumentSplitCounts::from_verified(entries)
    }
}

impl MockResponse for drive_proof_verifier::DocumentSum {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        bincode::encode_to_vec(self.0, bincode_config).expect("encode DocumentSum")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let (sum, _): (i64, _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentSum");
        drive_proof_verifier::DocumentSum(sum)
    }
}

/// Wire shape for `DocumentSplitSums` mock round-trip. Mirrors
/// [`DocumentSplitCountTriples`] — preserves the `in_key` axis
/// and the verified-vs-absent sum distinction (`Option<i64>`)
/// across the roundtrip.
type DocumentSplitSumTriples = Vec<(Option<Vec<u8>>, Vec<u8>, Option<i64>)>;

impl MockResponse for drive_proof_verifier::DocumentSplitSums {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        let triples: DocumentSplitSumTriples = self
            .0
            .iter()
            .map(|e| (e.in_key.clone(), e.key.clone(), e.sum))
            .collect();
        bincode::encode_to_vec(triples, bincode_config).expect("encode DocumentSplitSums")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let (triples, _): (DocumentSplitSumTriples, _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentSplitSums");
        let entries: Vec<drive_proof_verifier::SplitSumEntry> = triples
            .into_iter()
            .map(|(in_key, key, sum)| drive_proof_verifier::SplitSumEntry { in_key, key, sum })
            .collect();
        drive_proof_verifier::DocumentSplitSums(entries)
    }
}

impl MockResponse for drive_proof_verifier::DocumentAverage {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        bincode::encode_to_vec((self.count, self.sum), bincode_config)
            .expect("encode DocumentAverage")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let ((count, sum), _): ((u64, i64), _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentAverage");
        drive_proof_verifier::DocumentAverage { count, sum }
    }
}

/// Wire shape for `DocumentSplitAverages` mock round-trip. Same
/// `(in_key, key)` axes as the sum variant, but carries both
/// `Option<u64>` (count) and `Option<i64>` (sum) so the verified-vs-
/// absent state of each axis can roundtrip independently.
type DocumentSplitAverageTuples = Vec<(Option<Vec<u8>>, Vec<u8>, Option<u64>, Option<i64>)>;

impl MockResponse for drive_proof_verifier::DocumentSplitAverages {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        let tuples: DocumentSplitAverageTuples = self
            .0
            .iter()
            .map(|e| (e.in_key.clone(), e.key.clone(), e.count, e.sum))
            .collect();
        bincode::encode_to_vec(tuples, bincode_config).expect("encode DocumentSplitAverages")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let (tuples, _): (DocumentSplitAverageTuples, _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentSplitAverages");
        let entries: Vec<drive_proof_verifier::SplitAverageEntry> = tuples
            .into_iter()
            .map(
                |(in_key, key, count, sum)| drive_proof_verifier::SplitAverageEntry {
                    in_key,
                    key,
                    count,
                    sum,
                },
            )
            .collect();
        drive_proof_verifier::DocumentSplitAverages(entries)
    }
}

/// Wire shape for `DocumentRankedEntries` mock round-trip: the page's
/// `starting_rank`, then `(group key, axis tag, value)` triples in list
/// order — **order is the ranking**, so a map-shaped encoding (as used
/// nowhere here, but as would be the obvious alternative) would destroy
/// the answer.
///
/// `starting_rank` is part of the encoding rather than reconstructed as
/// `0` on decode, because it is exactly what an offset test needs to
/// assert: a mock that dropped it would make every expectation look
/// like an offset-0 query and quietly pass a round-trip that lost the
/// rank base.
///
/// The value is widened to `i128` across all three axes: `Count`
/// (`u64`) and `Sum` (`i64`) both fit losslessly, and `AvgFixedPoint`
/// is already an `i128`. One numeric column keeps the tuple flat while
/// the tag preserves which axis produced it, so a mock expectation
/// can't quietly turn a count into a sum.
type DocumentRankedPage = (u64, Vec<(Vec<u8>, u8, i128)>);

const RANKED_TAG_COUNT: u8 = 0;
const RANKED_TAG_SUM: u8 = 1;
const RANKED_TAG_AVG: u8 = 2;

impl MockResponse for drive_proof_verifier::DocumentRankedEntries {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let bincode_config = standard();
        let triples: Vec<(Vec<u8>, u8, i128)> = self
            .entries
            .iter()
            .map(|e| match e.value {
                drive_proof_verifier::RankedEntryValue::Count(count) => {
                    (e.key.clone(), RANKED_TAG_COUNT, count as i128)
                }
                drive_proof_verifier::RankedEntryValue::Sum(sum) => {
                    (e.key.clone(), RANKED_TAG_SUM, sum as i128)
                }
                drive_proof_verifier::RankedEntryValue::AvgFixedPoint(avg) => {
                    (e.key.clone(), RANKED_TAG_AVG, avg)
                }
            })
            .collect();
        let page: DocumentRankedPage = (self.starting_rank, triples);
        bincode::encode_to_vec(page, bincode_config).expect("encode DocumentRankedEntries")
    }

    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let bincode_config = standard();
        let ((starting_rank, triples), _): (DocumentRankedPage, _) =
            bincode::decode_from_slice(buf, bincode_config).expect("decode DocumentRankedEntries");
        let entries: Vec<drive_proof_verifier::RankedEntry> = triples
            .into_iter()
            .map(|(key, tag, value)| {
                let value = match tag {
                    RANKED_TAG_COUNT => drive_proof_verifier::RankedEntryValue::Count(
                        u64::try_from(value).expect("a Count entry round-trips through i128"),
                    ),
                    RANKED_TAG_SUM => drive_proof_verifier::RankedEntryValue::Sum(
                        i64::try_from(value).expect("a Sum entry round-trips through i128"),
                    ),
                    RANKED_TAG_AVG => drive_proof_verifier::RankedEntryValue::AvgFixedPoint(value),
                    other => panic!("unknown ranked axis tag {other} in mock expectation"),
                };
                drive_proof_verifier::RankedEntry { key, value }
            })
            .collect();
        drive_proof_verifier::DocumentRankedEntries {
            starting_rank,
            entries,
        }
    }
}

impl MockResponse for drive_proof_verifier::DocumentHavingEntries {
    /// Rides the ranked page encoding with a starting rank of `0`: a
    /// having page is the same ordered `(group key, axis tag, value)`
    /// list, just addressed by value bound instead of by rank, and it
    /// has no rank base to preserve.
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        drive_proof_verifier::DocumentRankedEntries {
            starting_rank: 0,
            entries: self.entries.clone(),
        }
        .mock_serialize(sdk)
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let page = drive_proof_verifier::DocumentRankedEntries::mock_deserialize(sdk, buf);
        drive_proof_verifier::DocumentHavingEntries {
            entries: page.entries,
        }
    }
}
