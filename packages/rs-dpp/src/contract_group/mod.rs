//! Contract groups.
//!
//! A contract group is an identity-owned set of contracts, contract document types and contract
//! tokens. A group is registered by a data contract create transition, which derives the group id
//! from the registering identity and the transition's identity nonce. Later contracts created by
//! an owner of the group can add themselves, one of their document types, or one of their tokens
//! to the group in their own create transition.
//!
//! Drive stores every group under the `ContractGroups` root tree together with a backwards index
//! from each member contract to the groups it belongs to.

use crate::data_contract::{DocumentName, TokenContractPosition};
use crate::prelude::IdentityNonce;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// The domain prefix hashed into every contract group id.
pub const CONTRACT_GROUP_ID_DOMAIN: &[u8] = b"contract_group";

/// Derives the id of the contract group registered by a data contract create transition.
///
/// The id is `hash_double("contract_group" || owner id || identity nonce (big endian))`. The
/// contract created by the same transition has id `hash_double(owner id || identity nonce)`, so
/// both ids are known to the client before broadcasting and can never collide.
pub fn generate_contract_group_id(
    owner_id: &Identifier,
    identity_nonce: IdentityNonce,
) -> Identifier {
    let mut bytes = CONTRACT_GROUP_ID_DOMAIN.to_vec();
    bytes.extend_from_slice(owner_id.as_slice());
    bytes.extend_from_slice(&identity_nonce.to_be_bytes());
    Identifier::from(hash_double(bytes))
}

/// Who owns a contract group, and so who may add members to it.
///
/// A member is added by the identity that creates the member contract, so a join is allowed when
/// that identity is an owner: the single owner, or one of the co-owners. Co-owners act alone; the
/// set is not a multisig.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum ContractGroupOwner {
    /// One identity owns the group.
    SingleOwner(Identifier),
    /// Several identities own the group; any one of them may add members. At least two
    /// distinct identities, at most `SystemLimits::max_contract_group_owners`.
    MultiOwner(BTreeSet<Identifier>),
}

impl ContractGroupOwner {
    /// Whether `identity_id` is an owner of the group.
    pub fn includes(&self, identity_id: &Identifier) -> bool {
        match self {
            ContractGroupOwner::SingleOwner(owner_id) => owner_id == identity_id,
            ContractGroupOwner::MultiOwner(owner_ids) => owner_ids.contains(identity_id),
        }
    }

    /// The number of owning identities.
    pub fn owner_count(&self) -> usize {
        match self {
            ContractGroupOwner::SingleOwner(_) => 1,
            ContractGroupOwner::MultiOwner(owner_ids) => owner_ids.len(),
        }
    }

    /// The owning identities.
    pub fn owner_ids(&self) -> BTreeSet<Identifier> {
        match self {
            ContractGroupOwner::SingleOwner(owner_id) => BTreeSet::from([*owner_id]),
            ContractGroupOwner::MultiOwner(owner_ids) => owner_ids.clone(),
        }
    }
}

impl fmt::Display for ContractGroupOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractGroupOwner::SingleOwner(owner_id) => write!(f, "single owner {}", owner_id),
            ContractGroupOwner::MultiOwner(owner_ids) => {
                write!(f, "{} owners", owner_ids.len())
            }
        }
    }
}

/// What part of the contract being created joins a contract group.
///
/// A member always belongs to the contract of the create transition that declares it; a contract
/// cannot enrol another contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum ContractGroupMember {
    /// The whole contract: every document type and every token it has or will have.
    Contract,
    /// One document type of the contract.
    DocumentType(DocumentName),
    /// One token of the contract, by its position in the contract.
    Token(TokenContractPosition),
}

impl fmt::Display for ContractGroupMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractGroupMember::Contract => write!(f, "contract"),
            ContractGroupMember::DocumentType(name) => write!(f, "document type {}", name),
            ContractGroupMember::Token(position) => write!(f, "token {}", position),
        }
    }
}

/// A declaration that a part of the created contract joins a contract group.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContractGroupMembership {
    /// The group joined. Either a group registered earlier by an owner, or the group registered
    /// by this same create transition.
    pub contract_group_id: Identifier,
    /// The part of the created contract that joins.
    pub member: ContractGroupMember,
}

/// The registration of a new contract group, carried by a data contract create transition.
///
/// The id is not on the wire: it is derived from the transition's owner and identity nonce with
/// [`generate_contract_group_id`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContractGroupRegistration {
    /// The owner or owners of the group. The registering identity must be one of them.
    pub owner: ContractGroupOwner,
    /// An optional human readable name, bounded by `SystemLimits::max_contract_group_name_length`.
    pub name: Option<String>,
    /// An optional description, bounded by
    /// `SystemLimits::max_contract_group_description_length`.
    pub description: Option<String>,
}

/// The stored information of a contract group, kept as one item under the group's tree.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    From,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[platform_serialize(unversioned)]
pub enum ContractGroupInfo {
    /// Version 0.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ContractGroupInfoV0),
}

/// Version 0 of the stored contract group information.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContractGroupInfoV0 {
    /// The owner or owners of the group.
    pub owner: ContractGroupOwner,
    /// An optional human readable name.
    pub name: Option<String>,
    /// An optional description.
    pub description: Option<String>,
}

impl ContractGroupInfo {
    /// The owner or owners of the group.
    pub fn owner(&self) -> &ContractGroupOwner {
        match self {
            ContractGroupInfo::V0(info) => &info.owner,
        }
    }

    /// The group's name, when it has one.
    pub fn name(&self) -> Option<&str> {
        match self {
            ContractGroupInfo::V0(info) => info.name.as_deref(),
        }
    }

    /// The group's description, when it has one.
    pub fn description(&self) -> Option<&str> {
        match self {
            ContractGroupInfo::V0(info) => info.description.as_deref(),
        }
    }
}

impl From<ContractGroupRegistration> for ContractGroupInfo {
    fn from(registration: ContractGroupRegistration) -> Self {
        let ContractGroupRegistration {
            owner,
            name,
            description,
        } = registration;
        ContractGroupInfo::V0(ContractGroupInfoV0 {
            owner,
            name,
            description,
        })
    }
}

impl From<&ContractGroupRegistration> for ContractGroupInfo {
    fn from(registration: &ContractGroupRegistration) -> Self {
        registration.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::DataContract;

    #[test]
    fn should_derive_a_contract_group_id_distinct_from_the_contract_id() {
        let owner_id = Identifier::from([7u8; 32]);
        let nonce = 42;

        let group_id = generate_contract_group_id(&owner_id, nonce);
        let contract_id = DataContract::generate_data_contract_id_v0(owner_id, nonce);

        assert_ne!(group_id, contract_id);
        assert_eq!(group_id, generate_contract_group_id(&owner_id, nonce));
        assert_ne!(group_id, generate_contract_group_id(&owner_id, nonce + 1));
        assert_ne!(
            group_id,
            generate_contract_group_id(&Identifier::from([8u8; 32]), nonce)
        );
    }

    #[test]
    fn should_resolve_owners_for_both_owner_kinds() {
        let alice = Identifier::from([1u8; 32]);
        let bob = Identifier::from([2u8; 32]);
        let carol = Identifier::from([3u8; 32]);

        let single = ContractGroupOwner::SingleOwner(alice);
        assert!(single.includes(&alice));
        assert!(!single.includes(&bob));
        assert_eq!(single.owner_count(), 1);

        let multi = ContractGroupOwner::MultiOwner(BTreeSet::from([alice, bob]));
        assert!(multi.includes(&alice));
        assert!(multi.includes(&bob));
        assert!(!multi.includes(&carol));
        assert_eq!(multi.owner_count(), 2);
    }

    #[test]
    fn should_round_trip_the_stored_info_through_bincode() {
        use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};

        let info: ContractGroupInfo = ContractGroupRegistration {
            owner: ContractGroupOwner::MultiOwner(BTreeSet::from([
                Identifier::from([1u8; 32]),
                Identifier::from([2u8; 32]),
            ])),
            name: Some("dashpay".to_string()),
            description: None,
        }
        .into();

        let bytes = info.serialize_to_bytes().expect("serialize");
        let decoded = ContractGroupInfo::deserialize_from_bytes_untrusted(&bytes)
            .expect("deserialize untrusted");

        assert_eq!(decoded, info);
        assert_eq!(decoded.name(), Some("dashpay"));
        assert_eq!(decoded.description(), None);
        assert_eq!(decoded.owner().owner_count(), 2);
    }
}
