//! This module defines functions within the Drive struct related to identities.
//! Functions include inserting new identities into the `Identities` subtree and
//! fetching identities from the subtree.
//!

#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::RootTree;
#[cfg(feature = "server")]
use crate::util::object_size_info::DriveKeyInfo;
use std::fmt;

#[cfg(any(feature = "server", feature = "verify"))]
use dpp::identity::Purpose;
#[cfg(feature = "server")]
use dpp::identity::{KeyID, SecurityLevel};

#[cfg(feature = "server")]
/// Everything related to withdrawals
pub mod withdrawals;

#[cfg(feature = "server")]
use dpp::identity::Purpose::AUTHENTICATION;
#[cfg(feature = "server")]
use integer_encoding::VarInt;

#[cfg(any(feature = "server", feature = "verify"))]
mod balance;
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) mod contract_info;
#[cfg(feature = "server")]
mod estimation_costs;
#[cfg(any(feature = "server", feature = "verify"))]
mod fetch;
#[cfg(feature = "server")]
mod insert;
#[cfg(any(feature = "server", feature = "verify"))]
/// Module related to Identity Keys
pub mod key;
/// Module related to updating of identity
#[cfg(feature = "server")]
pub mod update;

use crate::drive::identity::contract_info::ContractInfoStructure;
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(any(feature = "server", feature = "verify"))]
pub use fetch::queries::*;

/// Identity path
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn identity_path(identity_id: &[u8]) -> [&[u8]; 2] {
    [Into::<&[u8; 1]>::into(RootTree::Identities), identity_id]
}

/// Identity path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn identity_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::Identities).to_vec(),
        identity_id.to_vec(),
    ]
}

#[cfg(feature = "server")]
/// The path for the contract info for an identity
pub fn identity_contract_info_root_path(identity_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityContractInfo),
    ]
}

#[cfg(feature = "server")]
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
#[cfg(any(feature = "server", feature = "verify"))]
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

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
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
#[cfg(any(feature = "server", feature = "verify"))]
/// Identity key tree path
pub(crate) fn identity_key_tree_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
    ]
}

#[cfg(any(feature = "server", feature = "verify"))]
/// The path for all identity keys as a vec
pub fn identity_key_tree_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeys as u8],
    ]
}

#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
/// Identity key location within identity vector
pub(crate) fn identity_key_location_within_identity_vec(encoded_key_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys).to_vec(),
        encoded_key_id.to_vec(),
    ]
}

/// identity query keys tree path
#[cfg(feature = "server")]
/// Identity query keys tree path
pub(crate) fn identity_query_keys_tree_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
    ]
}

/// identity query keys tree path vec
#[cfg(any(feature = "server", feature = "verify"))]
/// Identity query keys tree path vector
pub(crate) fn identity_query_keys_tree_path_vec(identity_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeyReferences as u8],
    ]
}

/// identity query keys purpose tree path
#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
/// Identity query keys full tree path
pub(crate) fn identity_query_keys_for_direct_searchable_reference_full_tree_path(
    purpose: Purpose,
    identity_id: &[u8],
) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
        Into::<&[u8; 1]>::into(purpose),
    ]
}

/// identity query keys full tree path
#[cfg(feature = "server")]
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
#[cfg(any(feature = "server", feature = "verify"))]
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

#[cfg(any(feature = "server", feature = "verify"))]
impl fmt::Display for IdentityRootStructure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant_name = match self {
            IdentityRootStructure::IdentityTreeRevision => "Revision",
            IdentityRootStructure::IdentityTreeNonce => "Nonce",
            IdentityRootStructure::IdentityTreeKeys => "IdentityKeys",
            IdentityRootStructure::IdentityTreeKeyReferences => "IdentityKeyReferences",
            IdentityRootStructure::IdentityTreeNegativeCredit => "NegativeCredit",
            IdentityRootStructure::IdentityContractInfo => "ContractInfo",
        };
        write!(f, "{}", variant_name)
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl TryFrom<u8> for IdentityRootStructure {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            192 => Ok(IdentityRootStructure::IdentityTreeRevision),
            64 => Ok(IdentityRootStructure::IdentityTreeNonce),
            128 => Ok(IdentityRootStructure::IdentityTreeKeys),
            160 => Ok(IdentityRootStructure::IdentityTreeKeyReferences),
            96 => Ok(IdentityRootStructure::IdentityTreeNegativeCredit),
            32 => Ok(IdentityRootStructure::IdentityContractInfo),
            _ => Err(Error::Drive(DriveError::NotSupported(
                "unknown identity root structure tree item",
            ))),
        }
    }
}

#[cfg(feature = "server")]
impl IdentityRootStructure {
    fn to_drive_key_info<'a>(self) -> DriveKeyInfo<'a> {
        DriveKeyInfo::Key(vec![self as u8])
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl From<IdentityRootStructure> for u8 {
    fn from(root_tree: IdentityRootStructure) -> Self {
        root_tree as u8
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl From<IdentityRootStructure> for [u8; 1] {
    fn from(root_tree: IdentityRootStructure) -> Self {
        [root_tree as u8]
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
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
