//! Object types that can be retrieved from proofs.
//!
//! Some DAPI requests return response types that are not defined in the Dash Platform Protocol,
//! like [GetIdentityBalanceRequest](dapi_grpc::platform::v0::GetIdentityBalanceRequest) which returns [`u64`].
//! In this case, the [FromProof](crate::FromProof) trait is implemented for dedicated object type
//! defined in this module.

use std::collections::BTreeMap;

use dpp::{
    block::{epoch::EpochIndex, extended_epoch_info::ExtendedEpochInfo},
    document::Document,
    identity::KeyID,
    prelude::{DataContract, Identifier, IdentityPublicKey, Revision},
    util::deserializer::ProtocolVersion,
};

/// Collection of objects returned by the [FetchMany](crate::platform::FetchMany) operation.
///
/// Collection of objects of type `O`, indexed by key `K`.
///
/// Object is an option, representing:
///
/// * `Some(O)` - object is found
/// * `None` - object is not found, and the platform provided proof of non-existence
pub type Collection<K, O> = BTreeMap<K, Option<O>>;

/// History of a data contract.
///
/// Contains a map of data contract revisions to data contracts.
pub type DataContractHistory = BTreeMap<u64, DataContract>;
/// Multiple data contracts.
///
/// Mapping between data contract IDs and data contracts.
/// If data contract is not found, it is represented as `None`.
pub type DataContracts = Collection<[u8; 32], DataContract>;

/// Identity balance.
pub type IdentityBalance = u64;
/// Identity balance and revision of the identity.
pub type IdentityBalanceAndRevision = (u64, Revision);

/// Public keys belonging to some identity.
///
/// Map of [key IDs](KeyID) to the [public key](IdentityPublicKey).
pub type IdentityPublicKeys = Collection<KeyID, IdentityPublicKey>;

/// Collection of documents.
pub type Documents = Collection<Identifier, Document>;

/// Collection of epoch information
pub type ExtendedEpochInfos = Collection<EpochIndex, ExtendedEpochInfo>;

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
pub type ProtocolVersionUpgrades = Collection<ProtocolVersion, ProtocolVersionVoteCount>;
