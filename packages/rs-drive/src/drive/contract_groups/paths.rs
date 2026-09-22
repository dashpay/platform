//! Paths of the `ContractGroups` root tree.
//!
//! ```text
//! [124] ContractGroups
//! ├── [0] Groups
//! │   └── <contract group id>
//! │       ├── [0] Info            -> Item(bincode ContractGroupInfo)
//! │       ├── [1] Contracts       -> <contract id> -> Item([])
//! │       ├── [2] DocumentTypes   -> <contract id || document type name> -> Item([])
//! │       └── [3] Tokens          -> <contract id || token position, u16 BE> -> Item([])
//! └── [1] Members
//!     └── <contract id>
//!         ├── [0] Groups          -> <contract group id> -> Reference to Groups/<group>/[1]/<contract id>
//!         ├── [1] DocumentTypes   -> <document type name> -> <contract group id> -> Reference
//!         └── [2] Tokens          -> <token position> -> <contract group id> -> Reference
//! ```
//!
//! Document type and token members of a group sit on one level under a composite key, the
//! 32 byte contract id followed by the name or the position, rather than under a subtree per
//! contract. Keys sort by contract first either way, and a flat level pages with a plain range
//! after the cursor key: a subtree per contract would make a continuation page descend into the
//! cursor's contract, and GroveDB charges an empty descent against the page limit.
//!
//! The `Members` side holds plain GroveDB references back to the forward entries. Memberships
//! are append-only and contracts are never deleted, so a reference can never dangle.

use crate::drive::RootTree;

/// The subtree of the root tree that holds every contract group, keyed by contract group id.
pub const CONTRACT_GROUPS_GROUPS_KEY: &[u8; 1] = &[0];
/// The subtree of the root tree that holds the backwards index, keyed by member contract id.
pub const CONTRACT_GROUPS_MEMBERS_KEY: &[u8; 1] = &[1];

/// Inside a group: the stored `ContractGroupInfo` item.
pub const CONTRACT_GROUP_INFO_KEY: &[u8; 1] = &[0];
/// Inside a group: the whole-contract members, keyed by contract id.
pub const CONTRACT_GROUP_CONTRACTS_KEY: &[u8; 1] = &[1];
/// Inside a group: the document type members, keyed by contract id followed by document type name.
pub const CONTRACT_GROUP_DOCUMENT_TYPES_KEY: &[u8; 1] = &[2];
/// Inside a group: the token members, keyed by contract id followed by token position.
pub const CONTRACT_GROUP_TOKENS_KEY: &[u8; 1] = &[3];

/// Inside a member contract's backwards index: the groups the whole contract belongs to.
pub const CONTRACT_MEMBERSHIPS_GROUPS_KEY: &[u8; 1] = &[0];
/// Inside a member contract's backwards index: the groups each document type belongs to.
pub const CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY: &[u8; 1] = &[1];
/// Inside a member contract's backwards index: the groups each token belongs to.
pub const CONTRACT_MEMBERSHIPS_TOKENS_KEY: &[u8; 1] = &[2];

/// The number of path segments a backwards reference keeps from the root: only the root tree
/// key, so the reference is `[ContractGroups] ++ forward path`.
pub const CONTRACT_GROUP_BACKWARDS_REFERENCE_ROOT_HEIGHT: u8 = 1;

/// `[ContractGroups]`
pub fn contract_groups_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::ContractGroups)]
}

/// `[ContractGroups]`
pub fn contract_groups_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::ContractGroups as u8]]
}

/// `[ContractGroups, Groups]`
pub fn contract_groups_groups_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_GROUPS_KEY,
    ]
}

/// `[ContractGroups, Groups]`
pub fn contract_groups_groups_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
    ]
}

/// `[ContractGroups, Groups, <group id>]`
pub fn contract_group_path(contract_group_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_GROUPS_KEY,
        contract_group_id,
    ]
}

/// `[ContractGroups, Groups, <group id>]`
pub fn contract_group_path_vec(contract_group_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
        contract_group_id.to_vec(),
    ]
}

/// `[ContractGroups, Groups, <group id>, Contracts]`
pub fn contract_group_contracts_path(contract_group_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_GROUPS_KEY,
        contract_group_id,
        CONTRACT_GROUP_CONTRACTS_KEY,
    ]
}

/// `[ContractGroups, Groups, <group id>, Contracts]`
pub fn contract_group_contracts_path_vec(contract_group_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
        contract_group_id.to_vec(),
        CONTRACT_GROUP_CONTRACTS_KEY.to_vec(),
    ]
}

/// `[ContractGroups, Groups, <group id>, DocumentTypes]`
pub fn contract_group_document_types_path(contract_group_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_GROUPS_KEY,
        contract_group_id,
        CONTRACT_GROUP_DOCUMENT_TYPES_KEY,
    ]
}

/// `[ContractGroups, Groups, <group id>, DocumentTypes]`
pub fn contract_group_document_types_path_vec(contract_group_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
        contract_group_id.to_vec(),
        CONTRACT_GROUP_DOCUMENT_TYPES_KEY.to_vec(),
    ]
}

/// The key of a document type member inside a group's `DocumentTypes` level: the contract id
/// followed by the document type name.
pub fn contract_group_document_type_member_key(
    contract_id: &[u8; 32],
    document_type_name: &str,
) -> Vec<u8> {
    let mut key = Vec::with_capacity(32 + document_type_name.len());
    key.extend_from_slice(contract_id);
    key.extend_from_slice(document_type_name.as_bytes());
    key
}

/// The key of a token member inside a group's `Tokens` level: the contract id followed by the
/// token position, big endian.
pub fn contract_group_token_member_key(contract_id: &[u8; 32], token_position: u16) -> Vec<u8> {
    let mut key = Vec::with_capacity(34);
    key.extend_from_slice(contract_id);
    key.extend_from_slice(&token_position.to_be_bytes());
    key
}

/// `[ContractGroups, Groups, <group id>, Tokens]`
pub fn contract_group_tokens_path(contract_group_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_GROUPS_KEY,
        contract_group_id,
        CONTRACT_GROUP_TOKENS_KEY,
    ]
}

/// `[ContractGroups, Groups, <group id>, Tokens]`
pub fn contract_group_tokens_path_vec(contract_group_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
        contract_group_id.to_vec(),
        CONTRACT_GROUP_TOKENS_KEY.to_vec(),
    ]
}

/// `[ContractGroups, Members]`
pub fn contract_groups_members_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
    ]
}

/// `[ContractGroups, Members]`
pub fn contract_groups_members_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_MEMBERS_KEY.to_vec(),
    ]
}

/// `[ContractGroups, Members, <contract id>]`
pub fn contract_memberships_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
        contract_id,
    ]
}

/// `[ContractGroups, Members, <contract id>]`
pub fn contract_memberships_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_MEMBERS_KEY.to_vec(),
        contract_id.to_vec(),
    ]
}

/// `[ContractGroups, Members, <contract id>, Groups]`
pub fn contract_memberships_groups_path(contract_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
        contract_id,
        CONTRACT_MEMBERSHIPS_GROUPS_KEY,
    ]
}

/// `[ContractGroups, Members, <contract id>, Groups]`
pub fn contract_memberships_groups_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_MEMBERS_KEY.to_vec(),
        contract_id.to_vec(),
        CONTRACT_MEMBERSHIPS_GROUPS_KEY.to_vec(),
    ]
}

/// `[ContractGroups, Members, <contract id>, DocumentTypes]`
pub fn contract_memberships_document_types_path(contract_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
        contract_id,
        CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY,
    ]
}

/// `[ContractGroups, Members, <contract id>, DocumentTypes, <document type name>]`
pub fn contract_memberships_document_type_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
        contract_id,
        CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY,
        document_type_name,
    ]
}

/// `[ContractGroups, Members, <contract id>, DocumentTypes, <document type name>]`
pub fn contract_memberships_document_type_path_vec(
    contract_id: &[u8],
    document_type_name: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_MEMBERS_KEY.to_vec(),
        contract_id.to_vec(),
        CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY.to_vec(),
        document_type_name.to_vec(),
    ]
}

/// `[ContractGroups, Members, <contract id>, Tokens]`
pub fn contract_memberships_tokens_path(contract_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
        contract_id,
        CONTRACT_MEMBERSHIPS_TOKENS_KEY,
    ]
}

/// `[ContractGroups, Members, <contract id>, Tokens, <token position>]`
pub fn contract_memberships_token_path<'a>(
    contract_id: &'a [u8],
    token_position_bytes: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractGroups),
        CONTRACT_GROUPS_MEMBERS_KEY,
        contract_id,
        CONTRACT_MEMBERSHIPS_TOKENS_KEY,
        token_position_bytes,
    ]
}

/// `[ContractGroups, Members, <contract id>, Tokens, <token position>]`
pub fn contract_memberships_token_path_vec(
    contract_id: &[u8],
    token_position_bytes: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ContractGroups as u8],
        CONTRACT_GROUPS_MEMBERS_KEY.to_vec(),
        contract_id.to_vec(),
        CONTRACT_MEMBERSHIPS_TOKENS_KEY.to_vec(),
        token_position_bytes.to_vec(),
    ]
}
