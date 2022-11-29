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

//! Flags
//!

use crate::drive::flags::StorageFlags::{
    MultiEpoch, MultiEpochOwned, SingleEpoch, SingleEpochOwned,
};
use costs::storage_cost::removal::StorageRemovedBytes::SectionedStorageRemoval;
use costs::storage_cost::removal::{StorageRemovalPerEpochByIdentifier, StorageRemovedBytes};
use grovedb::ElementFlags;
use integer_encoding::VarInt;
use intmap::IntMap;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::error::storage_flags::StorageFlagsError;
use crate::error::Error;

type EpochIndex = u16;

type BaseEpoch = EpochIndex;

type BytesAddedInEpoch = u32;

type OwnerId = [u8; 32];

/// Storage flags
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageFlags {
    /// Single epoch
    /// represented as byte 0
    SingleEpoch(BaseEpoch),

    /// Multi epoch
    /// represented as byte 1
    MultiEpoch(BaseEpoch, BTreeMap<EpochIndex, BytesAddedInEpoch>),

    /// Single epoch owned
    /// represented as byte 2
    SingleEpochOwned(BaseEpoch, OwnerId),

    /// Multi epoch owned
    /// represented as byte 3
    MultiEpochOwned(BaseEpoch, BTreeMap<EpochIndex, BytesAddedInEpoch>, OwnerId),
}

impl StorageFlags {
    /// Create new single epoch storage flags
    pub fn new_single_epoch(epoch: BaseEpoch, maybe_owner_id: Option<OwnerId>) -> Self {
        match maybe_owner_id {
            None => SingleEpoch(epoch),
            Some(owner_id) => SingleEpochOwned(epoch, owner_id),
        }
    }

    fn combine_owner_id<'a>(&'a self, rhs: &'a Self) -> Result<Option<&'a OwnerId>, Error> {
        if let Some(our_owner_id) = self.owner_id() {
            if let Some(other_owner_id) = rhs.owner_id() {
                if our_owner_id != other_owner_id {
                    return Err(Error::StorageFlags(
                        StorageFlagsError::MergingStorageFlagsFromDifferentOwners(
                            "can not merge from different owners",
                        ),
                    ));
                }
            }
            Ok(Some(our_owner_id))
        } else if let Some(other_owner_id) = rhs.owner_id() {
            Ok(Some(other_owner_id))
        } else {
            Ok(None)
        }
    }

    fn combine_non_base_epoch_bytes(
        &self,
        rhs: &Self,
    ) -> Option<BTreeMap<EpochIndex, BytesAddedInEpoch>> {
        if let Some(our_epoch_index_map) = self.epoch_index_map() {
            if let Some(other_epoch_index_map) = rhs.epoch_index_map() {
                let mut combined_index_map = our_epoch_index_map.clone();
                other_epoch_index_map
                    .iter()
                    .for_each(|(epoch_index, bytes_added)| {
                        let original_value = combined_index_map.remove(epoch_index);
                        match original_value {
                            None => combined_index_map.insert(*epoch_index, *bytes_added),
                            Some(original_bytes) => combined_index_map
                                .insert(*epoch_index, original_bytes + *bytes_added),
                        };
                    });
                Some(combined_index_map)
            } else {
                Some(our_epoch_index_map.clone())
            }
        } else {
            rhs.epoch_index_map().cloned()
        }
    }

    fn combine_same_base_epoch(&self, rhs: Self) -> Result<Self, Error> {
        let base_epoch = *self.base_epoch();
        let owner_id = self.combine_owner_id(&rhs)?;
        let other_epoch_bytes = self.combine_non_base_epoch_bytes(&rhs);

        match (owner_id, other_epoch_bytes) {
            (None, None) => Ok(SingleEpoch(base_epoch)),
            (Some(owner_id), None) => Ok(SingleEpochOwned(base_epoch, *owner_id)),
            (None, Some(other_epoch_bytes)) => Ok(MultiEpoch(base_epoch, other_epoch_bytes)),
            (Some(owner_id), Some(other_epoch_bytes)) => {
                Ok(MultiEpochOwned(base_epoch, other_epoch_bytes, *owner_id))
            }
        }
    }

    fn combine_with_higher_base_epoch(&self, rhs: Self, added_bytes: u32) -> Result<Self, Error> {
        let base_epoch = *self.base_epoch();
        let epoch_with_adding_bytes = rhs.base_epoch();
        let owner_id = self.combine_owner_id(&rhs)?;
        let mut other_epoch_bytes = self.combine_non_base_epoch_bytes(&rhs).unwrap_or_default();
        let original_value = other_epoch_bytes.remove(epoch_with_adding_bytes);
        match original_value {
            None => other_epoch_bytes.insert(*epoch_with_adding_bytes, added_bytes),
            Some(original_bytes) => {
                other_epoch_bytes.insert(*epoch_with_adding_bytes, original_bytes + added_bytes)
            }
        };

        match owner_id {
            None => Ok(MultiEpoch(base_epoch, other_epoch_bytes)),
            Some(owner_id) => Ok(MultiEpochOwned(base_epoch, other_epoch_bytes, *owner_id)),
        }
    }

    fn combine_with_higher_base_epoch_remove_bytes(
        self,
        rhs: Self,
        removed_bytes: &StorageRemovedBytes,
    ) -> Result<Self, Error> {
        let base_epoch = *self.base_epoch();
        let owner_id = self.combine_owner_id(&rhs)?;
        let mut other_epoch_bytes = self.combine_non_base_epoch_bytes(&rhs).unwrap_or_default();
        if let SectionedStorageRemoval(sectioned_bytes_by_identifier) = removed_bytes {
            if sectioned_bytes_by_identifier.len() > 1 {
                return Err(Error::StorageFlags(
                    StorageFlagsError::MergingStorageFlagsFromDifferentOwners(
                        "can not remove bytes when there is no epoch",
                    ),
                ));
            }
            let identifier = owner_id.copied().unwrap_or_default();
            let sectioned_bytes =
                sectioned_bytes_by_identifier
                    .get(&identifier)
                    .ok_or(Error::StorageFlags(
                        StorageFlagsError::MergingStorageFlagsFromDifferentOwners(
                            "can not remove bytes when there is no epoch",
                        ),
                    ))?;
            sectioned_bytes
                .iter()
                .try_for_each(|(epoch, removed_bytes)| {
                    let bytes_added_in_epoch =
                        other_epoch_bytes
                            .get_mut(&(*epoch as u16))
                            .ok_or(Error::StorageFlags(
                                StorageFlagsError::RemovingAtEpochWithNoAssociatedStorage(
                                    "can not remove bytes when there is no epoch",
                                ),
                            ))?;
                    *bytes_added_in_epoch =
                        bytes_added_in_epoch.checked_sub(*removed_bytes).ok_or(
                            Error::StorageFlags(StorageFlagsError::StorageFlagsOverflow(
                                "can't remove more bytes than exist at that epoch",
                            )),
                        )?;
                    Ok::<(), Error>(())
                })?;
        }

        match owner_id {
            None => Ok(MultiEpoch(base_epoch, other_epoch_bytes)),
            Some(owner_id) => Ok(MultiEpochOwned(base_epoch, other_epoch_bytes, *owner_id)),
        }
    }

    /// Optional combine added bytes
    pub fn optional_combine_added_bytes(
        ours: Option<Self>,
        theirs: Self,
        added_bytes: u32,
    ) -> Result<Self, Error> {
        match ours {
            None => Ok(theirs),
            Some(ours) => Ok(ours.combine_added_bytes(theirs, added_bytes)?),
        }
    }

    /// Optional combine removed bytes
    pub fn optional_combine_removed_bytes(
        ours: Option<Self>,
        theirs: Self,
        removed_bytes: &StorageRemovedBytes,
    ) -> Result<Self, Error> {
        match ours {
            None => Ok(theirs),
            Some(ours) => Ok(ours.combine_removed_bytes(theirs, removed_bytes)?),
        }
    }

    /// Combine added bytes
    pub fn combine_added_bytes(self, rhs: Self, added_bytes: u32) -> Result<Self, Error> {
        match self.base_epoch().cmp(rhs.base_epoch()) {
            Ordering::Equal => self.combine_same_base_epoch(rhs),
            Ordering::Less => self.combine_with_higher_base_epoch(rhs, added_bytes),
            Ordering::Greater => Err(Error::StorageFlags(
                StorageFlagsError::MergingStorageFlagsWithDifferentBaseEpoch(
                    "can not merge with new item in older base epoch",
                ),
            )),
        }
    }

    /// Combine removed bytes
    pub fn combine_removed_bytes(
        self,
        rhs: Self,
        removed_bytes: &StorageRemovedBytes,
    ) -> Result<Self, Error> {
        match self.base_epoch().cmp(rhs.base_epoch()) {
            Ordering::Equal => self.combine_same_base_epoch(rhs),
            Ordering::Less => self.combine_with_higher_base_epoch_remove_bytes(rhs, removed_bytes),
            Ordering::Greater => Err(Error::StorageFlags(
                StorageFlagsError::MergingStorageFlagsWithDifferentBaseEpoch(
                    "can not merge with new item in older base epoch",
                ),
            )),
        }
    }

    /// Returns base epoch
    pub fn base_epoch(&self) -> &BaseEpoch {
        match self {
            SingleEpoch(base_epoch)
            | MultiEpoch(base_epoch, _)
            | SingleEpochOwned(base_epoch, _)
            | MultiEpochOwned(base_epoch, _, _) => base_epoch,
        }
    }

    /// Returns owner id
    pub fn owner_id(&self) -> Option<&OwnerId> {
        match self {
            SingleEpochOwned(_, owner_id) | MultiEpochOwned(_, _, owner_id) => Some(owner_id),
            _ => None,
        }
    }

    /// Returns epoch index map
    pub fn epoch_index_map(&self) -> Option<&BTreeMap<EpochIndex, BytesAddedInEpoch>> {
        match self {
            MultiEpoch(_, epoch_int_map) | MultiEpochOwned(_, epoch_int_map, _) => {
                Some(epoch_int_map)
            }
            _ => None,
        }
    }

    /// Returns optional default storage flags
    pub fn optional_default() -> Option<Self> {
        None
    }

    /// Returns default optional storage flag as ref
    pub fn optional_default_as_ref() -> Option<&'static Self> {
        None
    }

    /// Returns type byte
    pub fn type_byte(&self) -> u8 {
        match self {
            SingleEpoch(_) => 0,
            MultiEpoch(..) => 1,
            SingleEpochOwned(..) => 2,
            MultiEpochOwned(..) => 3,
        }
    }

    fn append_to_vec_base_epoch(&self, buffer: &mut Vec<u8>) {
        match self {
            SingleEpoch(base_epoch)
            | MultiEpoch(base_epoch, ..)
            | SingleEpochOwned(base_epoch, ..)
            | MultiEpochOwned(base_epoch, ..) => buffer.extend(base_epoch.to_be_bytes()),
        }
    }

    fn maybe_append_to_vec_epoch_map(&self, buffer: &mut Vec<u8>) {
        match self {
            MultiEpoch(_, epoch_map) | MultiEpochOwned(_, epoch_map, _) => {
                epoch_map.iter().for_each(|(epoch_index, bytes_added)| {
                    buffer.extend(epoch_index.to_be_bytes());
                    buffer.extend(bytes_added.encode_var_vec());
                })
            }
            _ => {}
        }
    }

    fn maybe_append_to_vec_owner_id(&self, buffer: &mut Vec<u8>) {
        match self {
            SingleEpochOwned(_, owner_id) | MultiEpochOwned(_, _, owner_id) => {
                buffer.extend(owner_id);
            }
            _ => {}
        }
    }

    /// Serialize storage flags
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = vec![self.type_byte()];
        self.maybe_append_to_vec_owner_id(&mut buffer);
        self.append_to_vec_base_epoch(&mut buffer);
        self.maybe_append_to_vec_epoch_map(&mut buffer);
        buffer
    }

    /// Deserialize single epoch storage flags from bytes
    pub fn deserialize_single_epoch(data: &[u8]) -> Result<Self, Error> {
        if data.len() != 3 {
            Err(Error::StorageFlags(
                StorageFlagsError::StorageFlagsWrongSize("single epoch must be 3 bytes total"),
            ))
        } else {
            let epoch = u16::from_be_bytes(data[1..3].try_into().map_err(|_| {
                Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                    "single epoch must be 3 bytes total",
                ))
            })?);
            Ok(SingleEpoch(epoch))
        }
    }

    /// Deserialize multi epoch storage flags from bytes
    pub fn deserialize_multi_epoch(data: &[u8]) -> Result<Self, Error> {
        let len = data.len();
        if len < 6 {
            Err(Error::StorageFlags(
                StorageFlagsError::StorageFlagsWrongSize(
                    "multi epoch must be at least 6 bytes total",
                ),
            ))
        } else {
            let base_epoch = u16::from_be_bytes(data[1..3].try_into().map_err(|_| {
                Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                    "multi epoch must have enough bytes for the base epoch",
                ))
            })?);
            let mut offset = 3;
            let mut bytes_per_epoch: BTreeMap<u16, u32> = BTreeMap::default();
            while offset + 2 < len {
                // 2 for epoch size
                let epoch_index =
                    u16::from_be_bytes(data[offset..offset + 2].try_into().map_err(|_| {
                        Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                            "multi epoch must have enough bytes epoch indexes",
                        ))
                    })?);
                offset += 2;
                let (bytes_at_epoch, bytes_used) = u32::decode_var(&data[offset..]).ok_or(
                    Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                        "multi epoch must have enough bytes for the amount of bytes used",
                    )),
                )?;
                offset += bytes_used;
                bytes_per_epoch.insert(epoch_index, bytes_at_epoch);
            }
            Ok(MultiEpoch(base_epoch, bytes_per_epoch))
        }
    }

    /// Deserialize single epoch owned storage flags from bytes
    pub fn deserialize_single_epoch_owned(data: &[u8]) -> Result<Self, Error> {
        if data.len() != 35 {
            Err(Error::StorageFlags(
                StorageFlagsError::StorageFlagsWrongSize(
                    "single epoch owned must be 35 bytes total",
                ),
            ))
        } else {
            let owner_id: OwnerId = data[1..33].try_into().map_err(|_| {
                Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                    "single epoch owned must be 35 bytes total for owner id",
                ))
            })?;
            let epoch = u16::from_be_bytes(data[33..35].try_into().map_err(|_| {
                Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                    "single epoch owned must be 35 bytes total for epoch",
                ))
            })?);
            Ok(SingleEpochOwned(epoch, owner_id))
        }
    }

    /// Deserialize multi epoch owned storage flags from bytes
    pub fn deserialize_multi_epoch_owned(data: &[u8]) -> Result<Self, Error> {
        let len = data.len();
        if len < 38 {
            Err(Error::StorageFlags(
                StorageFlagsError::StorageFlagsWrongSize(
                    "multi epoch owned must be at least 38 bytes total",
                ),
            ))
        } else {
            let owner_id: OwnerId = data[1..33].try_into().map_err(|_| {
                Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                    "multi epoch owned must be 38 bytes total for owner id",
                ))
            })?;
            let base_epoch = u16::from_be_bytes(data[33..35].try_into().map_err(|_| {
                Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                    "multi epoch must have enough bytes for the base epoch",
                ))
            })?);
            let mut offset = 3;
            let mut bytes_per_epoch: BTreeMap<u16, u32> = BTreeMap::default();
            while offset + 2 < len {
                // 2 for epoch size
                let epoch_index =
                    u16::from_be_bytes(data[offset..offset + 2].try_into().map_err(|_| {
                        Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                            "multi epoch must have enough bytes epoch indexes",
                        ))
                    })?);
                offset += 2;
                let (bytes_at_epoch, bytes_used) = u32::decode_var(&data[offset..]).ok_or(
                    Error::StorageFlags(StorageFlagsError::StorageFlagsWrongSize(
                        "multi epoch must have enough bytes for the amount of bytes used",
                    )),
                )?;
                offset += bytes_used;
                bytes_per_epoch.insert(epoch_index, bytes_at_epoch);
            }
            Ok(MultiEpochOwned(base_epoch, bytes_per_epoch, owner_id))
        }
    }

    /// Deserialize storage flags from bytes
    pub fn deserialize(data: &[u8]) -> Result<Option<Self>, Error> {
        let first_byte = data.first();
        match first_byte {
            None => Ok(None),
            Some(first_byte) => match *first_byte {
                0 => Ok(Some(Self::deserialize_single_epoch(data)?)),
                1 => Ok(Some(Self::deserialize_multi_epoch(data)?)),
                2 => Ok(Some(Self::deserialize_single_epoch_owned(data)?)),
                3 => Ok(Some(Self::deserialize_multi_epoch_owned(data)?)),
                _ => Err(Error::StorageFlags(
                    StorageFlagsError::DeserializeUnknownStorageFlagsType(
                        "unknown storage flags serialization",
                    ),
                )),
            },
        }
    }

    /// Creates storage flags from a slice.
    pub fn from_slice(data: &[u8]) -> Result<Option<Self>, Error> {
        Self::deserialize(data)
    }

    /// Creates storage flags from element flags.
    pub fn from_element_flags_ref(data: &ElementFlags) -> Result<Option<Self>, Error> {
        Self::from_slice(data.as_slice())
    }

    /// Create Storage flags from optional element flags ref
    pub fn from_some_element_flags_ref(data: &Option<ElementFlags>) -> Result<Option<Self>, Error> {
        match data {
            None => Ok(None),
            Some(data) => Self::from_slice(data.as_slice()),
        }
    }

    /// Map to owned optional element flags
    pub fn map_owned_to_element_flags(maybe_storage_flags: Option<Self>) -> ElementFlags {
        maybe_storage_flags
            .map(|storage_flags| storage_flags.serialize())
            .unwrap_or_default()
    }

    /// Map to optional element flags
    pub fn map_to_some_element_flags(maybe_storage_flags: Option<&Self>) -> Option<ElementFlags> {
        maybe_storage_flags.map(|storage_flags| storage_flags.serialize())
    }

    /// Creates optional element flags
    pub fn to_some_element_flags(&self) -> Option<ElementFlags> {
        Some(self.serialize())
    }

    /// Creates element flags.
    pub fn to_element_flags(&self) -> ElementFlags {
        self.serialize()
    }

    /// split_storage_removed_bytes removes bytes as LIFO
    pub fn split_storage_removed_bytes(
        &self,
        removed_key_bytes: u32,
        removed_value_bytes: u32,
    ) -> Result<(StorageRemovedBytes, StorageRemovedBytes), grovedb::Error> {
        fn single_storage_removal(
            removed_bytes: u32,
            base_epoch: &BaseEpoch,
            owner_id: Option<&OwnerId>,
        ) -> StorageRemovedBytes {
            let bytes_left = removed_bytes;
            let mut sectioned_storage_removal: IntMap<u32> = IntMap::default();
            if bytes_left > 0 {
                // We need to take some from the base epoch
                sectioned_storage_removal.insert(*base_epoch as u64, removed_bytes);
            }
            let mut sectioned_storage_removal_by_identifier: StorageRemovalPerEpochByIdentifier =
                BTreeMap::new();
            if let Some(owner_id) = owner_id {
                sectioned_storage_removal_by_identifier
                    .insert(*owner_id, sectioned_storage_removal);
            } else {
                let default = [0u8; 32];
                sectioned_storage_removal_by_identifier.insert(default, sectioned_storage_removal);
            }
            SectionedStorageRemoval(sectioned_storage_removal_by_identifier)
        }

        fn sectioned_storage_removal(
            removed_bytes: u32,
            base_epoch: &BaseEpoch,
            other_epoch_bytes: &BTreeMap<EpochIndex, BytesAddedInEpoch>,
            owner_id: Option<&OwnerId>,
        ) -> StorageRemovedBytes {
            let mut bytes_left = removed_bytes;
            let mut rev_iter = other_epoch_bytes.iter().rev();
            let mut sectioned_storage_removal: IntMap<u32> = IntMap::default();
            while bytes_left > 0 {
                if let Some((epoch_index, bytes_in_epoch)) = rev_iter.next_back() {
                    if *bytes_in_epoch < bytes_left {
                        bytes_left -= bytes_in_epoch;
                        sectioned_storage_removal.insert(*epoch_index as u64, *bytes_in_epoch);
                    } else if *bytes_in_epoch >= bytes_left {
                        //take all bytes
                        bytes_left = 0;
                        sectioned_storage_removal.insert(*epoch_index as u64, bytes_left);
                    }
                } else {
                    break;
                }
            }
            if bytes_left > 0 {
                // We need to take some from the base epoch
                sectioned_storage_removal.insert(*base_epoch as u64, bytes_left);
            }
            let mut sectioned_storage_removal_by_identifier: StorageRemovalPerEpochByIdentifier =
                BTreeMap::new();
            if let Some(owner_id) = owner_id {
                sectioned_storage_removal_by_identifier
                    .insert(*owner_id, sectioned_storage_removal);
            } else {
                let default = [0u8; 32];
                sectioned_storage_removal_by_identifier.insert(default, sectioned_storage_removal);
            }
            SectionedStorageRemoval(sectioned_storage_removal_by_identifier)
        }
        match self {
            SingleEpoch(base_epoch) => {
                let value_storage_removal =
                    single_storage_removal(removed_value_bytes, base_epoch, None);
                let key_storage_removal =
                    single_storage_removal(removed_key_bytes, base_epoch, None);
                Ok((key_storage_removal, value_storage_removal))
            }
            SingleEpochOwned(base_epoch, owner_id) => {
                let value_storage_removal =
                    single_storage_removal(removed_value_bytes, base_epoch, Some(owner_id));
                let key_storage_removal =
                    single_storage_removal(removed_key_bytes, base_epoch, Some(owner_id));
                Ok((key_storage_removal, value_storage_removal))
            }
            MultiEpoch(base_epoch, other_epoch_bytes) => {
                let value_storage_removal = sectioned_storage_removal(
                    removed_value_bytes,
                    base_epoch,
                    other_epoch_bytes,
                    None,
                );
                let key_storage_removal = sectioned_storage_removal(
                    removed_key_bytes,
                    base_epoch,
                    other_epoch_bytes,
                    None,
                );
                Ok((key_storage_removal, value_storage_removal))
            }
            MultiEpochOwned(base_epoch, other_epoch_bytes, owner_id) => {
                let value_storage_removal = sectioned_storage_removal(
                    removed_value_bytes,
                    base_epoch,
                    other_epoch_bytes,
                    Some(owner_id),
                );
                let key_storage_removal = sectioned_storage_removal(
                    removed_key_bytes,
                    base_epoch,
                    other_epoch_bytes,
                    Some(owner_id),
                );
                Ok((key_storage_removal, value_storage_removal))
            }
        }
    }
}
