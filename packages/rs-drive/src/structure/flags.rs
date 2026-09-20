use crate::util::storage_flags::StorageFlags;
use grovedb::Element;
use serde::Serialize;

/// Which element flags sit on an element.
///
/// Every GroveDB element can carry a byte string of flags that GroveDB stores
/// and never reads. Drive uses it for [`StorageFlags`]: who paid for the
/// element's bytes and in which epoch, which is what a refund is computed from
/// when the element is deleted or shrinks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum FlagsKind {
    /// No flags. Removing the element refunds nothing.
    None,
    /// Storage flags without an owner: the epoch the bytes were paid in, and
    /// for an element that grew later, the bytes added in each later epoch.
    Epoch,
    /// Storage flags with an owner: the same, plus the identity a refund goes
    /// to.
    EpochOwned,
    /// Flags that are not storage flags
    Other,
}

impl FlagsKind {
    /// Every kind, in declaration order
    pub const ALL: [FlagsKind; 4] = [
        FlagsKind::None,
        FlagsKind::Epoch,
        FlagsKind::EpochOwned,
        FlagsKind::Other,
    ];

    /// The kind of flags on an element. Looks through the `NonCounted`,
    /// `NotSummed` and `NotCountedOrSummed` wrappers.
    pub fn of(element: &Element) -> FlagsKind {
        let Some(flags) = element.underlying().get_flags() else {
            return FlagsKind::None;
        };
        match StorageFlags::from_element_flags_ref(flags) {
            Ok(None) => FlagsKind::None,
            Ok(Some(StorageFlags::SingleEpoch(..) | StorageFlags::MultiEpoch(..))) => {
                FlagsKind::Epoch
            }
            Ok(Some(StorageFlags::SingleEpochOwned(..) | StorageFlags::MultiEpochOwned(..))) => {
                FlagsKind::EpochOwned
            }
            Err(_) => FlagsKind::Other,
        }
    }

    /// Whether a node's flags say only that the element carries none
    pub fn is_none_only(flags: &[FlagsKind]) -> bool {
        flags == [FlagsKind::None]
    }

    /// What the flags mean
    pub fn meaning(&self) -> &'static str {
        match self {
            FlagsKind::None => {
                "No flags. Nobody is recorded as having paid for the element, so removing it \
                 refunds nothing."
            }
            FlagsKind::Epoch => {
                "Storage flags without an owner. They record the epoch in which the element's \
                 bytes were paid for. If the element grew later, they also record how many \
                 bytes were added in each later epoch."
            }
            FlagsKind::EpochOwned => {
                "Storage flags with an owner. They record the epoch in which the element's \
                 bytes were paid for, the bytes added in each later epoch if it grew, and the \
                 identity that paid. When the element is deleted or shrinks, the unused part \
                 of the storage fee is refunded to that identity, epoch by epoch."
            }
            FlagsKind::Other => "Flags that are not Drive's storage flags.",
        }
    }

    /// How the flags are laid out in bytes
    pub fn layout(&self) -> &'static str {
        match self {
            FlagsKind::None => "absent",
            FlagsKind::Epoch => {
                "type byte 0, base epoch u16 BE; or type byte 1, base epoch u16 BE, then for \
                 each later epoch: epoch u16 BE and bytes added as a varint"
            }
            FlagsKind::EpochOwned => {
                "type byte 2, owner id 32 bytes, base epoch u16 BE; or type byte 3, owner id \
                 32 bytes, base epoch u16 BE, then for each later epoch: epoch u16 BE and bytes \
                 added as a varint"
            }
            FlagsKind::Other => "unknown",
        }
    }
}
