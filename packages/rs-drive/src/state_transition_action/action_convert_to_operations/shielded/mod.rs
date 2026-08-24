mod identity_create_from_shielded_pool_transition;
mod shield_from_asset_lock_transition;
mod shield_transition;
mod shielded_transfer_transition;
mod shielded_withdrawal_transition;
mod unshield_transition;

use crate::state_transition_action::shielded::ShieldedActionNote;
use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
use crate::util::batch::DriveOperation;

/// Insert nullifiers into the permanent tree (double-spend prevention) and
/// per-block sync storage (catch-up RPCs).
pub(super) fn insert_nullifiers<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    notes: &[ShieldedActionNote],
) {
    if !notes.is_empty() {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNullifiers {
                nullifiers: notes.iter().map(|n| n.nullifier).collect(),
            },
        ));
    }
}

/// Insert notes into the CommitmentTree (appends cmx to frontier + stores
/// cmx||rho||cv_net||encrypted_note).
///
/// Each action's nullifier (rho) is stored alongside the note so light clients can derive
/// Rho for trial decryption; cv_net is stored unencrypted for OVK recovery of outgoing notes.
pub(super) fn insert_notes<'a>(ops: &mut Vec<DriveOperation<'a>>, notes: &[ShieldedActionNote]) {
    for note in notes {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNote {
                nullifier: note.nullifier,
                cmx: note.cmx,
                cv_net: note.cv_net,
                encrypted_note: note.encrypted_note.clone(),
            },
        ));
    }
}

/// Update pool total balance.
pub(super) fn update_balance<'a>(ops: &mut Vec<DriveOperation<'a>>, new_total_balance: u64) {
    ops.push(DriveOperation::ShieldedPoolOperation(
        ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
    ));
}

/// Measurement support for the pool-paid shielded fee-floor tests.
///
/// The pool-paid shielded transitions (ShieldedTransfer, Unshield,
/// ShieldedWithdrawal) charge a FLAT fee and book
/// `storage_fee = min(actual_storage, flat)`, `processing = flat -
/// storage_fee`; they never validate affordability against a per-transition
/// estimate. The invariant such a fee has to satisfy is therefore an
/// **amortized** one: over a whole commitment-tree epoch — including the one
/// append per epoch that compacts the dense buffer into a chunk blob — the
/// flat fee must cover the average real write cost, and must stay above the
/// average real storage so the booking split never starves the proposer.
/// Under the GROVE_V4 fixed per-append model (grovedb #829/#830) the
/// compaction is amortized inside GroveDB itself, so the epoch average and
/// the boundary append coincide; measuring the full epoch keeps the
/// invariant honest under any model — whatever a boundary append meters,
/// the pool absorbs it, and a client cannot land on it more than once per
/// epoch.
///
/// Measured once per test binary (an epoch is 2048 real appends).
#[cfg(test)]
pub(super) mod fee_floor_support {
    use std::sync::OnceLock;

    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    use crate::drive::shielded::paths::SHIELDED_NOTES_CHUNK_POWER;
    use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
    use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
    use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    /// Real metered cost of one epoch of 1-action shielded transfers,
    /// applied for real (`apply = true`) on a fresh pool.
    pub(crate) struct TransferEpoch {
        /// Appends measured (one epoch).
        pub appends: u64,
        /// Average total (storage + processing) per append.
        pub avg_total: u64,
        /// Average storage fee per append.
        pub avg_storage: u64,
        /// The compacting append's storage fee — the figure the pool-paid
        /// booking split `min(actual_storage, flat)` must stay below.
        pub boundary_storage: u64,
        /// The compacting append's total.
        pub boundary_total: u64,
        /// An ordinary (non-compacting) append's total, late in the epoch.
        pub ordinary_total: u64,
    }

    /// A production-sized 1-action transfer, distinct per index.
    pub(crate) fn transfer_action(i: u32, fee_amount: u64) -> ShieldedTransferTransitionAction {
        ShieldedTransferTransitionAction::V0(ShieldedTransferTransitionActionV0 {
            notes: vec![note(i)],
            anchor: [0xAA; 32],
            fee_amount,
            current_total_balance: fee_amount + 1_000_000,
        })
    }

    /// A production-sized note (216-byte ciphertext), distinct per index.
    pub(crate) fn note(i: u32) -> ShieldedActionNote {
        let b = i.to_be_bytes();
        let mut nf = [0u8; 32];
        nf[..4].copy_from_slice(&b);
        nf[4] = 1;
        let mut cmx = [0u8; 32];
        cmx[..4].copy_from_slice(&b);
        cmx[4] = 2;
        let mut cv = [0u8; 32];
        cv[..4].copy_from_slice(&b);
        cv[4] = 3;
        ShieldedActionNote {
            nullifier: nf,
            cmx,
            cv_net: cv,
            encrypted_note: vec![0x77; 216],
        }
    }

    static EPOCH: OnceLock<TransferEpoch> = OnceLock::new();

    /// One epoch of real 1-action transfers, measured once.
    pub(crate) fn transfer_epoch() -> &'static TransferEpoch {
        EPOCH.get_or_init(|| {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();
            let epoch = Epoch::new(0).unwrap();
            let tx = drive.grove.start_transaction();
            let appends: u64 = 1u64 << SHIELDED_NOTES_CHUNK_POWER;
            let fee_amount = 1_000_000_000;
            let mut total = 0u64;
            let mut storage = 0u64;
            let mut boundary_storage = 0;
            let mut boundary_total = 0;
            let mut ordinary_total = 0;
            for i in 0..appends {
                let ops = transfer_action(i as u32, fee_amount)
                    .into_high_level_drive_operations(&epoch, platform_version)
                    .expect("operations");
                let fr = drive
                    .apply_drive_operations(
                        ops,
                        true,
                        &BlockInfo::default(),
                        Some(&tx),
                        platform_version,
                        None,
                    )
                    .expect("apply");
                let t = fr.total_base_fee();
                total += t;
                storage += fr.storage_fee;
                if i + 1 == appends {
                    boundary_storage = fr.storage_fee;
                    boundary_total = t;
                }
                if i + 8 == appends {
                    ordinary_total = t;
                }
            }
            TransferEpoch {
                appends,
                // Ceiling division: a truncated average could let a flat fee
                // marginally below the exact epoch-wide cost slip past the
                // floor assertions.
                avg_total: total.div_ceil(appends),
                avg_storage: storage.div_ceil(appends),
                boundary_storage,
                boundary_total,
                ordinary_total,
            }
        })
    }
}
