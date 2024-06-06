//! Object types that can be retrieved from proofs.
//!
//! Some DAPI requests return response types that are not defined in the Dash Platform Protocol,
//! like [GetIdentityBalanceRequest](dapi_grpc::platform::v0::GetIdentityBalanceRequest) which returns [`u64`].
//! In this case, the [FromProof](crate::FromProof) trait is implemented for dedicated object type
//! defined in this module.

use crate::proof::Length;
use bincode::Decode;
use dpp::bincode::Encode;
use dpp::prelude::{IdentityNonce, TimestampMillis};
pub use dpp::version::ProtocolVersionVoteCount;
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
use drive::query::vote_poll_vote_state_query::Contender;
use std::collections::{BTreeMap, BTreeSet};

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
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct Contenders {
    /// Contenders indexed by their identity IDs.
    pub contenders: RetrievedObjects<Identifier, Contender>,
    /// Tally of abstain votes.
    pub abstain_vote_tally: Option<u32>,
    ///
    pub lock_vote_tally: Option<u32>,
}

impl FromIterator<(Identifier, Option<Contender>)> for Contenders {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<Contender>)>>(iter: T) -> Self {
        Self {
            contenders: BTreeMap::from_iter(iter),
            abstain_vote_tally: None,
            lock_vote_tally: None,
        }
    }
}

/// Identifier of a single voter
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, derive_more::From, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, serde::Serialize, serde::Deserialize)
)]
pub struct Voter(pub Identifier);

/// Multiple voters.
#[derive(Debug, Clone, derive_more::From, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, serde::Serialize, serde::Deserialize)
)]
pub struct Voters(pub BTreeSet<Voter>);

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
#[derive(derive_more::From)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum ContestedResource {
    /// Contested document
    Document(Document),
}

/// Contested resources
pub type ContestedResources = RetrievedObjects<Identifier, ContestedResource>;

/// A contested vote for querying
pub type ContestedVote = (ContestedDocumentResourceVotePoll, ResourceVoteChoice);

/// Votes casted by some identity.
pub type ResourceVotesByIdentity = RetrievedObjects<Identifier, ResourceVote>;

/// Contested document resource vote polls grouped by timestamp.
#[derive(Clone, Debug, Default, derive_more::From, Encode, Decode)]
pub struct VotePollsGroupedByTimestamp(pub RetrievedObjects<TimestampMillis, Vec<VotePoll>>);

/// Insert items into the map, appending them to the existing values for the same key.
impl FromIterator<(u64, Option<Vec<VotePoll>>)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, Option<Vec<VotePoll>>)>>(iter: T) -> Self {
        let mut map = BTreeMap::new();

        for (timestamp, vote_poll) in iter {
            let entry = map.entry(timestamp).or_insert(Some(Vec::new()));
            if let Some(vote_poll) = vote_poll {
                if let Some(inner) = entry {
                    inner.extend(vote_poll);
                } else {
                    panic!("unexpected None value in VotePollsGroupedByTimestamp::from_iter(), this should never happen")
                }
            }
        }

        Self(map)
    }
}

/// Insert items into the map, appending them to the existing values for the same key.
impl FromIterator<(u64, Vec<VotePoll>)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, Vec<VotePoll>)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().map(|(k, v)| (k, Some(v))))
    }
}

/// Insert items into the map, grouping them by timestamp.
impl FromIterator<(u64, Option<VotePoll>)> for VotePollsGroupedByTimestamp {
    fn from_iter<T: IntoIterator<Item = (u64, Option<VotePoll>)>>(iter: T) -> Self {
        Self::from_iter(iter.into_iter().map(|(k, opt)| (k, opt.map(|v| vec![v]))))
    }
}

impl IntoIterator for VotePollsGroupedByTimestamp {
    type Item = (u64, Option<Vec<VotePoll>>);
    type IntoIter = std::collections::btree_map::IntoIter<u64, Option<Vec<VotePoll>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Length for VotePollsGroupedByTimestamp {
    fn count_some(&self) -> usize {
        self.0
            .values()
            .filter_map(|opt| opt.as_ref().map(|v| v.len()))
            .sum()
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
/// Information about the protocol version upgrade states and number of received vote_choices, indexed by protocol version.
/// Returned by [ProtocolVersionVoteCount::fetch_many()].
///
/// ## Data Structure
///
/// * [`ProtocolVersion`] - key determining protocol version
/// * [`ProtocolVersionVoteCount`] - value, number of vote_choices for the protocol version upgrade
pub type ProtocolVersionUpgrades = RetrievedObjects<ProtocolVersion, ProtocolVersionVoteCount>;

/// Vote of a masternode for a protocol version.
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct MasternodeProtocolVote {
    /// ProTxHash of the masternode
    pub pro_tx_hash: ProTxHash,
    /// Version for which this masternode voted
    pub voted_version: ProtocolVersion,
}

/// Information about protocol version voted by each node.
///
/// Information about protocol version voted by each node, returned by [ProtocolVersion::fetch_many()].
/// Indexed by [ProTxHash] of nodes.
pub type MasternodeProtocolVotes = RetrievedObjects<ProTxHash, MasternodeProtocolVote>;
