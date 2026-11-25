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
    pub has_shielded_recipient: bool,
    pub shielded_recipient: [u8; 43],
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
            has_shielded_recipient: entry.shielded_recipient.is_some(),
            shielded_recipient: entry.shielded_recipient.unwrap_or([0; 43]),
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
        entry.shielded_recipient = self
            .has_shielded_recipient
            .then_some(self.shielded_recipient);
        Ok(entry)
    }
}

/// Owner's removal decision; unknown status or revision fails closed.
#[no_mangle]
pub extern "C" fn asset_lock_operation_permits_removal(
    status: u8,
    has_operation_id: bool,
    has_shielded_recipient: bool,
    revision: u64,
) -> bool {
    status_from_u8(status).is_ok_and(|status| {
        status.permits_removal(has_operation_id, has_shielded_recipient, revision)
    })
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
    use crate::core_wallet_types::SweepBatchFFI;
    use crate::persistence::{FFIPersister, PersistenceCallbacks, PersistenceExtensionCallbacks};
    use dashcore::{OutPoint, Transaction, Txid};
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::prelude::AssetLockProof;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::changeset::SweepBatch;
    use platform_wallet::changeset::{
        AssetLockChangeSet, CoreChangeSet, PersistenceCapabilities, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use platform_wallet::AssetLockStatus;
    use std::ffi::c_void;

    fn entry() -> AssetLockEntry {
        AssetLockEntry {
            operation_id: Some([9; 16]),
            operation_revision: 0,
            expected_operation_revision: None,
            shielded_recipient: None,
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
        assert_eq!(unsafe { old.decode() }.unwrap(), previous);
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
    fn should_preserve_recipient_across_funding_conflict() {
        let mut previous = entry();
        previous.operation_id = None;
        previous.shielded_recipient = Some([0x71; 43]);
        let mut conflict = previous.clone();
        conflict.status = AssetLockStatus::Conflicted;
        conflict.advance_operation_revision();
        let mut recovery = conflict.clone();
        recovery.status = AssetLockStatus::RecoveredFromChain;
        recovery.shielded_recipient = None;
        recovery.proof = Some(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 88,
            out_point: recovery.out_point,
        }));
        recovery.advance_operation_revision();
        let (rows, _storage) = build_asset_lock_entries(&[&previous, &conflict, &recovery]);
        let old = AssetLockOperationEntryFFI::new(&previous, rows[0]);
        let current = AssetLockOperationEntryFFI::new(&conflict, rows[1]);
        let returned = AssetLockOperationEntryFFI::new(&recovery, rows[2]);
        assert_eq!(unsafe { old.decode() }.unwrap(), previous);
        assert_eq!(unsafe { current.decode() }.unwrap(), conflict);
        let action =
            |incoming, stored| unsafe { asset_lock_operation_persistence_action(incoming, stored) };
        assert_eq!(action(&current, &old), 1);
        assert_eq!(action(&old, &current), 0);

        let mut redelivered = current;
        redelivered.has_shielded_recipient = false;
        assert_eq!(action(&redelivered, &old), 1);
        assert_eq!(action(&redelivered, &current), 1);
        assert_eq!(action(&returned, &current), 1);
        let mut redirected = current;
        redirected.shielded_recipient = [0x72; 43];
        assert_eq!(action(&redirected, &old), 0);
        assert_eq!(action(&redirected, &current), 0);
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
    fn should_keep_dispatched_recovery_bindings_through_removal() {
        for (operation, recipient) in [(true, false), (false, true), (true, true)] {
            let allowed = |status, revision| {
                asset_lock_operation_permits_removal(status, operation, recipient, revision)
            };
            assert!(allowed(0, 0));
            assert!(!allowed(1, 0));
            assert!(!allowed(7, 1));
            assert!(!allowed(5, 2));
        }
        assert!(!asset_lock_operation_permits_removal(255, false, false, 0));
        assert!(!asset_lock_operation_permits_removal(7, false, false, 0));
        assert!(!asset_lock_operation_permits_removal(4, false, false, 0));
        assert!(asset_lock_operation_permits_removal(1, false, false, 0));
    }

    #[test]
    fn should_project_funding_conflict_to_legacy_ffi_store() {
        #[derive(Default)]
        struct Round {
            staged: Vec<&'static str>,
            committed: Vec<&'static str>,
            ordered_calls: usize,
        }
        unsafe extern "C" fn begin(context: *mut c_void, _: *const u8) -> i32 {
            let round = &mut *context.cast::<Round>();
            round.staged.clear();
            0
        }
        unsafe extern "C" fn end(context: *mut c_void, _: *const u8, commit: bool) -> i32 {
            let round = &mut *context.cast::<Round>();
            if commit {
                round.committed.append(&mut round.staged);
            }
            round.staged.clear();
            0
        }
        unsafe extern "C" fn sweep(
            context: *mut c_void,
            _: *const u8,
            rows: *const SweepBatchFFI,
            count: usize,
        ) -> i32 {
            if count != 1 || (*rows).txids_count != 1 || *(*rows).txids != [7; 32] {
                return 1;
            }
            (*context.cast::<Round>()).staged.push("core sweep");
            0
        }
        unsafe extern "C" fn legacy(
            context: *mut c_void,
            _: *const u8,
            rows: *const AssetLockEntryFFI,
            row_count: usize,
            removed: *const [u8; 36],
            removed_count: usize,
        ) -> i32 {
            let event = match (row_count, removed_count) {
                (0, 1) if (&*removed)[..32] == [7; 32] => "legacy remove",
                (1, 0) if (*rows).status == 5 => "legacy recovery",
                (1, 0) if (*rows).status == 4 => "legacy consumption",
                _ => return 1,
            };
            (*context.cast::<Round>()).staged.push(event);
            0
        }
        unsafe extern "C" fn ordered(
            context: *mut c_void,
            _: *const u8,
            _: *const AssetLockOperationEntryFFI,
            _: usize,
            _: *const [u8; 36],
            _: usize,
        ) -> i32 {
            (*context.cast::<Round>()).ordered_calls += 1;
            0
        }
        unsafe extern "C" fn load_revision(
            _: *mut c_void,
            _: *const u8,
            _: *const u8,
            _: *mut u64,
        ) -> i32 {
            0
        }
        unsafe extern "C" fn load_binding(
            _: *mut c_void,
            _: *const u8,
            _: *const u8,
            _: *mut bool,
            _: *mut u8,
        ) -> i32 {
            0
        }
        unsafe extern "C" fn persist_recipient(
            _: *mut c_void,
            _: *const u8,
            _: *const u8,
            _: *const u8,
        ) -> i32 {
            0
        }

        let build_round = |entry: AssetLockEntry, with_sweep: bool| PlatformWalletChangeSet {
            core: with_sweep.then(|| CoreChangeSet {
                sweeps: vec![SweepBatch {
                    txids: vec![entry.out_point.txid],
                    superseded_by: Txid::from_byte_array([8; 32]),
                    winner_mined_height: Some(5),
                    released_outpoints: Vec::new(),
                }],
                ..Default::default()
            }),
            asset_locks: Some(AssetLockChangeSet {
                asset_locks: [(entry.out_point, entry)].into_iter().collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let host = |round: &mut Round, extensions, declaration| {
            FFIPersister::new_with_persistence_capabilities_and_extensions(
                PersistenceCallbacks {
                    context: (round as *mut Round).cast(),
                    on_changeset_begin_fn: Some(begin),
                    on_changeset_end_fn: Some(end),
                    on_persist_asset_locks_fn: Some(legacy),
                    ..Default::default()
                },
                declaration,
                extensions,
            )
        };
        let base = PersistenceExtensionCallbacks {
            wallet_changeset_sweeps: Some(sweep),
            ..Default::default()
        };
        let wallet_id = [42; 32];
        let mut conflict = entry();
        conflict.operation_id = None;
        conflict.shielded_recipient = Some([0x71; 43]);
        conflict.status = AssetLockStatus::Conflicted;
        conflict.advance_operation_revision();
        let mut round = Round::default();
        let persister = host(&mut round, base, PersistenceCapabilities::NONE);
        persister
            .store(wallet_id, build_round(conflict.clone(), true))
            .unwrap();
        assert_eq!(round.committed, ["core sweep", "legacy remove"]);
        let mut recovery = conflict.clone();
        recovery.status = AssetLockStatus::RecoveredFromChain;
        recovery.proof = Some(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 88,
            out_point: recovery.out_point,
        }));
        recovery.advance_operation_revision();
        persister
            .store(wallet_id, build_round(recovery.clone(), false))
            .unwrap();
        recovery.status = AssetLockStatus::Consumed;
        recovery.expected_operation_revision = None;
        persister
            .store(wallet_id, build_round(recovery, false))
            .unwrap();
        assert_eq!(
            round.committed,
            [
                "core sweep",
                "legacy remove",
                "legacy recovery",
                "legacy consumption"
            ]
        );

        let uuid_only = PersistenceExtensionCallbacks {
            persist_asset_lock_operations: Some(ordered),
            load_asset_lock_operation_revision: Some(load_revision),
            load_asset_lock_operation_id: Some(load_binding),
            ..base
        };
        for (extensions, declaration, operation_id) in [
            (base, PersistenceCapabilities::NONE, Some([1; 16])),
            (
                base,
                PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS,
                None,
            ),
            (
                PersistenceExtensionCallbacks {
                    load_asset_lock_shielded_recipient: Some(load_binding),
                    ..base
                },
                PersistenceCapabilities::NONE,
                None,
            ),
            (
                PersistenceExtensionCallbacks {
                    persist_asset_lock_operations: Some(ordered),
                    load_asset_lock_operation_revision: Some(load_revision),
                    persist_asset_lock_shielded_recipient: Some(persist_recipient),
                    ..base
                },
                PersistenceCapabilities::NONE,
                None,
            ),
            (
                PersistenceExtensionCallbacks {
                    persist_asset_lock_operations: Some(ordered),
                    load_asset_lock_shielded_recipient: Some(load_binding),
                    persist_asset_lock_shielded_recipient: Some(persist_recipient),
                    ..base
                },
                PersistenceCapabilities::NONE,
                None,
            ),
            (
                uuid_only,
                PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS,
                Some([1; 16]),
            ),
            (
                PersistenceExtensionCallbacks {
                    persist_asset_lock_shielded_recipient: Some(persist_recipient),
                    ..uuid_only
                },
                PersistenceCapabilities::NONE,
                Some([1; 16]),
            ),
        ] {
            let mut round = Round::default();
            let conflict = AssetLockEntry {
                operation_id,
                ..conflict.clone()
            };
            assert!(host(&mut round, extensions, declaration)
                .store(wallet_id, build_round(conflict, true))
                .is_err());
            assert!(round.committed.is_empty());
            assert_eq!(round.ordered_calls, 0);
        }
        let full = PersistenceExtensionCallbacks {
            load_asset_lock_shielded_recipient: Some(load_binding),
            persist_asset_lock_shielded_recipient: Some(persist_recipient),
            ..uuid_only
        };
        for (extensions, operation_id) in [
            (uuid_only, Some([1; 16])),
            (full, None),
            (full, Some([1; 16])),
        ] {
            let mut round = Round::default();
            let conflict = AssetLockEntry {
                operation_id,
                ..conflict.clone()
            };
            host(&mut round, extensions, PersistenceCapabilities::NONE)
                .store(wallet_id, build_round(conflict, true))
                .unwrap();
            assert_eq!(round.committed, ["core sweep"]);
            assert_eq!(round.ordered_calls, 1);
        }
    }

    #[test]
    fn should_replace_legacy_callback_even_for_revision_zero() {
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
