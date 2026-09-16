use crate::drive::contract_groups::paths::{
    CONTRACT_GROUP_CONTRACTS_KEY, CONTRACT_GROUP_DOCUMENT_TYPES_KEY, CONTRACT_GROUP_INFO_KEY,
    CONTRACT_GROUP_TOKENS_KEY, CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY,
    CONTRACT_MEMBERSHIPS_GROUPS_KEY, CONTRACT_MEMBERSHIPS_TOKENS_KEY,
};
use dpp::contract_group::ContractGroupInfo;
use dpp::data_contract::TokenContractPosition;
use dpp::identifier::Identifier;
use dpp::serialization::{PlatformDeserializableTrusted, PlatformDeserializableUntrusted};
use grovedb::Element;
use std::collections::{BTreeMap, BTreeSet};

/// The members of one contract group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractGroupMembers {
    /// Contracts that belong to the group as a whole.
    pub contracts: BTreeSet<Identifier>,
    /// Document types that belong to the group, by contract.
    pub document_types: BTreeMap<Identifier, BTreeSet<String>>,
    /// Tokens that belong to the group, by contract and token position.
    pub tokens: BTreeMap<Identifier, BTreeSet<TokenContractPosition>>,
}

impl ContractGroupMembers {
    /// Whether the group has no members at all.
    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty() && self.document_types.is_empty() && self.tokens.is_empty()
    }
}

/// A contract group with its stored information and its members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractGroup {
    /// The contract group id.
    pub id: Identifier,
    /// The stored information: owner, name, description.
    pub info: ContractGroupInfo,
    /// The members.
    pub members: ContractGroupMembers,
}

/// Whether stored bytes come from this node's own state or from a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeTrust {
    /// Bytes read from this node's own GroveDB.
    Trusted,
    /// Bytes returned by another party, for instance inside a proof.
    Untrusted,
}

fn identifier_from(bytes: &[u8], what: &str) -> Result<Identifier, String> {
    Identifier::from_bytes(bytes).map_err(|_| {
        format!(
            "{} is not a 32 byte identifier: {} bytes",
            what,
            bytes.len()
        )
    })
}

fn token_position_from(bytes: &[u8]) -> Result<TokenContractPosition, String> {
    let bytes: [u8; 2] = bytes
        .try_into()
        .map_err(|_| format!("token position is not encoded on 2 bytes: {}", bytes.len()))?;
    Ok(TokenContractPosition::from_be_bytes(bytes))
}

fn document_type_name_from(bytes: &[u8]) -> Result<String, String> {
    String::from_utf8(bytes.to_vec())
        .map_err(|_| "document type name is not valid UTF-8".to_string())
}

impl ContractGroup {
    /// Rebuilds a contract group from the path, key and element triples the
    /// [`Drive::contract_group_query`](crate::drive::Drive::contract_group_query) returns, whether
    /// from a raw query or from a verified proof.
    ///
    /// Returns `Ok(None)` when the results hold no info item, which is how an absent group looks.
    /// Returns `Err` with a description when a triple does not fit the group layout.
    pub fn from_path_key_elements<I>(
        contract_group_id: Identifier,
        path_key_elements: I,
        trust: DecodeTrust,
    ) -> Result<Option<Self>, String>
    where
        I: IntoIterator<Item = (Vec<Vec<u8>>, Vec<u8>, Element)>,
    {
        let mut info = None;
        let mut members = ContractGroupMembers::default();
        for (path, key, element) in path_key_elements {
            // The group path is [ContractGroups, Groups, <group id>]: three segments.
            match (path.len(), path.get(3).map(Vec::as_slice)) {
                (3, None) if key.as_slice() == CONTRACT_GROUP_INFO_KEY => {
                    let Element::Item(bytes, _) = element else {
                        return Err("contract group info is not an item".to_string());
                    };
                    let decoded = match trust {
                        DecodeTrust::Trusted => {
                            ContractGroupInfo::deserialize_from_bytes_trusted(&bytes)
                        }
                        DecodeTrust::Untrusted => {
                            ContractGroupInfo::deserialize_from_bytes_untrusted(&bytes)
                        }
                    }
                    .map_err(|e| format!("contract group info does not decode: {}", e))?;
                    info = Some(decoded);
                }
                (4, Some(subtree)) if subtree == CONTRACT_GROUP_CONTRACTS_KEY => {
                    members
                        .contracts
                        .insert(identifier_from(&key, "member contract id")?);
                }
                (5, Some(subtree)) if subtree == CONTRACT_GROUP_DOCUMENT_TYPES_KEY => {
                    let contract_id = identifier_from(&path[4], "member contract id")?;
                    members
                        .document_types
                        .entry(contract_id)
                        .or_default()
                        .insert(document_type_name_from(&key)?);
                }
                (5, Some(subtree)) if subtree == CONTRACT_GROUP_TOKENS_KEY => {
                    let contract_id = identifier_from(&path[4], "member contract id")?;
                    members
                        .tokens
                        .entry(contract_id)
                        .or_default()
                        .insert(token_position_from(&key)?);
                }
                _ => {
                    return Err(format!(
                        "unexpected entry at path depth {} with key {}",
                        path.len(),
                        hex::encode(&key)
                    ));
                }
            }
        }
        Ok(info.map(|info| ContractGroup {
            id: contract_group_id,
            info,
            members,
        }))
    }
}

/// The contract groups a contract belongs to: as a whole, through its document types, and
/// through its tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractGroupMembershipsForContract {
    /// Groups the whole contract belongs to.
    pub contract: BTreeSet<Identifier>,
    /// Groups each document type belongs to, by document type name.
    pub document_types: BTreeMap<String, BTreeSet<Identifier>>,
    /// Groups each token belongs to, by token position.
    pub tokens: BTreeMap<TokenContractPosition, BTreeSet<Identifier>>,
}

impl ContractGroupMembershipsForContract {
    /// Whether the contract belongs to no group in any way.
    pub fn is_empty(&self) -> bool {
        self.contract.is_empty() && self.document_types.is_empty() && self.tokens.is_empty()
    }

    /// Every group id the contract touches, in any way.
    pub fn all_contract_group_ids(&self) -> BTreeSet<Identifier> {
        self.contract
            .iter()
            .copied()
            .chain(self.document_types.values().flatten().copied())
            .chain(self.tokens.values().flatten().copied())
            .collect()
    }

    /// Rebuilds the memberships from the path, key and element triples the
    /// [`Drive::contract_group_memberships_for_contract_query`](crate::drive::Drive::contract_group_memberships_for_contract_query)
    /// returns, whether from a raw query (references unresolved) or from a verified proof
    /// (references resolved to the forward items). Only paths and keys carry information.
    pub fn from_path_key_elements<I>(path_key_elements: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (Vec<Vec<u8>>, Vec<u8>, Element)>,
    {
        let mut memberships = Self::default();
        for (path, key, _element) in path_key_elements {
            // The memberships path is [ContractGroups, Members, <contract id>]: three segments.
            match (path.len(), path.get(3).map(Vec::as_slice)) {
                (4, Some(subtree)) if subtree == CONTRACT_MEMBERSHIPS_GROUPS_KEY => {
                    memberships
                        .contract
                        .insert(identifier_from(&key, "contract group id")?);
                }
                (5, Some(subtree)) if subtree == CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY => {
                    memberships
                        .document_types
                        .entry(document_type_name_from(&path[4])?)
                        .or_default()
                        .insert(identifier_from(&key, "contract group id")?);
                }
                (5, Some(subtree)) if subtree == CONTRACT_MEMBERSHIPS_TOKENS_KEY => {
                    memberships
                        .tokens
                        .entry(token_position_from(&path[4])?)
                        .or_default()
                        .insert(identifier_from(&key, "contract group id")?);
                }
                _ => {
                    return Err(format!(
                        "unexpected entry at path depth {} with key {}",
                        path.len(),
                        hex::encode(&key)
                    ));
                }
            }
        }
        Ok(memberships)
    }
}
