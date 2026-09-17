//! Serialization of storage flags.
//!
//! Types 0 to 3 are produced and parsed by the pinned
//! `grovedb_epoch_based_storage_flags` crate so that their bytes cannot
//! drift from what is on chain. Types 4 and 5 are the contract bucket
//! variants defined here.

use super::{
    BaseEpoch, BytesAddedInEpoch, ContractId, CrateStorageFlags, EpochIndex, StorageFlags,
    CONTRACT_BUCKET_OWNER_SIZE, OWNER_ID_SIZE, SINGLE_EPOCH_FLAGS_SIZE,
};
use dpp::fee::refund_owner::ContractCreditBucketPosition;
use grovedb::ElementFlags;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;
use integer_encoding::VarInt;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Type byte of the single epoch contract bucket variant
const SINGLE_EPOCH_CONTRACT_BUCKET_TYPE: u8 = 4;

/// Type byte of the multi epoch contract bucket variant
const MULTI_EPOCH_CONTRACT_BUCKET_TYPE: u8 = 5;

/// Size of the fixed header of both contract bucket variants: type byte,
/// contract id, position and base epoch
const CONTRACT_BUCKET_HEADER_SIZE: usize =
    (SINGLE_EPOCH_FLAGS_SIZE + CONTRACT_BUCKET_OWNER_SIZE) as usize;

/// Smallest possible epoch map entry: two epoch index bytes and one varint byte
const MIN_EPOCH_MAP_ENTRY_SIZE: usize = 3;

impl StorageFlags {
    /// Serialize storage flags
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            StorageFlags::SingleEpochContractBucket(base_epoch, contract_id, position) => {
                let mut buffer = Vec::with_capacity(CONTRACT_BUCKET_HEADER_SIZE);
                Self::append_contract_bucket_header(
                    &mut buffer,
                    SINGLE_EPOCH_CONTRACT_BUCKET_TYPE,
                    contract_id,
                    *position,
                    *base_epoch,
                );
                buffer
            }
            StorageFlags::MultiEpochContractBucket(base_epoch, epochs, contract_id, position) => {
                let mut buffer = Vec::with_capacity(
                    CONTRACT_BUCKET_HEADER_SIZE + epochs.len() * MIN_EPOCH_MAP_ENTRY_SIZE,
                );
                Self::append_contract_bucket_header(
                    &mut buffer,
                    MULTI_EPOCH_CONTRACT_BUCKET_TYPE,
                    contract_id,
                    *position,
                    *base_epoch,
                );
                Self::append_epoch_map(&mut buffer, epochs);
                buffer
            }
            // the two historical multi epoch variants write the crate's
            // layout without cloning their epoch map; equality with the
            // crate's bytes is pinned by test
            StorageFlags::MultiEpoch(base_epoch, epochs) => {
                let mut buffer = Vec::with_capacity(
                    SINGLE_EPOCH_FLAGS_SIZE as usize + epochs.len() * MIN_EPOCH_MAP_ENTRY_SIZE,
                );
                buffer.push(self.type_byte());
                buffer.extend_from_slice(&base_epoch.to_be_bytes());
                Self::append_epoch_map(&mut buffer, epochs);
                buffer
            }
            StorageFlags::MultiEpochOwned(base_epoch, epochs, owner_id) => {
                let mut buffer = Vec::with_capacity(
                    (SINGLE_EPOCH_FLAGS_SIZE + OWNER_ID_SIZE) as usize
                        + epochs.len() * MIN_EPOCH_MAP_ENTRY_SIZE,
                );
                buffer.push(self.type_byte());
                buffer.extend_from_slice(owner_id);
                buffer.extend_from_slice(&base_epoch.to_be_bytes());
                Self::append_epoch_map(&mut buffer, epochs);
                buffer
            }
            // the single epoch variants carry no map, so the crate value is
            // a plain copy
            StorageFlags::SingleEpoch(_) | StorageFlags::SingleEpochOwned(..) => {
                self.to_crate_flags_keyed_by_removal_key().serialize()
            }
        }
    }

    /// Serialized size of storage flags, equal to `serialize().len()`
    pub fn serialized_size(&self) -> u32 {
        match self {
            StorageFlags::SingleEpochContractBucket(..) => CONTRACT_BUCKET_HEADER_SIZE as u32,
            StorageFlags::MultiEpochContractBucket(_, epochs, ..) => {
                CONTRACT_BUCKET_HEADER_SIZE as u32 + Self::epoch_map_size(epochs)
            }
            StorageFlags::MultiEpoch(_, epochs) => {
                SINGLE_EPOCH_FLAGS_SIZE + Self::epoch_map_size(epochs)
            }
            StorageFlags::MultiEpochOwned(_, epochs, _) => {
                SINGLE_EPOCH_FLAGS_SIZE + OWNER_ID_SIZE + Self::epoch_map_size(epochs)
            }
            StorageFlags::SingleEpoch(_) => SINGLE_EPOCH_FLAGS_SIZE,
            StorageFlags::SingleEpochOwned(..) => SINGLE_EPOCH_FLAGS_SIZE + OWNER_ID_SIZE,
        }
    }

    fn append_contract_bucket_header(
        buffer: &mut Vec<u8>,
        type_byte: u8,
        contract_id: &ContractId,
        position: ContractCreditBucketPosition,
        base_epoch: BaseEpoch,
    ) {
        buffer.push(type_byte);
        buffer.extend_from_slice(contract_id);
        buffer.extend_from_slice(&position.to_be_bytes());
        buffer.extend_from_slice(&base_epoch.to_be_bytes());
    }

    fn append_epoch_map(buffer: &mut Vec<u8>, epochs: &BTreeMap<EpochIndex, BytesAddedInEpoch>) {
        epochs.iter().for_each(|(epoch_index, bytes_added)| {
            buffer.extend_from_slice(&epoch_index.to_be_bytes());
            buffer.extend(bytes_added.encode_var_vec());
        })
    }

    fn epoch_map_size(epochs: &BTreeMap<EpochIndex, BytesAddedInEpoch>) -> u32 {
        epochs
            .values()
            .map(|bytes_added| 2 + bytes_added.encode_var_vec().len() as u32)
            .sum()
    }

    fn deserialize_contract_bucket_header(
        data: &[u8],
    ) -> Result<(BaseEpoch, ContractId, ContractCreditBucketPosition), StorageFlagsError> {
        let contract_id: ContractId = data
            .get(1..33)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| {
                StorageFlagsError::StorageFlagsWrongSize(
                    "contract bucket flags must have 32 bytes of contract id".to_string(),
                )
            })?;
        let position = data
            .get(33..35)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_be_bytes)
            .ok_or_else(|| {
                StorageFlagsError::StorageFlagsWrongSize(
                    "contract bucket flags must have 2 bytes of bucket position".to_string(),
                )
            })?;
        let base_epoch = data
            .get(35..37)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_be_bytes)
            .ok_or_else(|| {
                StorageFlagsError::StorageFlagsWrongSize(
                    "contract bucket flags must have 2 bytes of base epoch".to_string(),
                )
            })?;
        Ok((base_epoch, contract_id, position))
    }

    /// Deserialize single epoch contract bucket storage flags from bytes
    fn deserialize_single_epoch_contract_bucket(data: &[u8]) -> Result<Self, StorageFlagsError> {
        if data.len() != CONTRACT_BUCKET_HEADER_SIZE {
            return Err(StorageFlagsError::StorageFlagsWrongSize(
                "single epoch contract bucket must be 37 bytes total".to_string(),
            ));
        }
        let (base_epoch, contract_id, position) = Self::deserialize_contract_bucket_header(data)?;
        Ok(StorageFlags::SingleEpochContractBucket(
            base_epoch,
            contract_id,
            position,
        ))
    }

    /// Deserialize multi epoch contract bucket storage flags from bytes.
    ///
    /// Unlike the crate's multi epoch decoders this one is strict: the epoch
    /// map must hold at least one entry and the bytes must end exactly at
    /// the end of the last entry.
    fn deserialize_multi_epoch_contract_bucket(data: &[u8]) -> Result<Self, StorageFlagsError> {
        let len = data.len();
        if len < CONTRACT_BUCKET_HEADER_SIZE + MIN_EPOCH_MAP_ENTRY_SIZE {
            return Err(StorageFlagsError::StorageFlagsWrongSize(
                "multi epoch contract bucket must be at least 40 bytes total".to_string(),
            ));
        }
        let (base_epoch, contract_id, position) = Self::deserialize_contract_bucket_header(data)?;
        let mut offset = CONTRACT_BUCKET_HEADER_SIZE;
        let mut bytes_per_epoch: BTreeMap<EpochIndex, BytesAddedInEpoch> = BTreeMap::default();
        while offset < len {
            let epoch_index = data
                .get(offset..offset + 2)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u16::from_be_bytes)
                .ok_or_else(|| {
                    StorageFlagsError::StorageFlagsWrongSize(
                        "multi epoch contract bucket must have enough bytes for epoch indexes"
                            .to_string(),
                    )
                })?;
            offset += 2;
            let (bytes_at_epoch, bytes_used) = data
                .get(offset..)
                .and_then(u32::decode_var)
                .ok_or_else(|| {
                    StorageFlagsError::StorageFlagsWrongSize(
                        "multi epoch contract bucket must have enough bytes for the amount of bytes used"
                            .to_string(),
                    )
                })?;
            offset += bytes_used;
            bytes_per_epoch.insert(epoch_index, bytes_at_epoch);
        }
        if bytes_per_epoch.is_empty() {
            return Err(StorageFlagsError::StorageFlagsWrongSize(
                "multi epoch contract bucket must carry at least one epoch entry".to_string(),
            ));
        }
        Ok(StorageFlags::MultiEpochContractBucket(
            base_epoch,
            bytes_per_epoch,
            contract_id,
            position,
        ))
    }

    /// Deserialize storage flags from bytes.
    ///
    /// Decodes all six variants: the four historical ones through the crate,
    /// the two contract bucket ones here. An unknown type byte is an error.
    pub fn deserialize(data: &[u8]) -> Result<Option<Self>, StorageFlagsError> {
        match data.first() {
            None => Ok(None),
            Some(&SINGLE_EPOCH_CONTRACT_BUCKET_TYPE) => {
                Ok(Some(Self::deserialize_single_epoch_contract_bucket(data)?))
            }
            Some(&MULTI_EPOCH_CONTRACT_BUCKET_TYPE) => {
                Ok(Some(Self::deserialize_multi_epoch_contract_bucket(data)?))
            }
            Some(_) => Ok(CrateStorageFlags::deserialize(data)?.map(Self::from)),
        }
    }

    /// Creates storage flags from a slice.
    pub fn from_slice(data: &[u8]) -> Result<Option<Self>, StorageFlagsError> {
        Self::deserialize(data)
    }

    /// Creates storage flags from element flags.
    pub fn from_element_flags_ref(data: &ElementFlags) -> Result<Option<Self>, StorageFlagsError> {
        Self::from_slice(data.as_slice())
    }

    /// Create Storage flags from optional element flags ref
    pub fn map_some_element_flags_ref(
        data: &Option<ElementFlags>,
    ) -> Result<Option<Self>, StorageFlagsError> {
        match data {
            None => Ok(None),
            Some(data) => Self::from_slice(data.as_slice()),
        }
    }

    /// Create Storage flags from optional element flags ref
    pub fn map_cow_some_element_flags_ref(
        data: &Option<ElementFlags>,
    ) -> Result<Option<Cow<'_, Self>>, StorageFlagsError> {
        match data {
            None => Ok(None),
            Some(data) => Self::from_slice(data.as_slice()).map(|option| option.map(Cow::Owned)),
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

    /// Map to optional element flags
    pub fn map_cow_to_some_element_flags(
        maybe_storage_flags: Option<Cow<Self>>,
    ) -> Option<ElementFlags> {
        maybe_storage_flags.map(|storage_flags| storage_flags.serialize())
    }

    /// Map to optional element flags
    pub fn map_borrowed_cow_to_some_element_flags(
        maybe_storage_flags: &Option<Cow<Self>>,
    ) -> Option<ElementFlags> {
        maybe_storage_flags
            .as_ref()
            .map(|storage_flags| storage_flags.serialize())
    }

    /// Creates optional element flags
    pub fn to_some_element_flags(&self) -> Option<ElementFlags> {
        Some(self.serialize())
    }

    /// Creates element flags.
    pub fn to_element_flags(&self) -> ElementFlags {
        self.serialize()
    }
}
