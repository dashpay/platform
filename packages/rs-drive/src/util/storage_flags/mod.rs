//! Storage flags
//!
//! Element flags are the opaque bytes GroveDB stores next to every element.
//! Drive uses them to remember which epoch paid for the bytes and who paid,
//! so that a later removal can refund the right owner at the right rate.
//!
//! GroveDB never interprets these bytes; only the closures Drive passes to
//! the batch apply and the readers in this crate do. That is why the codec
//! lives here. The four historical variants keep the exact encoding of the
//! `grovedb_epoch_based_storage_flags` crate: their bytes are produced and
//! parsed by that crate, so they cannot drift. Two variants are added for
//! bytes paid for by a contract credit bucket. The type byte declares the
//! owner's kind; it is never inferred from the shape of an identifier.
//!
//! The parse helpers (`deserialize`, `from_element_flags_ref`, the `map_*`
//! helpers) decode all six variants so that readers carry bucket-owned flags
//! through unchanged. The batch apply generations that predate typed owners
//! do not use this type at all: their closures are bound to the crate's
//! flags type, which rejects the two bucket variants, so they fail closed on
//! flags they were never written to price. The typed closure entry points
//! here (`update_element_flags_typed`, `split_removal_bytes_typed`) accept
//! all six variants and record the owner of every sectioned removal.

mod codec;
mod combine;
mod split;
mod update;

use dpp::fee::refund_owner::{ContractCreditBucketPosition, RefundOwner};
use dpp::identifier::Identifier;
use grovedb_epoch_based_storage_flags::StorageFlags as CrateStorageFlags;
pub use grovedb_epoch_based_storage_flags::{
    MergingOwnersStrategy, MINIMUM_NON_BASE_FLAGS_SIZE, SINGLE_EPOCH_FLAGS_SIZE,
};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use self::StorageFlags::{
    MultiEpoch, MultiEpochContractBucket, MultiEpochOwned, SingleEpoch, SingleEpochContractBucket,
    SingleEpochOwned,
};

/// An epoch index as stored in element flags
pub type EpochIndex = u16;

/// The epoch in which an element was first stored
pub type BaseEpoch = EpochIndex;

/// Bytes added to an element in a later epoch
pub type BytesAddedInEpoch = u32;

/// The identity that owns stored bytes, as 32 raw bytes
pub type OwnerId = [u8; 32];

/// The contract whose credit bucket owns stored bytes, as 32 raw bytes
pub type ContractId = [u8; 32];

/// Size of the owner id carried by the identity-owned variants
const OWNER_ID_SIZE: u32 = 32;

/// Size of the contract bucket owner: contract id plus two-byte position
const CONTRACT_BUCKET_OWNER_SIZE: u32 = 34;

/// Storage flags: the epoch that paid for an element's bytes and, when
/// someone other than the system paid, the recorded owner.
///
/// Variants 0 to 3 are encoded and decoded by the pinned
/// `grovedb_epoch_based_storage_flags` crate and are byte for byte what is on
/// chain today. Variants 4 and 5 carry a contract credit bucket owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageFlags {
    /// Single epoch, unowned
    /// represented as byte 0
    SingleEpoch(BaseEpoch),

    /// Multi epoch, unowned
    /// represented as byte 1
    MultiEpoch(BaseEpoch, BTreeMap<EpochIndex, BytesAddedInEpoch>),

    /// Single epoch owned by an identity
    /// represented as byte 2
    SingleEpochOwned(BaseEpoch, OwnerId),

    /// Multi epoch owned by an identity
    /// represented as byte 3
    MultiEpochOwned(BaseEpoch, BTreeMap<EpochIndex, BytesAddedInEpoch>, OwnerId),

    /// Single epoch owned by a contract credit bucket
    /// represented as byte 4
    ///
    /// Layout: type byte, contract id (32 bytes), bucket position (2 bytes,
    /// big-endian), base epoch (2 bytes, big-endian); 37 bytes in total.
    ///
    /// Provisional: the type byte allocation is proposed in the fees
    /// workstream register (issue 4689) and pending the owner's confirmation.
    SingleEpochContractBucket(BaseEpoch, ContractId, ContractCreditBucketPosition),

    /// Multi epoch owned by a contract credit bucket
    /// represented as byte 5
    ///
    /// Layout: the 37-byte header of the single epoch bucket variant followed
    /// by the crate's epoch map encoding (2-byte epoch index, varint bytes).
    MultiEpochContractBucket(
        BaseEpoch,
        BTreeMap<EpochIndex, BytesAddedInEpoch>,
        ContractId,
        ContractCreditBucketPosition,
    ),
}

impl fmt::Display for StorageFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleEpoch(base_epoch) => {
                write!(f, "SingleEpoch(BaseEpoch: {})", base_epoch)
            }
            MultiEpoch(base_epoch, epochs) => {
                write!(f, "MultiEpoch(BaseEpoch: {}, Epochs: ", base_epoch)?;
                for (index, bytes) in epochs {
                    write!(f, "[EpochIndex: {}, BytesAdded: {}] ", index, bytes)?;
                }
                write!(f, ")")
            }
            SingleEpochOwned(base_epoch, owner_id) => {
                write!(
                    f,
                    "SingleEpochOwned(BaseEpoch: {}, OwnerId: {})",
                    base_epoch,
                    hex::encode(owner_id)
                )
            }
            MultiEpochOwned(base_epoch, epochs, owner_id) => {
                write!(f, "MultiEpochOwned(BaseEpoch: {}, Epochs: ", base_epoch)?;
                for (index, bytes) in epochs {
                    write!(f, "[EpochIndex: {}, BytesAdded: {}] ", index, bytes)?;
                }
                write!(f, ", OwnerId: {})", hex::encode(owner_id))
            }
            SingleEpochContractBucket(base_epoch, contract_id, position) => {
                write!(
                    f,
                    "SingleEpochContractBucket(BaseEpoch: {}, ContractId: {}, Position: {})",
                    base_epoch,
                    hex::encode(contract_id),
                    position
                )
            }
            MultiEpochContractBucket(base_epoch, epochs, contract_id, position) => {
                write!(
                    f,
                    "MultiEpochContractBucket(BaseEpoch: {}, Epochs: ",
                    base_epoch
                )?;
                for (index, bytes) in epochs {
                    write!(f, "[EpochIndex: {}, BytesAdded: {}] ", index, bytes)?;
                }
                write!(
                    f,
                    ", ContractId: {}, Position: {})",
                    hex::encode(contract_id),
                    position
                )
            }
        }
    }
}

impl From<CrateStorageFlags> for StorageFlags {
    fn from(value: CrateStorageFlags) -> Self {
        match value {
            CrateStorageFlags::SingleEpoch(base_epoch) => SingleEpoch(base_epoch),
            CrateStorageFlags::MultiEpoch(base_epoch, epochs) => MultiEpoch(base_epoch, epochs),
            CrateStorageFlags::SingleEpochOwned(base_epoch, owner_id) => {
                SingleEpochOwned(base_epoch, owner_id)
            }
            CrateStorageFlags::MultiEpochOwned(base_epoch, epochs, owner_id) => {
                MultiEpochOwned(base_epoch, epochs, owner_id)
            }
        }
    }
}

impl StorageFlags {
    /// Create new single epoch storage flags, owned by an identity when an
    /// owner id is given
    pub fn new_single_epoch(epoch: BaseEpoch, maybe_owner_id: Option<OwnerId>) -> Self {
        match maybe_owner_id {
            None => SingleEpoch(epoch),
            Some(owner_id) => SingleEpochOwned(epoch, owner_id),
        }
    }

    /// Create new single epoch storage flags for a typed owner, or unowned
    /// flags when no owner is given
    pub fn new_single_epoch_for_owner(epoch: BaseEpoch, owner: Option<RefundOwner>) -> Self {
        match owner {
            None => SingleEpoch(epoch),
            Some(RefundOwner::Identity(identity_id)) => {
                SingleEpochOwned(epoch, identity_id.to_buffer())
            }
            Some(RefundOwner::ContractBucket {
                contract_id,
                position,
            }) => SingleEpochContractBucket(epoch, contract_id.to_buffer(), position),
        }
    }

    /// Sets the owner id if we have identity-owned storage flags
    pub fn set_owner_id(&mut self, owner_id: OwnerId) {
        match self {
            SingleEpochOwned(_, previous_owner_id) | MultiEpochOwned(_, _, previous_owner_id) => {
                *previous_owner_id = owner_id;
            }
            _ => {}
        }
    }

    /// Returns base epoch
    pub fn base_epoch(&self) -> &BaseEpoch {
        match self {
            SingleEpoch(base_epoch)
            | MultiEpoch(base_epoch, _)
            | SingleEpochOwned(base_epoch, _)
            | MultiEpochOwned(base_epoch, ..)
            | SingleEpochContractBucket(base_epoch, ..)
            | MultiEpochContractBucket(base_epoch, ..) => base_epoch,
        }
    }

    /// Returns the owner id when an identity owns the bytes.
    ///
    /// Contract bucket owners are not identities and return `None` here; use
    /// [`Self::refund_owner`] for the typed owner of any variant.
    pub fn owner_id(&self) -> Option<&OwnerId> {
        match self {
            SingleEpochOwned(_, owner_id) | MultiEpochOwned(_, _, owner_id) => Some(owner_id),
            _ => None,
        }
    }

    /// Returns the typed owner of the bytes, if anyone other than the system
    /// paid for them
    pub fn refund_owner(&self) -> Option<RefundOwner> {
        match self {
            SingleEpoch(_) | MultiEpoch(..) => None,
            SingleEpochOwned(_, owner_id) | MultiEpochOwned(_, _, owner_id) => {
                Some(RefundOwner::Identity(Identifier::from(*owner_id)))
            }
            SingleEpochContractBucket(_, contract_id, position)
            | MultiEpochContractBucket(_, _, contract_id, position) => {
                Some(RefundOwner::ContractBucket {
                    contract_id: Identifier::from(*contract_id),
                    position: *position,
                })
            }
        }
    }

    /// Returns epoch index map
    pub fn epoch_index_map(&self) -> Option<&BTreeMap<EpochIndex, BytesAddedInEpoch>> {
        match self {
            MultiEpoch(_, epoch_int_map)
            | MultiEpochOwned(_, epoch_int_map, _)
            | MultiEpochContractBucket(_, epoch_int_map, ..) => Some(epoch_int_map),
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

    /// Returns default optional storage flag as ref
    pub fn optional_default_as_cow() -> Option<Cow<'static, Self>> {
        None
    }

    /// Returns type byte
    pub fn type_byte(&self) -> u8 {
        match self {
            SingleEpoch(_) => 0,
            MultiEpoch(..) => 1,
            SingleEpochOwned(..) => 2,
            MultiEpochOwned(..) => 3,
            SingleEpochContractBucket(..) => 4,
            MultiEpochContractBucket(..) => 5,
        }
    }

    /// Approximate serialized size of flags with or without an identity owner
    pub fn approximate_size(
        has_owner_id: bool,
        approximate_changes_and_bytes_count: Option<(u16, u8)>,
    ) -> u32 {
        CrateStorageFlags::approximate_size(has_owner_id, approximate_changes_and_bytes_count)
    }

    /// Approximate serialized size of flags for a typed owner: 3 bytes for
    /// the type byte and base epoch, plus 32 for an identity owner or 34 for
    /// a contract bucket owner, plus the approximate epoch map size
    pub fn approximate_size_for_owner(
        owner: Option<&RefundOwner>,
        approximate_changes_and_bytes_count: Option<(u16, u8)>,
    ) -> u32 {
        let mut size = SINGLE_EPOCH_FLAGS_SIZE;
        match owner {
            None => {}
            Some(RefundOwner::Identity(_)) => size += OWNER_ID_SIZE,
            Some(RefundOwner::ContractBucket { .. }) => size += CONTRACT_BUCKET_OWNER_SIZE,
        }
        if let Some((approximate_change_count, bytes_changed_required_size)) =
            approximate_changes_and_bytes_count
        {
            size += (approximate_change_count as u32) * (2 + bytes_changed_required_size as u32)
        }
        size
    }

    /// Wrap Storage Flags into optional owned cow
    pub fn into_optional_cow<'a>(self) -> Option<Cow<'a, Self>> {
        Some(Cow::Owned(self))
    }

    /// Builds the flags for a base epoch, an optional epoch map and an
    /// optional typed owner. An empty epoch map gives a single epoch variant.
    pub(super) fn from_parts(
        base_epoch: BaseEpoch,
        epochs: Option<BTreeMap<EpochIndex, BytesAddedInEpoch>>,
        owner: Option<RefundOwner>,
    ) -> Self {
        let epochs = epochs.filter(|epochs| !epochs.is_empty());
        match (owner, epochs) {
            (None, None) => SingleEpoch(base_epoch),
            (None, Some(epochs)) => MultiEpoch(base_epoch, epochs),
            (Some(RefundOwner::Identity(identity_id)), None) => {
                SingleEpochOwned(base_epoch, identity_id.to_buffer())
            }
            (Some(RefundOwner::Identity(identity_id)), Some(epochs)) => {
                MultiEpochOwned(base_epoch, epochs, identity_id.to_buffer())
            }
            (
                Some(RefundOwner::ContractBucket {
                    contract_id,
                    position,
                }),
                None,
            ) => SingleEpochContractBucket(base_epoch, contract_id.to_buffer(), position),
            (
                Some(RefundOwner::ContractBucket {
                    contract_id,
                    position,
                }),
                Some(epochs),
            ) => MultiEpochContractBucket(base_epoch, epochs, contract_id.to_buffer(), position),
        }
    }

    /// Splits the flags into their base epoch, epoch map and typed owner,
    /// moving the map out.
    pub(super) fn into_parts(
        self,
    ) -> (
        BaseEpoch,
        Option<BTreeMap<EpochIndex, BytesAddedInEpoch>>,
        Option<RefundOwner>,
    ) {
        let owner = self.refund_owner();
        match self {
            SingleEpoch(base_epoch)
            | SingleEpochOwned(base_epoch, _)
            | SingleEpochContractBucket(base_epoch, ..) => (base_epoch, None, owner),
            MultiEpoch(base_epoch, epochs)
            | MultiEpochOwned(base_epoch, epochs, _)
            | MultiEpochContractBucket(base_epoch, epochs, ..) => (base_epoch, Some(epochs), owner),
        }
    }

    /// The crate's value with every typed owner replaced by its removal key,
    /// moving the epoch map instead of cloning it. See
    /// [`Self::to_crate_flags_keyed_by_removal_key`] for why this is sound.
    pub(super) fn into_crate_flags_keyed_by_removal_key(self) -> CrateStorageFlags {
        let (base_epoch, epochs, owner) = self.into_parts();
        let key = owner.map(|owner| owner.removal_key());
        match (key, epochs) {
            (None, None) => CrateStorageFlags::SingleEpoch(base_epoch),
            (None, Some(epochs)) => CrateStorageFlags::MultiEpoch(base_epoch, epochs),
            (Some(key), None) => CrateStorageFlags::SingleEpochOwned(base_epoch, key),
            (Some(key), Some(epochs)) => {
                CrateStorageFlags::MultiEpochOwned(base_epoch, epochs, key)
            }
        }
    }

    /// The crate's value with every typed owner replaced by its removal key.
    ///
    /// This is what lets the crate's epoch arithmetic (split and combine) run
    /// unchanged over the bucket variants: the crate only ever compares owner
    /// ids for equality and sections removed bytes under them, and the
    /// removal key is exactly the key those bytes must be sectioned under.
    /// Callers map the result back through [`Self::refund_owner`] of the
    /// inputs so the kind is never read from the key.
    pub(super) fn to_crate_flags_keyed_by_removal_key(&self) -> CrateStorageFlags {
        match self {
            SingleEpoch(base_epoch) => CrateStorageFlags::SingleEpoch(*base_epoch),
            MultiEpoch(base_epoch, epochs) => {
                CrateStorageFlags::MultiEpoch(*base_epoch, epochs.clone())
            }
            SingleEpochOwned(base_epoch, owner_id) => {
                CrateStorageFlags::SingleEpochOwned(*base_epoch, *owner_id)
            }
            MultiEpochOwned(base_epoch, epochs, owner_id) => {
                CrateStorageFlags::MultiEpochOwned(*base_epoch, epochs.clone(), *owner_id)
            }
            SingleEpochContractBucket(base_epoch, contract_id, position) => {
                CrateStorageFlags::SingleEpochOwned(
                    *base_epoch,
                    bucket_removal_key(contract_id, *position),
                )
            }
            MultiEpochContractBucket(base_epoch, epochs, contract_id, position) => {
                CrateStorageFlags::MultiEpochOwned(
                    *base_epoch,
                    epochs.clone(),
                    bucket_removal_key(contract_id, *position),
                )
            }
        }
    }
}

/// The carrier key of a contract bucket owner
fn bucket_removal_key(
    contract_id: &ContractId,
    position: ContractCreditBucketPosition,
) -> [u8; 32] {
    RefundOwner::ContractBucket {
        contract_id: Identifier::from(*contract_id),
        position,
    }
    .removal_key()
}

#[cfg(test)]
mod tests;
