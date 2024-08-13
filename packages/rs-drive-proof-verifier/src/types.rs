//! Object types that can be retrieved from proofs.
//!
//! Some DAPI requests return response types that are not defined in the Dash Platform Protocol,
//! like [GetIdentityBalanceRequest](dapi_grpc::platform::v0::GetIdentityBalanceRequest) which returns [`u64`].
//! In this case, the [FromProof](crate::FromProof) trait is implemented for dedicated object type
//! defined in this module.

use dpp::data_contract::document_type::DocumentType;
use dpp::fee::Credits;
use dpp::platform_value::Value;
use dpp::prelude::{IdentityNonce, TimestampMillis};
use dpp::version::PlatformVersion;
pub use dpp::version::ProtocolVersionVoteCount;
use dpp::voting::contender_structs::{Contender, ContenderWithSerializedDocument};
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::{
    block::{epoch::EpochIndex, extended_epoch_info::ExtendedEpochInfo},
    dashcore::ProTxHash,
    document::Document,
    identity::KeyID,
    prelude::{DataContract, Identifier, IdentityPublicKey, Revision},
    util::deserializer::ProtocolVersion,
};
use drive::grovedb::Element;
use std::collections::{BTreeMap, BTreeSet};

use drive::grovedb::query_result_type::Path;
#[cfg(feature = "mocks")]
use {
    bincode::{Decode, Encode},
    dpp::{dashcore::hashes::Hash, version as platform_version, ProtocolError},
    platform_serialization::{PlatformVersionEncode, PlatformVersionedDecode},
    platform_serialization_derive::{PlatformDeserialize, PlatformSerialize},
};

/// A data structure that holds a set of objects of a generic type `O`, indexed by a key of type `K`.
///
/// This type is typically returned by functions that operate on multiple objects, such as fetching multiple objects
/// from a server using [`FetchMany`](dash_sdk::platform::FetchMany) or parsing a proof that contains multiple objects
/// using [`FromProof`](crate::FromProof).
///
/// Each key in the `RetrievedObjects` corresponds to an object of generic type `O`.
/// If an object is found for a given key, the value is `Some(object)`.
/// If no object is found for a given key, the value is `None`.
///
/// # Generic Type Parameters
///
/// * `K`: The type of the keys in the map.
/// * `O`: The type of the objects in the map.
pub type RetrievedObjects<K, O> = BTreeMap<K, Option<O>>;

/// History of a data contract.
///
/// Contains a map of data contract revisions to data contracts.
pub type DataContractHistory = BTreeMap<u64, DataContract>;
/// Multiple data contracts.
///
/// Mapping between data contract IDs and data contracts.
/// If data contract is not found, it is represented as `None`.
pub type DataContracts = RetrievedObjects<Identifier, DataContract>;

/// Multiple contenders for a vote resolution.
///
/// Mapping between the contenders identity IDs and their info.
/// If a contender is not found, it is represented as `None`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize,),
    platform_serialize(unversioned)
)]
pub struct Contenders {
    /// Contenders indexed by their identity IDs.
    pub contenders: BTreeMap<Identifier, ContenderWithSerializedDocument>,
    /// Tally of abstain votes.
    pub abstain_vote_tally: Option<u32>,
    /// Tally of lock votes.
    pub lock_vote_tally: Option<u32>,
}

impl Contenders {
    /// Return a map of deserialized [Contender] objects indexed by their identity IDs.
    pub fn to_contenders(
        &self,
        document_type: &DocumentType,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Contender>, crate::Error> {
        self.contenders
            .iter()
            .map(|(id, v)| {
                let contender = v.try_to_contender(document_type.as_ref(), platform_version)?;
                Ok((*id, contender))
            })
            .collect::<Result<BTreeMap<Identifier, Contender>, dpp::ProtocolError>>()
            .map_err(Into::into)
    }
}

/// Create Contenders from an iterator of tuples.
///
/// This trait is a requirement of the [FetchMany](crate::FetchMany) trait.
impl FromIterator<(Identifier, Option<ContenderWithSerializedDocument>)> for Contenders {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<ContenderWithSerializedDocument>)>>(
        iter: T,
    ) -> Self {
        Self {
            contenders: BTreeMap::from_iter(
                iter.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))),
            ),
            abstain_vote_tally: None,
            lock_vote_tally: None,
        }
    }
}

/// Identifier of a single voter
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, derive_more::From, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize,),
    platform_serialize(unversioned)
)]
pub struct Voter(pub Identifier);

/// Multiple voters.
#[derive(Debug, Clone, derive_more::From, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize,),
    platform_serialize(unversioned)
)]
pub struct Voters(pub BTreeSet<Voter>);

/// Create [Voters] from an iterator of tuples.
///
/// This trait is a requirement of the [FetchMany](crate::FetchMany) trait.
impl<A> FromIterator<(A, Option<Voter>)> for Voters {
    fn from_iter<T: IntoIterator<Item = (A, Option<Voter>)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().filter_map(|(_, v)| v))
    }
}

impl FromIterator<Voter> for Voters {
    fn from_iter<T: IntoIterator<Item = Voter>>(iter: T) -> Self {
        iter.into_iter().collect::<BTreeSet<_>>().into()
    }
}

/// Multiple grovedb elements.
///
/// Mapping between the key id and associated elements.
/// If element is not found, it is represented as `None`.
pub type Elements = RetrievedObjects<Vec<u8>, Element>;

/// Identity balance.
pub type IdentityBalance = u64;

/// Keys in a Path
#[derive(Debug, Clone)]
pub struct KeysInPath {
    /// The path of the keys
    pub path: Path,
    /// The keys
    pub keys: Vec<Vec<u8>>,
}

/// The total credits on Platform.
#[derive(Debug, derive_more::From, Clone, Copy)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
pub struct TotalCreditsOnPlatform(pub Credits);

/// A query with no parameters
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
pub struct NoParamQuery;

/// The item of an element fetch request
#[derive(Debug, derive_more::From, Clone)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
pub struct ElementFetchRequestItem(pub Element);

/// Identity balance and revision of the identity.
pub type IdentityBalanceAndRevision = (u64, Revision);

/// Contested resource values.
#[derive(Debug, derive_more::From, Clone, PartialEq)]
pub enum ContestedResource {
    /// Generic [Value]
    Value(Value),
}

impl ContestedResource {
    /// Get the value.
    pub fn encode_to_vec(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, bincode::error::EncodeError> {
        platform_serialization::platform_encode_to_vec(
            self,
            bincode::config::standard(),
            platform_version,
        )
    }
}

impl TryInto<Value> for ContestedResource {
    type Error = crate::Error;

    fn try_into(self) -> Result<Value, Self::Error> {
        match self {
            ContestedResource::Value(value) => Ok(value),
        }
    }
}

#[cfg(feature = "mocks")]
impl PlatformVersionEncode for ContestedResource {
    fn platform_encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
        _platform_version: &platform_version::PlatformVersion,
    ) -> Result<(), bincode::error::EncodeError> {
        match self {
            ContestedResource::Value(value) => value.encode(encoder),
        }
    }
}

#[cfg(feature = "mocks")]
impl PlatformVersionedDecode for ContestedResource {
    fn platform_versioned_decode<D: bincode::de::Decoder>(
        decoder: &mut D,
        _platform_version: &platform_version::PlatformVersion,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(ContestedResource::Value(Value::decode(decoder)?))
    }
}

/// Contested resources
#[derive(derive_more::From, Clone, Debug, Default)]
pub struct ContestedResources(pub Vec<ContestedResource>);

#[cfg(feature = "mocks")]
impl PlatformVersionEncode for ContestedResources {
    fn platform_encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &platform_version::PlatformVersion,
    ) -> Result<(), bincode::error::EncodeError> {
        self.0.platform_encode(encoder, platform_version)
    }
}

#[cfg(feature = "mocks")]
impl PlatformVersionedDecode for ContestedResources {
    fn platform_versioned_decode<D: bincode::de::Decoder>(
        decoder: &mut D,
        platform_version: &platform_version::PlatformVersion,
    ) -> Result<Self, bincode::error::DecodeError> {
        let inner = <Vec<ContestedResource>>::platform_versioned_decode(decoder, platform_version)?;
        Ok(Self(inner))
    }
}

/// Create [ContestedResources] from an iterator of tuples.
///
/// This trait is a requirement of the [FetchMany](crate::FetchMany) trait.
impl<A> FromIterator<(A, Option<ContestedResource>)> for ContestedResources {
    fn from_iter<T: IntoIterator<Item = (A, Option<ContestedResource>)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().filter_map(|(_k, v)| v))
    }
}

impl FromIterator<ContestedResource> for ContestedResources {
    fn from_iter<T: IntoIterator<Item = ContestedResource>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A contested vote for querying
#[derive(Debug, derive_more::From, Clone)]
#[cfg_attr(
    feature = "mocks",
    derive(PlatformSerialize, PlatformDeserialize, Encode, Decode),
    platform_serialize(unversioned)
)]
pub struct ContestedVote(ContestedDocumentResourceVotePoll, ResourceVoteChoice);

/// Votes casted by some identity.
pub type ResourceVotesByIdentity = RetrievedObjects<Identifier, ResourceVote>;

/// Prefunded specialized balance.
#[derive(Debug, derive_more::From, Copy, Clone)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
pub struct PrefundedSpecializedBalance(pub Credits);
impl PrefundedSpecializedBalance {
    /// Get the balance.
    pub fn to_credits(&self) -> Credits {
        Credits::from(self)
    }
}

impl From<&PrefundedSpecializedBalance> for Credits {
    fn from(value: &PrefundedSpecializedBalance) -> Self {
        value.0
    }
}

/// Contested document resource vote polls grouped by timestamp.
#[derive(Clone, Debug, Default, derive_more::From)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
pub struct VotePollsGroupedByTimestamp(pub Vec<(TimestampMillis, Vec<VotePoll>)>);
impl VotePollsGroupedByTimestamp {
    /// Sort the vote polls by timestamp.
    pub fn sorted(mut self, ascending: bool) -> Self {
        self.0.sort_by(|a, b| {
            if ascending {
                a.0.cmp(&b.0)
            } else {
                b.0.cmp(&a.0)
            }
        });

        self
    }
}

/// Insert items into the map, appending them to the existing values for the same key.
impl FromIterator<(u64, Vec<VotePoll>)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, Vec<VotePoll>)>>(iter: T) -> Self {
        // collect all vote polls for the same timestamp into a single vector
        let data = iter
            .into_iter()
            .fold(BTreeMap::new(), |mut acc, (timestamp, vote_poll)| {
                let entry: &mut Vec<VotePoll> = acc.entry(timestamp).or_default();
                entry.extend(vote_poll);
                acc
            });

        Self(data.into_iter().collect())
    }
}

/// Insert items into the map, grouping them by timestamp.
impl FromIterator<(u64, VotePoll)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, VotePoll)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().map(|(k, v)| (k, vec![v])))
    }
}

/// Create [VotePollsGroupedByTimestamp] from an iterator of tuples.
///
/// If multiple vote polls are found for the same timestamp, they are appended to the same vector.
///
/// This trait is a requirement of the [FetchMany](crate::FetchMany) trait.
impl FromIterator<(u64, Option<VotePoll>)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, Option<VotePoll>)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))))
    }
}

impl IntoIterator for VotePollsGroupedByTimestamp {
    type Item = (u64, Vec<VotePoll>);
    type IntoIter = std::vec::IntoIter<(u64, Vec<VotePoll>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// An identity nonce
#[derive(Debug)]
pub struct IdentityNonceFetcher(pub IdentityNonce);

/// An identity contract nonce
#[derive(Debug)]
pub struct IdentityContractNonceFetcher(pub IdentityNonce);

/// Public keys belonging to some identity.
///
/// Map of [key IDs](KeyID) to the [public key](IdentityPublicKey).
pub type IdentityPublicKeys = RetrievedObjects<KeyID, IdentityPublicKey>;

/// Collection of documents.
pub type Documents = RetrievedObjects<Identifier, Document>;

/// Collection of epoch information
pub type ExtendedEpochInfos = RetrievedObjects<EpochIndex, ExtendedEpochInfo>;

/// Results of protocol version upgrade voting.
///
/// Information about the protocol version upgrade states and number of received votes, indexed by protocol version.
/// Returned by [ProtocolVersionVoteCount::fetch_many()].
///
/// ## Data Structure
///
/// * [`ProtocolVersion`] - key determining protocol version
/// * [`ProtocolVersionVoteCount`] - value, number of votes for the protocol version upgrade
pub type ProtocolVersionUpgrades = RetrievedObjects<ProtocolVersion, ProtocolVersionVoteCount>;

/// Vote of a masternode for a protocol version.
#[derive(Debug)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct MasternodeProtocolVote {
    /// ProTxHash of the masternode
    pub pro_tx_hash: ProTxHash,
    /// Version for which this masternode voted
    pub voted_version: ProtocolVersion,
}

#[cfg(feature = "mocks")]
impl PlatformVersionEncode for MasternodeProtocolVote {
    fn platform_encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &platform_version::PlatformVersion,
    ) -> Result<(), bincode::error::EncodeError> {
        let protx_bytes: [u8; 32] = self.pro_tx_hash.to_raw_hash().to_byte_array();
        protx_bytes.platform_encode(encoder, platform_version)?;

        self.voted_version
            .platform_encode(encoder, platform_version)
    }
}

#[cfg(feature = "mocks")]
impl PlatformVersionedDecode for MasternodeProtocolVote {
    fn platform_versioned_decode<D: bincode::de::Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, bincode::error::DecodeError> {
        let pro_tx_hash_bytes = <[u8; 32]>::platform_versioned_decode(decoder, platform_version)?;
        let pro_tx_hash = ProTxHash::from_byte_array(pro_tx_hash_bytes);
        let voted_version = ProtocolVersion::platform_versioned_decode(decoder, platform_version)?;
        Ok(Self {
            pro_tx_hash,
            voted_version,
        })
    }
}

/// Information about protocol version voted by each node.
///
/// Information about protocol version voted by each node, returned by [ProtocolVersion::fetch_many()].
/// Indexed by [ProTxHash] of nodes.
pub type MasternodeProtocolVotes = RetrievedObjects<ProTxHash, MasternodeProtocolVote>;
