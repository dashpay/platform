//! FFI shape of the invitation-claim slot leases
//! ([`IdentitySlotLeaseChangeSet`]) and the helpers `persistence.rs` uses to
//! store and load them through the host's extension callbacks.

use std::collections::BTreeMap;
use std::os::raw::c_void;

use dashcore::{OutPoint, Txid};
use platform_wallet::changeset::{
    IdentitySlotLeaseChangeSet, IdentitySlotLeaseEntry, PersistenceError,
};
use platform_wallet::wallet::identity::SlotLeasePhase;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::asset_lock_persistence::outpoint_to_bytes;

/// `IdentitySlotLeaseFFI::phase`: keys may be derived, nothing was sent.
pub const IDENTITY_SLOT_LEASE_PHASE_RESERVED: u8 = 0;
/// `IdentitySlotLeaseFFI::phase`: a claim using the slot may have reached
/// Platform.
pub const IDENTITY_SLOT_LEASE_PHASE_SUBMITTED: u8 = 1;

/// One stored invitation-claim lease. Opaque to the host: store each field
/// and return it as it came, keyed by `invitation_out_point`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentitySlotLeaseFFI {
    /// The invitation's asset-lock outpoint: 32-byte raw txid, then the
    /// 4-byte little-endian vout.
    pub invitation_out_point: [u8; 36],
    /// The identity index the claim derives its keys at.
    pub identity_index: u32,
    /// [`IDENTITY_SLOT_LEASE_PHASE_RESERVED`] or
    /// [`IDENTITY_SLOT_LEASE_PHASE_SUBMITTED`].
    pub phase: u8,
}

/// Store a round's invitation-claim leases for `wallet_id`, inside the
/// round's begin/end bracket: upsert each of `leases` by its outpoint, delete
/// the lease of each 36-byte outpoint in `released`. A pointer is null when
/// its count is 0 and valid only for the duration of the call.
pub type PersistIdentitySlotLeasesFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    leases: *const IdentitySlotLeaseFFI,
    leases_count: usize,
    released: *const [u8; 36],
    released_count: usize,
) -> i32;

/// Return every stored invitation-claim lease of `wallet_id`. The host
/// allocates the array and keeps it valid until Rust hands it back through
/// [`FreeIdentitySlotLeasesFn`]; a null array with a zero count is an empty
/// one. An unreadable store must return an error, not an empty array.
pub type LoadIdentitySlotLeasesFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    out_leases: *mut *const IdentitySlotLeaseFFI,
    out_leases_count: *mut usize,
) -> i32;

/// Release the array a [`LoadIdentitySlotLeasesFn`] returned.
pub type FreeIdentitySlotLeasesFn = unsafe extern "C" fn(
    context: *mut c_void,
    leases: *const IdentitySlotLeaseFFI,
    leases_count: usize,
);

fn phase_to_u8(phase: SlotLeasePhase) -> u8 {
    match phase {
        SlotLeasePhase::Reserved => IDENTITY_SLOT_LEASE_PHASE_RESERVED,
        SlotLeasePhase::Submitted => IDENTITY_SLOT_LEASE_PHASE_SUBMITTED,
    }
}

fn phase_from_u8(phase: u8) -> Option<SlotLeasePhase> {
    match phase {
        IDENTITY_SLOT_LEASE_PHASE_RESERVED => Some(SlotLeasePhase::Reserved),
        IDENTITY_SLOT_LEASE_PHASE_SUBMITTED => Some(SlotLeasePhase::Submitted),
        _ => None,
    }
}

pub(crate) fn outpoint_from_bytes(bytes: &[u8; 36]) -> OutPoint {
    let mut txid = [0u8; 32];
    txid.copy_from_slice(&bytes[..32]);
    let mut vout = [0u8; 4];
    vout.copy_from_slice(&bytes[32..]);
    OutPoint::new(Txid::from_byte_array(txid), u32::from_le_bytes(vout))
}

/// Hand one round's leases to the host. Returns the callback's code, or 0
/// when the round carries none.
///
/// # Safety
/// `callback` and `context` are the host's, valid for the call.
pub(crate) unsafe fn persist_identity_slot_leases(
    callback: PersistIdentitySlotLeasesFn,
    context: *mut c_void,
    wallet_id: &WalletId,
    changeset: &IdentitySlotLeaseChangeSet,
) -> i32 {
    if changeset.leases.is_empty() && changeset.released.is_empty() {
        return 0;
    }
    let leases: Vec<IdentitySlotLeaseFFI> = changeset
        .leases
        .iter()
        .map(|(invitation, entry)| IdentitySlotLeaseFFI {
            invitation_out_point: outpoint_to_bytes(invitation),
            identity_index: entry.identity_index,
            phase: phase_to_u8(entry.phase),
        })
        .collect();
    let released: Vec<[u8; 36]> = changeset.released.iter().map(outpoint_to_bytes).collect();
    callback(
        context,
        wallet_id.as_ptr(),
        if leases.is_empty() {
            std::ptr::null()
        } else {
            leases.as_ptr()
        },
        leases.len(),
        if released.is_empty() {
            std::ptr::null()
        } else {
            released.as_ptr()
        },
        released.len(),
    )
}

/// Read `wallet_id`'s stored leases. A failed read, a missing buffer or a
/// phase this build does not know fails the load: a lease silently dropped
/// would let a registration take the slot a claim already used.
pub(crate) fn load_identity_slot_leases(
    load: LoadIdentitySlotLeasesFn,
    free: FreeIdentitySlotLeasesFn,
    context: *mut c_void,
    wallet_id: &WalletId,
) -> Result<BTreeMap<OutPoint, IdentitySlotLeaseEntry>, PersistenceError> {
    struct Guard {
        free: FreeIdentitySlotLeasesFn,
        context: *mut c_void,
        leases: *const IdentitySlotLeaseFFI,
        count: usize,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            if !self.leases.is_null() {
                unsafe { (self.free)(self.context, self.leases, self.count) };
            }
        }
    }

    let mut guard = Guard {
        free,
        context,
        leases: std::ptr::null(),
        count: 0,
    };
    let rc = unsafe {
        load(
            context,
            wallet_id.as_ptr(),
            &mut guard.leases,
            &mut guard.count,
        )
    };
    if rc != 0 {
        return Err(PersistenceError::backend(format!(
            "on_load_identity_slot_leases_fn returned error code {rc}"
        )));
    }
    if guard.count == 0 {
        return Ok(BTreeMap::new());
    }
    if guard.leases.is_null() {
        return Err(PersistenceError::backend(
            "identity slot leases: missing row buffer",
        ));
    }
    let rows = unsafe { std::slice::from_raw_parts(guard.leases, guard.count) };
    let mut leases = BTreeMap::new();
    for row in rows {
        let phase = phase_from_u8(row.phase).ok_or_else(|| {
            PersistenceError::backend(format!("identity slot lease: unknown phase {}", row.phase))
        })?;
        leases.insert(
            outpoint_from_bytes(&row.invitation_out_point),
            IdentitySlotLeaseEntry {
                identity_index: row.identity_index,
                phase,
            },
        );
    }
    Ok(leases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static STORED: RefCell<Vec<IdentitySlotLeaseFFI>> = const { RefCell::new(Vec::new()) };
        static RELEASED: RefCell<Vec<[u8; 36]>> = const { RefCell::new(Vec::new()) };
        static FREED: RefCell<usize> = const { RefCell::new(0) };
    }

    unsafe extern "C" fn persist(
        _context: *mut c_void,
        _wallet_id: *const u8,
        leases: *const IdentitySlotLeaseFFI,
        leases_count: usize,
        released: *const [u8; 36],
        released_count: usize,
    ) -> i32 {
        if leases_count > 0 {
            let rows = std::slice::from_raw_parts(leases, leases_count);
            STORED.with(|stored| stored.borrow_mut().extend_from_slice(rows));
        }
        if released_count > 0 {
            let rows = std::slice::from_raw_parts(released, released_count);
            RELEASED.with(|stored| stored.borrow_mut().extend_from_slice(rows));
        }
        0
    }

    unsafe extern "C" fn load(
        _context: *mut c_void,
        _wallet_id: *const u8,
        out_leases: *mut *const IdentitySlotLeaseFFI,
        out_leases_count: *mut usize,
    ) -> i32 {
        STORED.with(|stored| {
            let rows = stored.borrow().clone().into_boxed_slice();
            *out_leases_count = rows.len();
            *out_leases = Box::into_raw(rows) as *const IdentitySlotLeaseFFI;
        });
        0
    }

    unsafe extern "C" fn free(
        _context: *mut c_void,
        leases: *const IdentitySlotLeaseFFI,
        leases_count: usize,
    ) {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            leases as *mut IdentitySlotLeaseFFI,
            leases_count,
        )));
        FREED.with(|freed| *freed.borrow_mut() += 1);
    }

    unsafe extern "C" fn load_failing(
        _context: *mut c_void,
        _wallet_id: *const u8,
        _out_leases: *mut *const IdentitySlotLeaseFFI,
        _out_leases_count: *mut usize,
    ) -> i32 {
        7
    }

    fn invitation(byte: u8, vout: u32) -> OutPoint {
        OutPoint::new(Txid::from_byte_array([byte; 32]), vout)
    }

    #[test]
    fn should_round_trip_a_lease_through_the_host() {
        let mut changeset = IdentitySlotLeaseChangeSet::default();
        changeset.leases.insert(
            invitation(1, 3),
            IdentitySlotLeaseEntry {
                identity_index: 9,
                phase: SlotLeasePhase::Submitted,
            },
        );
        changeset.released.insert(invitation(2, 0));

        let rc = unsafe {
            persist_identity_slot_leases(persist, std::ptr::null_mut(), &[1; 32], &changeset)
        };
        let loaded = load_identity_slot_leases(load, free, std::ptr::null_mut(), &[1; 32]).unwrap();

        assert_eq!(rc, 0);
        assert_eq!(loaded, changeset.leases);
        assert_eq!(
            RELEASED.with(|released| released.borrow().clone()),
            vec![outpoint_to_bytes(&invitation(2, 0))]
        );
        assert_eq!(FREED.with(|freed| *freed.borrow()), 1);
    }

    #[test]
    fn should_fail_the_load_on_an_unknown_phase() {
        STORED.with(|stored| {
            stored.borrow_mut().push(IdentitySlotLeaseFFI {
                invitation_out_point: outpoint_to_bytes(&invitation(1, 0)),
                identity_index: 0,
                phase: 9,
            })
        });

        let loaded = load_identity_slot_leases(load, free, std::ptr::null_mut(), &[1; 32]);

        assert!(loaded.is_err());
        assert_eq!(FREED.with(|freed| *freed.borrow()), 1);
    }

    #[test]
    fn should_fail_the_load_when_the_host_cannot_read() {
        let loaded = load_identity_slot_leases(load_failing, free, std::ptr::null_mut(), &[1; 32]);

        assert!(loaded.is_err());
    }
}
