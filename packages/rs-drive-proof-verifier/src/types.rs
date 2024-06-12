//! Object types that can be retrieved from proofs.
//!
//! Some DAPI requests return response types that are not defined in the Dash Platform Protocol,
//! like [GetIdentityBalanceRequest](dapi_grpc::platform::v0::GetIdentityBalanceRequest) which returns [`u64`].
//! In this case, the [FromProof](crate::FromProof) trait is implemented for dedicated object type
//! defined in this module.

use dpp::dashcore::hashes::Hash;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, TimestampMillis};
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
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
#[cfg(feature = "mocks")]
use {
    bincode::{Decode, Encode},
    dpp::{version as platform_version, ProtocolError},
    platform_serialization::{PlatformVersionEncode, PlatformVersionedDecode},
    platform_serialization_derive::{PlatformDeserialize, PlatformSerialize},
};

use drive::grovedb::Element;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
#[derive(Default)]
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
/// Identity balance and revision of the identity.
pub type IdentityBalanceAndRevision = (u64, Revision);

/// Contested resource values.
/// At this point, only Documents are supported
#[derive(derive_more::From, Clone, Debug)]
pub enum ContestedResource {
    /// Contested document
    Document {
        /// Contested document
        document: Document,
        /// Name of the document type
        document_type_name: String,
        /// Data contract for which the document is contested
        data_contract: Arc<DataContract>,
    },
}
#[cfg(feature = "mocks")]
#[derive(Encode, Decode, Clone, Debug)]
enum ContestedResourceSerialized {
    Document {
        serialized_document: Vec<u8>,
        document_type_name: String,
        serialized_data_contract: Vec<u8>,
    },
}

#[cfg(feature = "mocks")]
impl PlatformVersionEncode for ContestedResource {
    fn platform_encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &platform_version::PlatformVersion,
    ) -> Result<(), bincode::error::EncodeError> {
        let serialized = match self {
            ContestedResource::Document {
                document,
                document_type_name,
                data_contract,
            } => {
                let document_type = data_contract
                    .document_type_borrowed_for_name(document_type_name)
                    .map_err(|e| {
                        bincode::error::EncodeError::OtherString(format!(
                            "Failed to get document type: {}",
                            e
                        ))
                    })?;
                let serialized_data_contract = data_contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .map_err(|e| {
                        bincode::error::EncodeError::OtherString(format!(
                            "Failed to serialize data contract: {}",
                            e
                        ))
                    })?;
                let serialized_document = document
                    .serialize(document_type.as_ref(), platform_version)
                    .map_err(|e| {
                        bincode::error::EncodeError::OtherString(format!(
                            "Failed to serialize document: {}",
                            e
                        ))
                    })?;

                ContestedResourceSerialized::Document {
                    serialized_document,
                    document_type_name: document_type_name.clone(),
                    serialized_data_contract,
                }
            }
        };

        serialized.encode(encoder)
    }
}

#[cfg(feature = "mocks")]
impl PlatformVersionedDecode for ContestedResource {
    fn platform_versioned_decode<D: bincode::de::Decoder>(
        decoder: &mut D,
        platform_version: &platform_version::PlatformVersion,
    ) -> Result<Self, bincode::error::DecodeError> {
        let serialized = ContestedResourceSerialized::decode(decoder)?;

        match serialized {
            ContestedResourceSerialized::Document {
                serialized_document,
                document_type_name,
                serialized_data_contract,
            } => {
                let data_contract = DataContract::versioned_deserialize(
                    &serialized_data_contract,
                    true,
                    platform_version,
                )
                .map_err(|e| {
                    bincode::error::DecodeError::OtherString(format!(
                        "Failed to deserialize data contract: {}",
                        e
                    ))
                })?;

                let document_type = data_contract
                    .document_type_for_name(&document_type_name)
                    .map_err(|e| {
                        bincode::error::DecodeError::OtherString(format!(
                            "Unknown document type {}: {}",
                            document_type_name, e
                        ))
                    })?;
                let document =
                    Document::from_bytes(&serialized_document, document_type, platform_version)
                        .map_err(|e| {
                            bincode::error::DecodeError::OtherString(format!(
                                "Failed to deserialize document: {}",
                                e
                            ))
                        })?;
                Ok(ContestedResource::Document {
                    document,
                    document_type_name,
                    data_contract: Arc::new(data_contract),
                })
            }
        }
    }
}
/// Contested resources
#[derive(derive_more::From, Clone, Debug, Default)]
pub struct ContestedResources(pub BTreeMap<Identifier, ContestedResource>);

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
        let map = BTreeMap::<Identifier, ContestedResource>::platform_versioned_decode(
            decoder,
            platform_version,
        )?;
        Ok(Self(map))
    }
}

/// Create [ContestedResources] from an iterator of tuples.
///
/// This trait is a requirement of the [FetchMany](crate::FetchMany) trait.
impl FromIterator<(Identifier, Option<ContestedResource>)> for ContestedResources {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<ContestedResource>)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))))
    }
}
impl FromIterator<(Identifier, ContestedResource)> for ContestedResources {
    fn from_iter<T: IntoIterator<Item = (Identifier, ContestedResource)>>(iter: T) -> Self {
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
pub struct PrefundedSpecializedBalance(Credits);
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
pub struct VotePollsGroupedByTimestamp(pub BTreeMap<TimestampMillis, Vec<VotePoll>>);

/// Insert items into the map, appending them to the existing values for the same key.
impl FromIterator<(u64, Vec<VotePoll>)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, Vec<VotePoll>)>>(iter: T) -> Self {
        let mut map = BTreeMap::new();

        for (timestamp, vote_poll) in iter {
            let entry = map.entry(timestamp).or_insert_with(Vec::new);
            entry.extend(vote_poll);
        }

        Self(map)
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
    type IntoIter = std::collections::btree_map::IntoIter<u64, Vec<VotePoll>>;

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
