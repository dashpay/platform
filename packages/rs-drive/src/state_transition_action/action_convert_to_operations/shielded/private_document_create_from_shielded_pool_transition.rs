use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::private_document_create_from_shielded_pool::PrivateDocumentCreateFromShieldedPoolTransitionAction;
use crate::util::batch::drive_op_batch::PrivateDocumentOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PrivateDocumentCreateFromShieldedPoolTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .private_document_create_from_shielded_pool_transition
        {
            0 => match self {
                PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. The spent nullifiers, known not to exist after validation
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. The change notes
                    insert_notes(&mut ops, &v0.notes);

                    // 3. The pool loses the fee, the whole value balance, to the fee pools
                    let new_total_balance = v0
                        .current_total_balance
                        .checked_sub(v0.fee_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting the \
                                 private document create fee"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    // 4. The entries, in order. The transformer read the store and checked
                    // every entry takes the type's entry size, so the append cannot fail.
                    ops.push(DriveOperation::PrivateDocumentOperation(
                        PrivateDocumentOperationType::AppendEntries {
                            contract: v0.data_contract,
                            document_type_name: v0.document_type_name,
                            entries: v0.entries,
                            chunk_power: v0.store_chunk_power,
                        },
                    ));

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "PrivateDocumentCreateFromShieldedPoolTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fee_floor_support;
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::drive::document::private_document_store::PRIVATE_DOCUMENT_STORE_KEY;
    use crate::drive::{Drive, RootTree};
    use crate::state_transition_action::shielded::private_document_create_from_shielded_pool::v0::PrivateDocumentCreateFromShieldedPoolTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::Identifier;
    use dpp::shielded::compute_private_document_create_from_shielded_pool_fee;
    use grovedb::TransactionArg;
    use grovedb_costs::OperationCost;
    use serde_json::json;
    use std::sync::Arc;

    const CONTRACT_ID: [u8; 32] = [7; 32];

    fn note(i: u8) -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [i; 32],
            cmx: [i.wrapping_add(100); 32],
            cv_net: [0x33; 32],
            encrypted_note: vec![0x44; 216],
        }
    }

    /// The contract holding the private `secret` type of `entry_size`-byte entries.
    fn contract(entry_size: u32) -> Arc<DataContractFetchInfo> {
        let contract = DataContract::from_value(
            dpp::platform_value::to_value(json!({
                "$formatVersion": "1",
                "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
                "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                "version": 1,
                "documentSchemas": {
                    "secret": {
                        "type": "object",
                        "private": { "entrySize": entry_size },
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "properties": {
                            "accountReference": {
                                "type": "integer", "minimum": 0, "maximum": 4294967295u64,
                                "position": 0
                            }
                        },
                        "additionalProperties": false
                    }
                }
            }))
            .expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers");
        Arc::new(DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: OperationCost::default(),
            fee: None,
        })
    }

    fn action(
        contract: Arc<DataContractFetchInfo>,
        store_chunk_power: u8,
        notes: Vec<ShieldedActionNote>,
        entries: Vec<Vec<u8>>,
        fee_amount: u64,
        current_total_balance: u64,
    ) -> PrivateDocumentCreateFromShieldedPoolTransitionAction {
        PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(
            PrivateDocumentCreateFromShieldedPoolTransitionActionV0 {
                data_contract: contract,
                document_type_name: "secret".to_string(),
                entries,
                store_chunk_power,
                notes,
                anchor: [9; 32],
                fee_amount,
                current_total_balance,
            },
        )
    }

    fn ops(
        action: PrivateDocumentCreateFromShieldedPoolTransitionAction,
    ) -> Result<Vec<DriveOperation<'static>>, Error> {
        action.into_high_level_drive_operations(&Epoch::new(0).unwrap(), PlatformVersion::latest())
    }

    #[test]
    fn should_record_nullifiers_and_change_notes_lower_the_pool_and_append_the_entries_last() {
        let entries = vec![vec![1u8; 24], vec![2u8; 24]];
        let ops = ops(action(
            contract(24),
            11,
            vec![note(1), note(2)],
            entries.clone(),
            500,
            10_000,
        ))
        .expect("operations");
        // InsertNullifiers + 2 InsertNote + UpdateTotalBalance + AppendEntries
        assert_eq!(ops.len(), 5);
        assert!(matches!(
            &ops[0],
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::InsertNullifiers { nullifiers })
                if nullifiers == &vec![[1u8; 32], [2u8; 32]]
        ));
        assert!(matches!(
            &ops[3],
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::UpdateTotalBalance {
                new_total_balance: 9_500
            })
        ));
        match &ops[4] {
            DriveOperation::PrivateDocumentOperation(
                PrivateDocumentOperationType::AppendEntries {
                    contract,
                    document_type_name,
                    entries: appended,
                    chunk_power,
                },
            ) => {
                assert_eq!(contract.contract.id().to_buffer(), CONTRACT_ID);
                assert_eq!(document_type_name, "secret");
                assert_eq!(appended, &entries);
                assert_eq!(*chunk_power, 11);
            }
            other => panic!("expected AppendEntries, got {other:?}"),
        }
    }

    #[test]
    fn should_refuse_a_fee_over_the_pool_total() {
        assert!(ops(action(
            contract(24),
            11,
            vec![note(1)],
            vec![vec![1u8; 24]],
            10_001,
            10_000
        ))
        .is_err());
    }

    /// A drive holding the contract with the private `secret` type of `entry_size`-byte entries,
    /// and the chunk power its store committed.
    fn drive_with_private_type(entry_size: u32) -> (Drive, Arc<DataContractFetchInfo>, u8) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = contract(entry_size);
        drive
            .apply_contract(
                &contract.contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");
        let chunk_power = drive
            .private_document_store_chunk_power_taking_entries(
                &CONTRACT_ID,
                "secret",
                entry_size,
                None,
                platform_version,
            )
            .expect("expected to read the store")
            .expect("the store takes the type's entries");
        (drive, contract, chunk_power)
    }

    fn store_count(drive: &Drive, transaction: TransactionArg) -> u64 {
        let type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &CONTRACT_ID,
            &[1],
            b"secret",
        ];
        drive
            .grove
            .private_document_store_count(
                type_path.as_slice(),
                &[PRIVATE_DOCUMENT_STORE_KEY],
                transaction,
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .expect("the store's count")
    }

    #[test]
    fn should_read_the_chunk_power_of_a_store_taking_the_entries_and_none_otherwise() {
        let (drive, _, chunk_power) = drive_with_private_type(4096);
        // 4096 + 16 bytes to an entry keeps 64 of them near the 512 KiB target.
        assert_eq!(chunk_power, 6);
        let platform_version = PlatformVersion::latest();
        for (type_name, entry_size) in [("secret", 4095), ("missing", 4096)] {
            assert_eq!(
                drive
                    .private_document_store_chunk_power_taking_entries(
                        &CONTRACT_ID,
                        type_name,
                        entry_size,
                        None,
                        platform_version,
                    )
                    .expect("expected to read"),
                None,
                "{type_name} with entries of {entry_size} bytes"
            );
        }
    }

    /// The fixed fee of a private document create, which the pool pays, covers what its
    /// operations meter, and its entries' storage share covers the entries' metered storage, so
    /// the pool-paid split (`storage = min(metered storage, fee)`) never zeroes the proposer's
    /// share. Checked for an entry size at every chunk power a store can take (11 down to 6),
    /// for one entry and for the most a transition carries, over appends that cross a chunk's
    /// compaction. Every batch is estimated before it is applied: the estimate succeeds, with the
    /// store described at its committed chunk power, and the entries' estimate covers what their
    /// append meters.
    #[test]
    fn should_charge_a_fixed_fee_covering_the_metered_cost_at_every_chunk_power() {
        let platform_version = PlatformVersion::latest();
        let storage = &platform_version.fee_version.storage;
        let private_document = &platform_version.fee_version.private_document;
        let per_byte =
            storage.storage_disk_usage_credit_per_byte + storage.storage_processing_credit_per_byte;
        let max_entries = platform_version
            .system_limits
            .max_private_document_entries_per_transition
            .expect("protocol version 15 admits private document creates")
            as usize;
        let mut nonce = 0u8;
        let mut note_index = 0u32;
        for entry_size in [1u32, 200, 400, 585, 1500, 3000, 4096] {
            let (drive, contract, chunk_power) = drive_with_private_type(entry_size);
            let transaction = drive.grove.start_transaction();
            let entry_storage_allowance =
                (entry_size as u64 + private_document.entry_overhead_bytes) * per_byte;
            // 70 appends: at entry size 4096 (chunk power 6, 64 entries to a chunk) they cross a
            // chunk's compaction. GroveDB charges every append of a store the same, the
            // compacting one included, so the smaller sizes' appends meter what any of theirs
            // does.
            let mut appended = 0u64;
            for entry_count in [1, max_entries, max_entries, 1, 1] {
                nonce = nonce.wrapping_add(1);
                // Distinct entries, as basic structure requires.
                let entries: Vec<Vec<u8>> = (0..entry_count)
                    .map(|i| {
                        let mut entry = vec![nonce; entry_size as usize];
                        entry[0] = i as u8;
                        entry
                    })
                    .collect();
                let fee = compute_private_document_create_from_shielded_pool_fee(
                    2,
                    entry_count,
                    entry_size,
                    platform_version,
                )
                .expect("fee");
                note_index += 2;
                let transition_operations = || {
                    ops(action(
                        contract.clone(),
                        chunk_power,
                        vec![
                            fee_floor_support::note(note_index),
                            fee_floor_support::note(note_index + 1),
                        ],
                        entries.clone(),
                        fee,
                        u64::MAX / 2,
                    ))
                    .expect("operations")
                };
                let apply = |operations, apply| {
                    drive.apply_drive_operations(
                        operations,
                        apply,
                        &BlockInfo::default(),
                        Some(&transaction),
                        platform_version,
                    )
                };
                apply(transition_operations(), false).unwrap_or_else(|e| {
                    panic!("entry size {entry_size}: the transition must estimate: {e}")
                });
                let metered = apply(transition_operations(), true).expect("apply");
                assert!(
                    fee >= metered.total_base_fee(),
                    "entry size {entry_size}, {entry_count} entries: fixed fee {fee} must cover \
                     the metered {}",
                    metered.total_base_fee()
                );
                // The entries alone, estimated and then metered on their own append.
                let entries_operations = || {
                    vec![DriveOperation::PrivateDocumentOperation(
                        PrivateDocumentOperationType::AppendEntries {
                            contract: contract.clone(),
                            document_type_name: "secret".to_string(),
                            entries: entries.clone(),
                            chunk_power,
                        },
                    )]
                };
                let entries_estimated = apply(entries_operations(), false).unwrap_or_else(|e| {
                    panic!("entry size {entry_size}: the entries must estimate: {e}")
                });
                let entries_metered = apply(entries_operations(), true).expect("apply the entries");
                assert!(
                    entries_estimated.storage_fee >= entries_metered.storage_fee
                        && entries_estimated.total_base_fee() >= entries_metered.total_base_fee(),
                    "entry size {entry_size}, {entry_count} entries: the estimate {entries_estimated:?} \
                     must cover the metered {entries_metered:?}"
                );
                assert!(
                    entry_count as u64 * entry_storage_allowance >= entries_metered.storage_fee,
                    "entry size {entry_size}, {entry_count} entries: the entries' storage \
                     allowance {} must cover their metered storage {}",
                    entry_count as u64 * entry_storage_allowance,
                    entries_metered.storage_fee
                );
                appended += 2 * entry_count as u64;
            }
            assert_eq!(store_count(&drive, Some(&transaction)), appended);
        }
    }

    /// Basic structure refuses two identical entries in one transition, but the store takes
    /// them: from protocol version 15, GroveDB V5 applies repeated appends of the same value to
    /// one store in one batch, which earlier GroveDB versions refuse as the same operation twice.
    #[test]
    fn should_append_two_identical_entries_in_one_batch() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract, chunk_power) = drive_with_private_type(24);
        drive
            .apply_drive_operations(
                vec![DriveOperation::PrivateDocumentOperation(
                    PrivateDocumentOperationType::AppendEntries {
                        contract,
                        document_type_name: "secret".to_string(),
                        entries: vec![vec![5u8; 24], vec![5u8; 24]],
                        chunk_power,
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to append both entries");
        assert_eq!(store_count(&drive, None), 2);
    }
}
