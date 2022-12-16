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
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::batch::key_info::KeyInfo;
use serde_json::to_vec;

pub mod fetch;
pub mod insert;
pub mod key;
pub mod update;
pub mod withdrawal_queue;

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
    [Into::<&[u8; 1]>::into(RootTree::Identities)]
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

pub(crate) fn identity_key_location_within_identity_vec(encoded_key_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys).to_vec(),
        encoded_key_id.to_vec(),
    ]
}

pub(crate) fn identity_query_keys_tree_path(identity_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
        &[],
    ]
}

pub(crate) fn identity_query_keys_purpose_tree_path<'a>(
    identity_id: &'a [u8],
    purpose: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
        &[],
        purpose,
    ]
}

pub(crate) fn identity_query_keys_full_tree_path<'a>(
    identity_id: &'a [u8],
    purpose: &'a [u8],
    security_level: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys),
        &[],
        purpose,
        security_level,
    ]
}

#[repr(u8)]
pub enum IdentityRootStructure {
    // Input data errors
    IdentityTreeRevision = 0,
    IdentityTreeKeys = 1,
    IdentityTreeNegativeCredit = 2,
}

impl IdentityRootStructure {
    fn to_drive_key_info(&self) -> DriveKeyInfo {
        DriveKeyInfo::Key(Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeys).to_vec())
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
            IdentityRootStructure::IdentityTreeNegativeCredit => &[2],
        }
    }
}

pub fn balance_from_bytes(identity_balance_bytes: &[u8]) -> Result<u64, Error> {
    let balance_bytes: [u8; 8] = identity_balance_bytes.try_into().map_err(|_| {
        Error::Drive(DriveError::CorruptedElementType(
            "identity balance was not represented in 8 bytes",
        ))
    })?;
    Ok(i64::from_be_bytes(balance_bytes) as u64)
}
