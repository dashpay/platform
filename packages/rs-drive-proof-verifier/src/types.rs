//! Object types that can be retrieved from proofs.
//!
//! Some DAPI requests return response types that are not defined in the Dash Platform Protocol,
//! like [GetIdentityBalanceRequest](dapi_grpc::platform::v0::GetIdentityBalanceRequest) which returns [`u64`].
//! In this case, the [FromProof](crate::FromProof) trait is implemented for dedicated object type
//! defined in this module.

use std::collections::BTreeMap;

use dpp::{
    block::{epoch::EpochIndex, extended_epoch_info::ExtendedEpochInfo},
    dashcore::ProTxHash,
    document::Document,
    identity::KeyID,
    prelude::{DataContract, Identifier, IdentityPublicKey, Revision},
    util::deserializer::ProtocolVersion,
};

/// A data structure that holds a set of objects of a generic type `O`, indexed by a key of type `K`.
///
/// This type is typically returned by functions that operate on multiple objects, such as fetching multiple objects
/// from a server using [`FetchMany`](rs_sdk::platform::FetchMany) or parsing a proof that contains multiple objects
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
pub type DataContracts = RetrievedObjects<[u8; 32], DataContract>;

/// Identity balance.
pub type IdentityBalance = u64;
/// Identity balance and revision of the identity.
pub type IdentityBalanceAndRevision = (u64, Revision);

/// Public keys belonging to some identity.
///
/// Map of [key IDs](KeyID) to the [public key](IdentityPublicKey).
pub type IdentityPublicKeys = RetrievedObjects<KeyID, IdentityPublicKey>;

/// Collection of documents.
pub type Documents = RetrievedObjects<Identifier, Document>;

/// Collection of epoch information
pub type ExtendedEpochInfos = RetrievedObjects<EpochIndex, ExtendedEpochInfo>;

/// Number of votes for a protocol version upgrade.
///
/// Number of votes for a protocol version upgrade, returned by [ProtocolVersionVoteCount::fetch_many()].
/// See [ProtocolVersionUpgrades].
pub type ProtocolVersionVoteCount = u64;

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

/// Information about protocol version voted by each node.
///
/// Information about protocol version voted by each node, returned by [ProtocolVersion::fetch_many()].
/// Indexed by [ProTxHash] of nodes.
pub type ProtocolVersionVotes = RetrievedObjects<ProTxHash, ProtocolVersion>;
