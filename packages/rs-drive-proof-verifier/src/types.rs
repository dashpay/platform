//! Object types that can be retrieved from proofs.
//!
//! Some DAPI requests return response types that are not defined in the Dash Platform Protocol,
//! like [GetIdentityBalanceRequest](dapi_grpc::platform::v0::GetIdentityBalanceRequest) which returns [`u64`].
//! In this case, the [FromProof](crate::FromProof) trait is implemented for dedicated object type
//! defined in this module.

use crate::proof::Length;
use dpp::prelude::IdentityNonce;
pub use dpp::version::ProtocolVersionVoteCount;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
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
use std::collections::BTreeMap;

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
pub struct Contenders {
    pub contenders: RetrievedObjects<Identifier, Contender>,
    pub abstain_vote_tally: Option<u32>,
    pub lock_vote_tally: Option<u32>,
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

/// Contested document resource vote polls grouped by timestamp.
#[derive(Clone, Debug)]
pub struct ContestedDocumentResourceVotePollsGroupedByTimestamp(
    pub BTreeMap<u64, Vec<ContestedDocumentResourceVotePoll>>,
);

/// Create iterator for `ContestedDocumentResourceVotePollsGroupedByTimestamp`.
///
/// This implementation flattens the `BTreeMap` into a vector of tuples `(u64, ContestedDocumentResourceVotePoll)`
/// and then creates an iterator from the vector.
///
/// It means it copies all values from the `BTreeMap`.
impl IntoIterator for ContestedDocumentResourceVotePollsGroupedByTimestamp {
    type Item = (u64, ContestedDocumentResourceVotePoll);
    type IntoIter = std::vec::IntoIter<(u64, ContestedDocumentResourceVotePoll)>;

    fn into_iter(self) -> Self::IntoIter {
        let v: Vec<(u64, ContestedDocumentResourceVotePoll)> =
            self.0.iter().fold(Vec::new(), |mut acc, (k, v)| {
                v.iter().for_each(|poll| {
                    acc.push((*k, poll.clone()));
                });
                acc
            });

        v.into_iter()
    }
}

impl Length for ContestedDocumentResourceVotePollsGroupedByTimestamp {
    fn count_some(&self) -> usize {
        self.0.values().map(|v| v.len()).sum()
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
