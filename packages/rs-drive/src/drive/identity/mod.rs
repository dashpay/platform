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

use crate::drive::object_size_info::DriveKeyInfo;
use crate::drive::RootTree;


use dpp::identity::KeyID;

use integer_encoding::{VarInt};


mod estimation_costs;
mod fetch;
mod insert;
mod key;
mod update;
mod withdrawal_queue;


pub use withdrawal_queue::add_initial_withdrawal_state_structure_operations;

pub(crate) const IDENTITY_KEY: [u8; 1] = [0];

pub(crate) fn identity_path(identity_id: &[u8]) -> [&[u8]; 2] {
    [Into::<&[u8; 1]>::into(RootTree::Identities), identity_id]
}

pub(crate) fn identity_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::Identities).to_vec(),
        identity_id.to_vec(),
    ]
}

pub(crate) fn balance_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Balances)]
}

pub(crate) fn balance_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Balances).to_vec()]
}

pub(crate) fn identity_key_tree_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
    ]
}

pub(crate) fn identity_key_tree_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeys as u8],
    ]
}

pub(crate) fn identity_key_path_vec(identity_id: &[u8], key_id: KeyID) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeys as u8],
        key_id.encode_var_vec(),
    ]
}

pub(crate) fn identity_key_location_within_identity_vec(encoded_key_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys).to_vec(),
        encoded_key_id.to_vec(),
    ]
}

pub(crate) fn identity_query_keys_tree_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
    ]
}

pub(crate) fn identity_query_keys_tree_path_vec(identity_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeyReferences as u8],
    ]
}

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

pub(crate) fn identity_query_keys_full_tree_path<'a>(
    identity_id: &'a [u8],
    purpose: &'a [u8],
    security_level: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyReferences),
        purpose,
        security_level,
    ]
}

/// The root structure of identities
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum IdentityRootStructure {
    /// The revision of identity data
    IdentityTreeRevision = 0,
    /// The keys that an identity has
    IdentityTreeKeys = 1,
    /// A Way to search for specific keys
    IdentityTreeKeyReferences = 2,
    /// Owed processing fees
    IdentityTreeNegativeCredit = 3,
}

impl IdentityRootStructure {
    fn to_drive_key_info(&self) -> DriveKeyInfo {
        DriveKeyInfo::Key(vec![*self as u8])
    }
}

impl From<IdentityRootStructure> for u8 {
    fn from(root_tree: IdentityRootStructure) -> Self {
        root_tree as u8
    }
}

impl From<IdentityRootStructure> for [u8; 1] {
    fn from(root_tree: IdentityRootStructure) -> Self {
        [root_tree as u8]
    }
}

impl From<IdentityRootStructure> for &'static [u8; 1] {
    fn from(identity_tree: IdentityRootStructure) -> Self {
        match identity_tree {
            IdentityRootStructure::IdentityTreeRevision => &[0],
            IdentityRootStructure::IdentityTreeKeys => &[1],
            IdentityRootStructure::IdentityTreeKeyReferences => &[2],
            IdentityRootStructure::IdentityTreeNegativeCredit => &[3],
        }
    }
}
