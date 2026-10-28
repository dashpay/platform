use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 2 of [`Drive::convert_drive_operations_to_grove_operations`] (protocol version
    /// 15): version 1, with every document operation built against the operations of the batch
    /// built before it, as `apply_drive_operations` 2 builds them
    /// ([`DriveOperation::append_low_level_drive_operations_after_ttl_drain`]); other operation
    /// kinds are built as before. Two deletions that empty an index tree together remove it,
    /// and an insert into a tree an earlier deletion of the batch empties keeps the tree.
    /// Credits that repaid an identity's debt and ephemeral operations are refused as by version
    /// 0, for the same reasons.
    ///
    /// The batch is prepared as `apply_drive_operations` prepares it, because each write of an
    /// identity balance, a contract fee pot, a prefunded specialized balance or a token key
    /// sets the value computed from state before the batch, so all but the last of two would
    /// be lost: a batch writing one token balance, supply, pool total or escrow more than once
    /// is refused ([`DriveOperation::refuse_repeated_token_balance_writes`]), and repeated
    /// balance writes are merged into one ([`DriveOperation::merge_balance_writes`]).
    ///
    /// No platform code calls it at protocol version 15: the proposer payout credits through
    /// `apply_drive_operations` from protocol version 14.
    #[inline(always)]
    pub(super) fn convert_drive_operations_to_grove_operations_v2(
        &self,
        drive_batch_operations: Vec<DriveOperation>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOpBatch, Error> {
        DriveOperation::refuse_repeated_token_balance_writes(&drive_batch_operations)?;
        self.refuse_repeated_counter_moves(
            &drive_batch_operations,
            block_info,
            transaction,
            platform_version,
        )?;
        let drive_batch_operations = DriveOperation::merge_balance_writes(drive_batch_operations)?;
        self.prepare_drive_operations_time_range_ttl(
            &drive_batch_operations,
            block_info,
            transaction,
            platform_version,
        )?;
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        for drive_op in drive_batch_operations {
            drive_op.append_low_level_drive_operations_after_ttl_drain(
                self,
                &mut None,
                block_info,
                &mut batch_operations,
                transaction,
                platform_version,
            )?;
        }
        if LowLevelDriveOperation::holds_repaid_identity_debt(&batch_operations) {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "convert_drive_operations_to_grove_operations cannot carry credits that repaid an \
                 identity's debt; apply them through apply_drive_operations",
            )));
        }
        if batch_operations.iter().any(|operation| {
            matches!(
                operation,
                LowLevelDriveOperation::EphemeralGroveOperation(..)
            )
        }) {
            return Err(Error::Drive(DriveError::NotSupported(
                "convert_drive_operations_to_grove_operations returns one plain batch and cannot \
                 carry ephemeral operations (a TTL'd subtree's, or a document's with a time to \
                 live), whose bytes are priced separately; apply them through \
                 apply_drive_operations",
            )));
        }
        Ok(GroveDbOpBatch::from_operations(
            LowLevelDriveOperation::grovedb_operations_consume(batch_operations),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::batch::drive_op_batch::TokenOperationType;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::{DriveOperation, IdentityOperationType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::test_utils::identities::create_test_identity;
    use dpp::block::block_info::BlockInfo;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    /// A batch writing one token key twice is refused: a frozen holding destroy naming one
    /// listing twice, and two destroys of one holding.
    #[test]
    fn should_refuse_repeated_token_writes() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let destroy = |order_ids: Vec<Identifier>| {
            DriveOperation::TokenOperation(TokenOperationType::TokenDestroyFrozenHolding {
                token_id: Identifier::from([5; 32]),
                holder_id: Identifier::from([6; 32]),
                balance_amount: 10,
                destroyed_amount: 10 + order_ids.len() as u64,
                order_ids,
            })
        };
        let listing = Identifier::from([7; 32]);
        for (case, operations) in [
            ("one listing twice", vec![destroy(vec![listing, listing])]),
            ("one holding twice", vec![destroy(vec![]), destroy(vec![])]),
        ] {
            let refused = drive.convert_drive_operations_to_grove_operations(
                operations,
                &BlockInfo::default(),
                None,
                latest,
            );
            assert!(
                matches!(
                    &refused,
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(message)))
                        if message.contains("more than once")
                ),
                "{case}: {refused:?}"
            );
        }
    }

    /// Two credits to one identity in one batch: generation 1 (protocol version 14) builds
    /// both from the committed balance, so its batch writes the balance twice and the first
    /// credit would be lost; generation 2 merges them into one write of both.
    #[test]
    fn should_merge_repeated_balance_writes_from_protocol_version_15() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let identity_id = [9; 32];
        create_test_identity(&drive, identity_id, Some(1), None, latest)
            .expect("expected to create the identity");
        let before = drive
            .fetch_identity_balance(identity_id, None, latest)
            .expect("expected to fetch the balance")
            .expect("expected a balance");
        let credits = || {
            vec![
                DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                    identity_id,
                    added_balance: 100,
                }),
                DriveOperation::IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                    identity_id,
                    added_balance: 50,
                }),
            ]
        };

        let generation_1 = PlatformVersion::get(14).expect("expected protocol version 14");
        let blind = drive
            .convert_drive_operations_to_grove_operations(
                credits(),
                &BlockInfo::default(),
                None,
                generation_1,
            )
            .expect("expected the conversion");
        assert_eq!(blind.len(), 2, "two writes of one balance: {blind:?}");

        let merged = drive
            .convert_drive_operations_to_grove_operations(
                credits(),
                &BlockInfo::default(),
                None,
                latest,
            )
            .expect("expected the conversion");
        assert_eq!(merged.len(), 1, "one write of both credits: {merged:?}");
        drive
            .grove_apply_batch(merged, false, None, &latest.drive)
            .expect("expected the batch to apply");
        assert_eq!(
            drive
                .fetch_identity_balance(identity_id, None, latest)
                .expect("expected to fetch the balance"),
            Some(before + 150)
        );
    }
}
