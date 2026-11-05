//! Revision-aware asset-lock persistence without changing legacy row layouts.

use platform_wallet::changeset::{AssetLockEntry, AssetLockPersistenceAction, PersistenceError};
use platform_wallet::wallet::asset_lock::tracked::MAX_ASSET_LOCK_OPERATION_REVISION;

use crate::asset_lock_persistence::AssetLockEntryFFI;
use crate::persistence::{build_unused_asset_locks, status_from_u8};
use crate::wallet_restore_types::WalletRestoreEntryFFI;

/// Borrowed V1 snapshot. All embedded byte slices live only for the callback.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AssetLockOperationEntryFFI {
    pub entry: AssetLockEntryFFI,
    pub has_operation_id: bool,
    pub operation_id: [u8; 16],
    pub operation_revision: u64,
    pub has_expected_revision: bool,
    pub expected_revision: u64,
}

impl AssetLockOperationEntryFFI {
    pub(crate) fn new(entry: &AssetLockEntry, ffi: AssetLockEntryFFI) -> Self {
        Self {
            entry: ffi,
            has_operation_id: entry.operation_id.is_some(),
            operation_id: entry.operation_id.unwrap_or_default(),
            operation_revision: entry.operation_revision,
            has_expected_revision: entry.expected_operation_revision.is_some(),
            expected_revision: entry.expected_operation_revision.unwrap_or_default(),
        }
    }

    unsafe fn decode(&self) -> Result<AssetLockEntry, PersistenceError> {
        if self.entry.proof_bytes.is_null() && self.entry.proof_bytes_len != 0 {
            return Err(PersistenceError::backend("asset-lock proof buffer missing"));
        }
        if self.operation_revision > MAX_ASSET_LOCK_OPERATION_REVISION
            || (self.has_expected_revision
                && self.expected_revision > MAX_ASSET_LOCK_OPERATION_REVISION)
        {
            return Err(PersistenceError::backend(
                "asset-lock operation revision out of range",
            ));
        }
        let restore = WalletRestoreEntryFFI {
            tracked_asset_locks: &self.entry,
            tracked_asset_locks_count: 1,
            ..Default::default()
        };
        let locks = build_unused_asset_locks(&restore)?;
        let tracked = locks
            .values()
            .flat_map(|account| account.values())
            .next()
            .ok_or_else(|| PersistenceError::backend("asset-lock operation entry missing"))?;
        let mut entry = AssetLockEntry::from(tracked);
        entry.operation_id = self.has_operation_id.then_some(self.operation_id);
        entry.operation_revision = self.operation_revision;
        entry.expected_operation_revision =
            self.has_expected_revision.then_some(self.expected_revision);
        Ok(entry)
    }
}

/// Owner's removal decision; unknown status or revision fails closed.
#[no_mangle]
pub extern "C" fn asset_lock_operation_permits_removal(
    status: u8,
    has_operation_id: bool,
    revision: u64,
) -> bool {
    revision <= MAX_ASSET_LOCK_OPERATION_REVISION
        && status_from_u8(status)
            .map(|status| status.permits_removal(has_operation_id.then_some([0; 16]), revision))
            .unwrap_or(false)
}

/// Delegate lifecycle persistence to the wallet owner. Returns 1 to apply,
/// 2 to ignore an absorbed terminal snapshot, or 0 to reject/roll back.
/// Both carriers and their borrowed buffers must remain valid for this call;
/// a null predecessor denotes a genuinely absent row.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_operation_persistence_action(
    incoming: *const AssetLockOperationEntryFFI,
    previous: *const AssetLockOperationEntryFFI,
) -> u8 {
    std::panic::catch_unwind(|| {
        let Some(incoming) = incoming.as_ref() else {
            return 0;
        };
        let Ok(incoming) = incoming.decode() else {
            return 0;
        };
        let previous = match previous.as_ref().map(|entry| entry.decode()).transpose() {
            Ok(previous) => previous,
            Err(_) => return 0,
        };
        match incoming.persistence_action(previous.as_ref()) {
            Ok(AssetLockPersistenceAction::Apply) => 1,
            Ok(AssetLockPersistenceAction::Ignore) => 2,
            Err(_) => 0,
        }
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_lock_persistence::build_asset_lock_entries;
    use dashcore::{hashes::Hash, OutPoint, Transaction, Txid};
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::prelude::AssetLockProof;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::AssetLockStatus;
    use std::ffi::c_void;

    fn entry() -> AssetLockEntry {
        AssetLockEntry {
            operation_id: Some([9; 16]),
            operation_revision: 0,
            expected_operation_revision: None,
            out_point: OutPoint::new(Txid::from_byte_array([7; 32]), 0),
            transaction: Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            account_index: 0,
            funding_type: AssetLockFundingType::AssetLockShieldedAddressTopUp,
            identity_index: 0,
            amount_duffs: 123,
            status: AssetLockStatus::Broadcast,
            proof: None,
        }
    }

    #[test]
    fn should_marshal_conflict_and_reject_stale_revision_without_losing_binding() {
        let previous = entry();
        let mut conflict = previous.clone();
        conflict.status = AssetLockStatus::Conflicted;
        conflict.operation_revision = 1;
        conflict.expected_operation_revision = Some(0);
        let (rows, _storage) = build_asset_lock_entries(&[&previous, &conflict]);
        let old = AssetLockOperationEntryFFI::new(&previous, rows[0]);
        let current = AssetLockOperationEntryFFI::new(&conflict, rows[1]);
        assert_eq!(current.entry.status, 7);
        assert_eq!(unsafe { current.decode() }.unwrap(), conflict);
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&current, &old) },
            1
        );
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&old, &current) },
            0
        );
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&current, &current) },
            1
        );
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&current, std::ptr::null()) },
            0
        );
        let mut malformed = current;
        malformed.operation_revision = u64::MAX;
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&malformed, &current) },
            0
        );
    }

    #[test]
    fn should_decode_same_outpoint_recovery_proof_before_accepting_revision() {
        let mut conflict = entry();
        conflict.status = AssetLockStatus::Conflicted;
        conflict.operation_revision = 1;
        let mut recovery = conflict.clone();
        recovery.status = AssetLockStatus::RecoveredFromChain;
        recovery.operation_revision = 2;
        recovery.expected_operation_revision = Some(1);
        recovery.proof = Some(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 123,
            out_point: recovery.out_point,
        }));
        let mut wrong_outpoint = recovery.clone();
        wrong_outpoint.proof = Some(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 123,
            out_point: OutPoint::null(),
        }));
        let (rows, _storage) = build_asset_lock_entries(&[&conflict, &recovery, &wrong_outpoint]);
        let old = AssetLockOperationEntryFFI::new(&conflict, rows[0]);
        let recovered = AssetLockOperationEntryFFI::new(&recovery, rows[1]);
        let invalid = AssetLockOperationEntryFFI::new(&wrong_outpoint, rows[2]);
        assert_eq!(unsafe { recovered.decode() }.unwrap(), recovery);
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&recovered, &old) },
            1
        );
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&invalid, &old) },
            0
        );
        assert_eq!(
            unsafe { asset_lock_operation_persistence_action(&old, &recovered) },
            0
        );
    }

    #[test]
    fn should_keep_dispatched_operation_bindings_through_removal() {
        assert!(asset_lock_operation_permits_removal(0, true, 0));
        assert!(!asset_lock_operation_permits_removal(1, true, 0));
        assert!(!asset_lock_operation_permits_removal(7, true, 1));
        assert!(!asset_lock_operation_permits_removal(5, true, 2));
        assert!(!asset_lock_operation_permits_removal(255, false, 0));
        assert!(asset_lock_operation_permits_removal(1, false, 0));
    }
    #[test]
    fn should_replace_legacy_callback_even_for_revision_zero() {
        use crate::persistence::{
            FFIPersister, PersistenceCallbacks, PersistenceExtensionCallbacks,
        };
        use platform_wallet::changeset::{
            AssetLockChangeSet, PersistenceCapabilities, PlatformWalletChangeSet,
            PlatformWalletPersistence,
        };
        unsafe extern "C" fn legacy(
            _: *mut c_void,
            _: *const u8,
            _: *const AssetLockEntryFFI,
            _: usize,
            _: *const [u8; 36],
            _: usize,
        ) -> i32 {
            99
        }
        unsafe extern "C" fn ordered(
            context: *mut c_void,
            _: *const u8,
            entries: *const AssetLockOperationEntryFFI,
            count: usize,
            _: *const [u8; 36],
            _: usize,
        ) -> i32 {
            if count != 1 || (*entries).operation_revision != 0 || !(*entries).has_operation_id {
                return 1;
            }
            *(context as *mut bool) = true;
            0
        }
        let mut called = false;
        let persister = FFIPersister::new_with_persistence_capabilities_and_extensions(
            PersistenceCallbacks {
                context: (&mut called as *mut bool).cast(),
                on_persist_asset_locks_fn: Some(legacy),
                ..Default::default()
            },
            PersistenceCapabilities::NONE,
            PersistenceExtensionCallbacks {
                persist_asset_lock_operations: Some(ordered),
                ..Default::default()
            },
        );
        let entry = entry();
        persister
            .store(
                [42; 32],
                PlatformWalletChangeSet {
                    asset_locks: Some(AssetLockChangeSet {
                        asset_locks: [(entry.out_point, entry)].into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(called);
    }
}
