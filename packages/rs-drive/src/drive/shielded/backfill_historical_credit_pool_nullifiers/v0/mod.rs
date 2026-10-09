use crate::drive::shielded::paths::{
    shielded_credit_pool_nullifiers_path, shielded_credit_pool_nullifiers_path_vec,
    shielded_credit_pool_path, MAIN_SHIELDED_CREDIT_POOL_KEY, SHIELDED_NOTES_CHUNK_POWER,
    SHIELDED_NOTES_KEY, SHIELDED_NULLIFIERS_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::{Element, RangePage, Transaction};
use std::collections::BTreeSet;

fn corrupted(message: &'static str) -> Error {
    Error::Drive(DriveError::CriticalCorruptedState(message))
}

/// Checks the entire page before any of its rhos can become permanent nullifiers.
fn validated_page(
    page: RangePage,
    start: u64,
    total: u64,
    limit: u16,
) -> Result<(u64, BTreeSet<[u8; 32]>), Error> {
    if page.total_count != total {
        return Err(corrupted(
            "CREDIT note count changed during nullifier backfill",
        ));
    }
    let remaining = total
        .checked_sub(start)
        .ok_or_else(|| corrupted("CREDIT note cursor exceeds count"))?;
    let expected = remaining.min(u64::from(limit));
    if u64::try_from(page.entries.len()).ok() != Some(expected) {
        return Err(corrupted("CREDIT note page is incomplete"));
    }
    let end = start
        .checked_add(expected)
        .ok_or_else(|| corrupted("CREDIT note cursor overflow"))?;
    let mut cursor = start;
    let mut rhos = BTreeSet::new();
    for (position, row) in page.entries {
        if position != cursor {
            return Err(corrupted("CREDIT note positions are not consecutive"));
        }
        if row.len() < 96 {
            return Err(corrupted("CREDIT note has a truncated fixed prefix"));
        }
        let rho = row
            .get(32..64)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| corrupted("CREDIT note rho is malformed"))?;
        rhos.insert(rho);
        cursor = cursor
            .checked_add(1)
            .ok_or_else(|| corrupted("CREDIT note cursor overflow"))?;
    }
    Ok((end, rhos))
}

impl Drive {
    #[inline(always)]
    pub(super) fn backfill_historical_credit_pool_nullifiers_v0(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.backfill_credit_nullifiers_with_io_v0(
            |start, limit| {
                self.grove
                    .commitment_tree_get_range(
                        shielded_credit_pool_path().as_slice(),
                        &[SHIELDED_NOTES_KEY],
                        start,
                        limit,
                        Some(transaction),
                        &platform_version.drive.grove_version,
                    )
                    .value
                    .map_err(Error::from)
            },
            |batch| {
                self.grove_apply_batch(
                    GroveDbOpBatch { operations: batch },
                    false,
                    Some(transaction),
                    &platform_version.drive,
                )
            },
            transaction,
            platform_version,
        )
    }

    // The I/O callbacks permit deterministic storage faults in tests without any
    // global failpoint or process state on the candidate execution path.
    fn backfill_credit_nullifiers_with_io_v0(
        &self,
        mut read_page: impl FnMut(u64, u16) -> Result<RangePage, Error>,
        mut apply_batch: impl FnMut(Vec<QualifiedGroveDbOp>) -> Result<(), Error>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let page_size = platform_version
            .system_limits
            .credit_pool_nullifier_backfill_page_size;
        let batch_size = usize::from(
            platform_version
                .system_limits
                .credit_pool_nullifier_backfill_batch_size,
        );
        if page_size == 0 || batch_size == 0 {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "CREDIT backfill limits must be nonzero",
            )));
        }
        let grove_version = &platform_version.drive.grove_version;
        let root_path: [&[u8]; 1] = [&[RootTree::ShieldedBalances as u8]];
        let pool = self
            .grove
            .get_raw_optional(
                root_path.as_slice().into(),
                MAIN_SHIELDED_CREDIT_POOL_KEY,
                Some(transaction),
                grove_version,
            )
            .value
            .map_err(Error::from)?;
        if !matches!(pool, Some(Element::SumTree(..))) {
            return Err(corrupted("CREDIT pool must be a SumTree"));
        }
        let pool_path = shielded_credit_pool_path();
        let notes = self
            .grove
            .get_raw_optional(
                pool_path.as_slice().into(),
                &[SHIELDED_NOTES_KEY],
                Some(transaction),
                grove_version,
            )
            .value
            .map_err(Error::from)?;
        let total = match notes {
            Some(Element::CommitmentTree(total, SHIELDED_NOTES_CHUNK_POWER, _)) => total,
            _ => {
                return Err(corrupted(
                    "CREDIT notes must be a commitment tree with the expected chunk power",
                ))
            }
        };
        let nullifiers = self
            .grove
            .get_raw_optional(
                pool_path.as_slice().into(),
                &[SHIELDED_NULLIFIERS_KEY],
                Some(transaction),
                grove_version,
            )
            .value
            .map_err(Error::from)?;
        if !matches!(nullifiers, Some(Element::ProvableCountTree(..))) {
            return Err(corrupted("CREDIT nullifiers must be a ProvableCountTree"));
        }
        let nullifiers_path = shielded_credit_pool_nullifiers_path();
        let mut start = 0;
        loop {
            let (end, rhos) =
                validated_page(read_page(start, page_size)?, start, total, page_size)?;
            let mut batch = Vec::with_capacity(batch_size.min(rhos.len()));
            for rho in rhos {
                let existing = self
                    .grove
                    .get_raw_optional(
                        nullifiers_path.as_slice().into(),
                        &rho,
                        Some(transaction),
                        grove_version,
                    )
                    .value
                    .map_err(Error::from)?;
                match existing {
                    Some(Element::Item(..)) => continue,
                    Some(_) => return Err(corrupted("CREDIT nullifier must be an Item")),
                    None => batch.push(
                        QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                            shielded_credit_pool_nullifiers_path_vec(),
                            rho.to_vec(),
                            Element::new_item(vec![]),
                        ),
                    ),
                }
                if batch.len() == batch_size {
                    apply_batch(std::mem::take(&mut batch))?;
                }
            }
            // Finish even a partial batch before reading another page: repeated rhos
            // on that page must see every insert from this one in the same transaction.
            if !batch.is_empty() {
                apply_batch(batch)?;
            }
            if end == total {
                break;
            }
            start = end;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
