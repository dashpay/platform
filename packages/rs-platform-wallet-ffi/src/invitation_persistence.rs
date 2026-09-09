//! FFI types for forwarding
//! [`InvitationChangeSet`](platform_wallet::changeset::InvitationChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! Mirrors the shape of `asset_lock_persistence`, but every field of an
//! [`InvitationEntry`] is plain-old-data (no transaction / proof buffers), so
//! unlike [`AssetLockEntryFFI`](crate::asset_lock_persistence::AssetLockEntryFFI)
//! there is **no** parallel storage `Vec` to keep alive and **no**
//! `unsafe impl Send/Sync` — the struct is fully self-contained. Swift maps each
//! upsert onto a `PersistentInvitation` row keyed by the outpoint and deletes
//! rows for each removed outpoint.

use platform_wallet::changeset::{InvitationEntry, InvitationStatus};

/// Flat, all-POD C mirror of one [`InvitationEntry`].
///
/// Field order places `amount_duffs` (u64) on an 8-byte boundary
/// (`32 + 4 = 36`, then `funding_index` at 36 lands the u64 at 40), so the
/// struct has no internal padding. Do not reorder without re-checking padding
/// on both sides.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InvitationEntryFFI {
    /// Outpoint of the funded voucher: 32-byte raw txid followed by 4-byte
    /// little-endian vout. Same encoding as `AssetLockEntryFFI.out_point`.
    pub out_point: [u8; 36],
    /// DIP-13 funding index the voucher key is derived from (`…/3'/idx'`).
    pub funding_index: u32,
    /// Voucher amount in duffs (1 DASH = 1e8 duffs).
    pub amount_duffs: u64,
    /// Advisory expiry (unix seconds).
    pub expiry_unix: u32,
    /// Creation time (unix seconds).
    pub created_at_secs: u32,
    /// `1` if the link carries inviter info (contact-bootstrap), else `0`.
    /// A `u8` — not a `bool` — so a foreign byte value can never be UB.
    pub has_inviter: u8,
    /// Discriminant of [`InvitationStatus`]:
    /// 0 = Created, 1 = Claimed, 2 = Reclaimed.
    pub status: u8,
}

// Pin the ABI size so a future field reorder/add that changes the layout is a
// compile error rather than a silent desync against the Swift-imported struct
// (matches the layout-assert convention used for every other `*EntryFFI`).
// `[u8;36]`@0, u32@36, u64@40, u32@48, u32@52, u8@56, u8@57 → data ends @58,
// struct align 8 → size 64.
const _: [u8; 64] = [0u8; std::mem::size_of::<InvitationEntryFFI>()];

/// Build the flat FFI entries from the changeset entries.
///
/// All-POD, so — unlike `build_asset_lock_entries` — there is no parallel
/// storage `Vec` and nothing to keep alive beyond the returned `Vec` itself
/// (which the callback dispatcher holds for the FFI window).
pub fn build_invitation_entries(entries: &[&InvitationEntry]) -> Vec<InvitationEntryFFI> {
    entries
        .iter()
        .map(|entry| InvitationEntryFFI {
            out_point: crate::asset_lock_persistence::outpoint_to_bytes(&entry.out_point),
            funding_index: entry.funding_index,
            amount_duffs: entry.amount_duffs,
            expiry_unix: entry.expiry_unix,
            created_at_secs: entry.created_at_secs,
            has_inviter: u8::from(entry.has_inviter),
            status: status_to_u8(&entry.status),
        })
        .collect()
}

/// Discriminant mapping for [`InvitationStatus`]. Wildcard-free so adding a
/// variant is a compile error rather than a silent mis-map. Pinned by a test.
pub fn status_to_u8(status: &InvitationStatus) -> u8 {
    match status {
        InvitationStatus::Created => 0,
        InvitationStatus::Claimed => 1,
        InvitationStatus::Reclaimed => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;

    fn entry(vout: u32, status: InvitationStatus) -> InvitationEntry {
        InvitationEntry {
            out_point: dashcore::OutPoint::new(
                dashcore::Txid::from_byte_array([vout as u8; 32]),
                vout,
            ),
            funding_index: vout,
            amount_duffs: 500_000 + u64::from(vout),
            expiry_unix: 1_800_000_000,
            created_at_secs: 1_799_913_600,
            has_inviter: vout.is_multiple_of(2),
            status,
        }
    }

    #[test]
    fn status_to_u8_pins_discriminants() {
        assert_eq!(status_to_u8(&InvitationStatus::Created), 0);
        assert_eq!(status_to_u8(&InvitationStatus::Claimed), 1);
        assert_eq!(status_to_u8(&InvitationStatus::Reclaimed), 2);
    }

    #[test]
    fn build_invitation_entries_round_trips_every_field() {
        let e0 = entry(0, InvitationStatus::Created);
        let e1 = entry(1, InvitationStatus::Reclaimed);
        let refs = [&e0, &e1];
        let ffi = build_invitation_entries(&refs);

        assert_eq!(ffi.len(), 2);
        // e0
        assert_eq!(
            ffi[0].out_point,
            crate::asset_lock_persistence::outpoint_to_bytes(&e0.out_point)
        );
        assert_eq!(ffi[0].funding_index, 0);
        assert_eq!(ffi[0].amount_duffs, 500_000);
        assert_eq!(ffi[0].expiry_unix, 1_800_000_000);
        assert_eq!(ffi[0].created_at_secs, 1_799_913_600);
        assert_eq!(ffi[0].has_inviter, 1); // vout 0 is even
        assert_eq!(ffi[0].status, 0);
        // e1
        assert_eq!(ffi[1].funding_index, 1);
        assert_eq!(ffi[1].amount_duffs, 500_001);
        assert_eq!(ffi[1].has_inviter, 0); // vout 1 is odd
        assert_eq!(ffi[1].status, 2);
    }
}
