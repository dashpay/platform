// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! This module defines functions within the Drive struct related to identities.
//! Functions include inserting new identities into the `Identities` subtree and
//! fetching identities from the subtree.
//!

#[cfg(feature = "full")]
use crate::drive::object_size_info::DriveKeyInfo;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::RootTree;

#[cfg(any(feature = "full", feature = "verify"))]
use dpp::identity::Purpose;
#[cfg(feature = "full")]
use dpp::identity::{KeyID, SecurityLevel};

#[cfg(feature = "full")]
/// Everything related to withdrawals
pub mod withdrawals;

#[cfg(feature = "full")]
use dpp::identity::Purpose::AUTHENTICATION;
#[cfg(feature = "full")]
use integer_encoding::VarInt;

#[cfg(any(feature = "full", feature = "verify"))]
mod balance;
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) mod contract_info;
#[cfg(feature = "full")]
mod estimation_costs;
#[cfg(any(feature = "full", feature = "verify"))]
mod fetch;
#[cfg(feature = "full")]
mod insert;
#[cfg(any(feature = "full", feature = "verify"))]
/// Module related to Identity Keys
pub mod key;
/// Module related to updating of identity
#[cfg(feature = "full")]
pub mod update;

#[cfg(feature = "full")]
pub use withdrawals::paths::add_initial_withdrawal_state_structure_operations;

use crate::drive::identity::contract_info::ContractInfoStructure;
#[cfg(any(feature = "full", feature = "verify"))]
pub use fetch::queries::*;

/// Identity path
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) fn identity_path(identity_id: &[u8]) -> [&[u8]; 2] {
    [Into::<&[u8; 1]>::into(RootTree::Identities), identity_id]
}

/// Identity path vector
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) fn identity_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::Identities).to_vec(),
        identity_id.to_vec(),
    ]
}

#[cfg(feature = "full")]
/// The path for the contract info for an identity
pub fn identity_contract_info_root_path(identity_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityContractInfo),
    ]
}

#[cfg(feature = "full")]
/// The path for the contract info for an identity as a vec
pub fn identity_contract_info_root_path_vec(identity_id: &[u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityContractInfo as u8],
    ]
}

/// The group is either a contract id or on a family of contracts owned by the same identity
pub fn identity_contract_info_group_path<'a>(
    identity_id: &'a [u8; 32],
    group_id: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityContractInfo),
        group_id,
    ]
}

/// The group is either a contract id or on a family of contracts owned by the same identity
pub fn identity_contract_info_group_path_vec(
    identity_id: &[u8; 32],
    group_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityContractInfo as u8],
        group_id.to_vec(),
    ]
}

/// The group is either a contract id or on a family of contracts owned by the same identity
pub fn identity_contract_info_group_keys_path_vec(
    identity_id: &[u8; 32],
    group_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityContractInfo as u8],
        group_id.to_vec(),
        vec![ContractInfoStructure::ContractInfoKeysKey as u8],
    ]
}

/// The group is either a contract id or on a family of contracts owned by the same identity
#[cfg(any(feature = "full", feature = "verify"))]
pub fn identity_contract_info_group_path_key_purpose_vec(
    identity_id: &[u8; 32],
    group_id: &[u8],
    key_purpose: Purpose,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityContractInfo as u8],
        group_id.to_vec(),
        vec![ContractInfoStructure::ContractInfoKeysKey as u8],
        vec![key_purpose as u8],
    ]
}

#[cfg(feature = "full")]
/// The path for a specific contract info for an identity
pub fn identity_contract_info_path<'a>(
    identity_id: &'a [u8],
    contract_id: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityContractInfo),
        contract_id,
    ]
}

#[cfg(feature = "full")]
/// The path for a specific contract info for an identity as a vec
pub fn identity_contract_info_path_vec(identity_id: &[u8], contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityContractInfo as u8],
        contract_id.to_vec(),
    ]
}

/// identity key tree path
#[cfg(any(feature = "full", feature = "verify"))]
/// Identity key tree path
pub(crate) fn identity_key_tree_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
    ]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// The path for all identity keys as a vec
pub fn identity_key_tree_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeys as u8],
    ]
}

#[cfg(feature = "full")]
/// The path for a specific key as a vec
pub fn identity_key_path_vec(identity_id: &[u8], key_id: KeyID) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeys as u8],
        key_id.encode_var_vec(),
    ]
}

/// identity key location within identity vec
#[cfg(feature = "full")]
/// Identity key location within identity vector
pub(crate) fn identity_key_location_within_identity_vec(encoded_key_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys).to_vec(),
        encoded_key_id.to_vec(),
    ]
}

/// identity query keys tree path
#[cfg(feature = "full")]
/// Identity query keys tree path
pub(crate) fn identity_query_keys_tree_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
    ]
}

/// identity query keys tree path vec
#[cfg(any(feature = "full", feature = "verify"))]
/// Identity query keys tree path vector
pub(crate) fn identity_query_keys_tree_path_vec(identity_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeyReferences as u8],
    ]
}

/// identity query keys purpose tree path
#[cfg(feature = "full")]
/// Identity query keys purpose tree path
pub(crate) fn identity_query_keys_purpose_tree_path<'a>(
    identity_id: &'a [u8],
    purpose: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
        purpose,
    ]
}

/// identity query keys purpose tree path vec
#[cfg(feature = "full")]
/// Identity query keys purpose tree path vec
pub(crate) fn identity_query_keys_purpose_tree_path_vec(
    identity_id: &[u8],
    purpose: Purpose,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeyReferences as u8],
        vec![purpose as u8],
    ]
}

/// identity query keys security level tree path vec
#[cfg(feature = "full")]
/// Identity query keys security level tree path vec
pub(crate) fn identity_query_keys_security_level_tree_path_vec(
    identity_id: &[u8],
    security_level: SecurityLevel,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeyReferences as u8],
        vec![AUTHENTICATION as u8],
        vec![security_level as u8],
    ]
}

/// identity query keys full tree path
#[cfg(feature = "full")]
/// Identity query keys full tree path
pub(crate) fn identity_query_keys_for_transfer_full_tree_path(identity_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
        Into::<&[u8; 1]>::into(Purpose::TRANSFER),
    ]
}

/// identity query keys full tree path
#[cfg(feature = "full")]
/// Identity query keys full tree path
pub(crate) fn identity_query_keys_for_authentication_full_tree_path<'a>(
    identity_id: &'a [u8],
    security_level: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
        Into::<&[u8; 1]>::into(Purpose::AUTHENTICATION),
        security_level,
    ]
}

/// The root structure of identities
#[cfg(any(feature = "full", feature = "verify"))]
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum IdentityRootStructure {
    /// The revision of the identity
    IdentityTreeRevision = 192,
    /// The nonce of the identity, it is used to prevent replay attacks
    IdentityTreeNonce = 64,
    /// The keys that an identity has
    IdentityTreeKeys = 128,
    /// A Way to search for specific keys
    IdentityTreeKeyReferences = 160,
    /// Owed processing fees
    IdentityTreeNegativeCredit = 96,
    /// Identity contract information
    IdentityContractInfo = 32,
}

#[cfg(feature = "full")]
impl IdentityRootStructure {
    fn to_drive_key_info<'a>(self) -> DriveKeyInfo<'a> {
        DriveKeyInfo::Key(vec![self as u8])
    }
}

#[cfg(any(feature = "full", feature = "verify"))]
impl From<IdentityRootStructure> for u8 {
    fn from(root_tree: IdentityRootStructure) -> Self {
        root_tree as u8
    }
}

#[cfg(any(feature = "full", feature = "verify"))]
impl From<IdentityRootStructure> for [u8; 1] {
    fn from(root_tree: IdentityRootStructure) -> Self {
        [root_tree as u8]
    }
}

#[cfg(any(feature = "full", feature = "verify"))]
impl From<IdentityRootStructure> for &'static [u8; 1] {
    fn from(identity_tree: IdentityRootStructure) -> Self {
        match identity_tree {
            IdentityRootStructure::IdentityTreeRevision => &[192],
            IdentityRootStructure::IdentityTreeNonce => &[64],
            IdentityRootStructure::IdentityTreeKeys => &[128],
            IdentityRootStructure::IdentityTreeKeyReferences => &[160],
            IdentityRootStructure::IdentityTreeNegativeCredit => &[96],
            IdentityRootStructure::IdentityContractInfo => &[32],
        }
    }
}
