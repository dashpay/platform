use crate::drive::contract_groups::paths::{
    CONTRACT_GROUP_CONTRACTS_KEY, CONTRACT_GROUP_DOCUMENT_TYPES_KEY, CONTRACT_GROUP_TOKENS_KEY,
    CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY, CONTRACT_MEMBERSHIPS_GROUPS_KEY,
    CONTRACT_MEMBERSHIPS_TOKENS_KEY,
};
use dpp::data_contract::TokenContractPosition;
use dpp::identifier::Identifier;
use grovedb::Element;
use std::collections::{BTreeMap, BTreeSet};

/// Which members of a contract group to read, and where to continue from.
///
/// A group can be joined by any number of contracts, so its members are read one kind at a time,
/// in key order, in pages bounded by a limit. The cursor of a page is its last entry; pass it as
/// `start_after` to read the next page. A page shorter than the limit is the last one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractGroupMembersQuery {
    /// Contracts that belong to the group as a whole, in contract id order.
    Contracts {
        /// Continue after this contract id.
        start_after: Option<Identifier>,
    },
    /// Document types that belong to the group, in contract id then document type name order.
    DocumentTypes {
        /// Continue after this contract id and document type name.
        start_after: Option<(Identifier, String)>,
    },
    /// Tokens that belong to the group, in contract id then token position order.
    Tokens {
        /// Continue after this contract id and token position.
        start_after: Option<(Identifier, TokenContractPosition)>,
    },
}

/// One page of a contract group's members of one kind, in the order the query defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractGroupMembersPage {
    /// Contracts that belong to the group as a whole.
    Contracts(Vec<Identifier>),
    /// Document types that belong to the group, as contract id and document type name.
    DocumentTypes(Vec<(Identifier, String)>),
    /// Tokens that belong to the group, as contract id and token position.
    Tokens(Vec<(Identifier, TokenContractPosition)>),
}

impl ContractGroupMembersPage {
    /// An empty page of the kind the query asks for.
    pub fn empty_for(query: &ContractGroupMembersQuery) -> Self {
        match query {
            ContractGroupMembersQuery::Contracts { .. } => Self::Contracts(vec![]),
            ContractGroupMembersQuery::DocumentTypes { .. } => Self::DocumentTypes(vec![]),
            ContractGroupMembersQuery::Tokens { .. } => Self::Tokens(vec![]),
        }
    }

    /// The number of members on the page.
    pub fn len(&self) -> usize {
        match self {
            Self::Contracts(entries) => entries.len(),
            Self::DocumentTypes(entries) => entries.len(),
            Self::Tokens(entries) => entries.len(),
        }
    }

    /// Whether the page holds no members.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The query for the page after this one, or `None` when this page is empty.
    pub fn next_query(&self) -> Option<ContractGroupMembersQuery> {
        match self {
            Self::Contracts(entries) => {
                entries
                    .last()
                    .map(|contract_id| ContractGroupMembersQuery::Contracts {
                        start_after: Some(*contract_id),
                    })
            }
            Self::DocumentTypes(entries) => {
                entries.last().map(
                    |(contract_id, name)| ContractGroupMembersQuery::DocumentTypes {
                        start_after: Some((*contract_id, name.clone())),
                    },
                )
            }
            Self::Tokens(entries) => {
                entries.last().map(
                    |(contract_id, position)| ContractGroupMembersQuery::Tokens {
                        start_after: Some((*contract_id, *position)),
                    },
                )
            }
        }
    }

    /// Rebuilds a page from the path, key and element triples the
    /// [`Drive::contract_group_members_query`](crate::drive::Drive::contract_group_members_query)
    /// returns, whether from a raw query or from a verified proof. The elements carry no data;
    /// every member is read from its path and key.
    ///
    /// Returns `Err` with a description when a triple does not fit the kind's layout.
    pub fn from_path_key_elements<I>(
        query: &ContractGroupMembersQuery,
        path_key_elements: I,
    ) -> Result<Self, String>
    where
        I: IntoIterator<Item = (Vec<Vec<u8>>, Vec<u8>, Element)>,
    {
        // The kind's subtree path is [ContractGroups, Groups, <group id>, <kind>]: four segments.
        let mut page = Self::empty_for(query);
        for (path, key, _element) in path_key_elements {
            match (&mut page, path.len(), path.get(3).map(Vec::as_slice)) {
                (Self::Contracts(entries), 4, Some(kind))
                    if kind == CONTRACT_GROUP_CONTRACTS_KEY =>
                {
                    entries.push(identifier_from(&key, "member contract id")?);
                }
                (Self::DocumentTypes(entries), 5, Some(kind))
                    if kind == CONTRACT_GROUP_DOCUMENT_TYPES_KEY =>
                {
                    entries.push((
                        identifier_from(&path[4], "member contract id")?,
                        document_type_name_from(&key)?,
                    ));
                }
                (Self::Tokens(entries), 5, Some(kind)) if kind == CONTRACT_GROUP_TOKENS_KEY => {
                    entries.push((
                        identifier_from(&path[4], "member contract id")?,
                        token_position_from(&key)?,
                    ));
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
        Ok(page)
    }
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

/// The contract groups one contract belongs to: as a whole, through its document types, and
/// through its tokens. Bounded by the memberships one create transition may declare, since
/// memberships are recorded at creation only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractGroupMembershipsForContract {
    /// The groups the whole contract belongs to.
    pub contract: BTreeSet<Identifier>,
    /// The groups each document type belongs to, by document type name.
    pub document_types: BTreeMap<String, BTreeSet<Identifier>>,
    /// The groups each token belongs to, by token position.
    pub tokens: BTreeMap<TokenContractPosition, BTreeSet<Identifier>>,
}

impl ContractGroupMembershipsForContract {
    /// Whether the contract belongs to no group at all.
    pub fn is_empty(&self) -> bool {
        self.contract.is_empty() && self.document_types.is_empty() && self.tokens.is_empty()
    }

    /// Every group the contract belongs to in any way.
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
    /// returns, whether from a raw query or from a verified proof.
    ///
    /// Returns `Err` with a description when a triple does not fit the memberships layout.
    pub fn from_path_key_elements<I>(path_key_elements: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (Vec<Vec<u8>>, Vec<u8>, Element)>,
    {
        // The contract path is [ContractGroups, Members, <contract id>]: three segments.
        let mut memberships = Self::default();
        for (path, key, _element) in path_key_elements {
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
