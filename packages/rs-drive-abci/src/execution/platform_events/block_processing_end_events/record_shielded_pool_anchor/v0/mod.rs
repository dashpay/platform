use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_anchors_path, shielded_credit_pool_path, SHIELDED_MOST_RECENT_ANCHOR_KEY,
    SHIELDED_NOTES_KEY,
};
use drive::grovedb::{Element, Transaction};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the current shielded pool anchor if the commitment tree changed this block.
    ///
    /// After all state transitions are processed, reads the current Sinsemilla anchor
    /// from the CommitmentTree at [AddressBalances, "s", [1]]. If it differs from the
    /// most recent anchor (stored at [AddressBalances, "s", [7]]), inserts
    /// `anchor_bytes → block_height.to_be_bytes()` into the anchors tree at
    /// [AddressBalances, "s", [6]] and updates the most recent anchor.
    ///
    /// This ensures anchors are only recorded once per block (not per-transaction),
    /// and only when the commitment tree actually changed.
    pub(super) fn record_shielded_pool_anchor_if_changed_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let grove_version = &platform_version.drive.grove_version;
        let pool_path = shielded_credit_pool_path();

        // 1. Read current anchor from CommitmentTree
        let current_anchor = self
            .drive
            .grove
            .commitment_tree_anchor(
                &pool_path,
                &[SHIELDED_NOTES_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(|e| Error::Drive(drive::error::Error::from(e)))?;

        let current_anchor_bytes: [u8; 32] = current_anchor.to_bytes();

        // 2. Read most recent anchor from the dedicated element
        let most_recent_anchor: [u8; 32] = self
            .drive
            .grove
            .get(
                &pool_path,
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(|e| Error::Drive(drive::error::Error::from(e)))
            .and_then(|element| {
                if let Element::Item(value, _) = element {
                    value.try_into().map_err(|_| {
                        Error::Drive(drive::error::Error::Drive(
                            drive::error::drive::DriveError::CorruptedElementType(
                                "most recent anchor is not 32 bytes",
                            ),
                        ))
                    })
                } else {
                    Ok([0u8; 32])
                }
            })?;

        // 3. Only store if different (skip zero anchor from empty tree)
        let should_store =
            current_anchor_bytes != most_recent_anchor && current_anchor_bytes != [0u8; 32];

        if should_store {
            let anchors_path = shielded_credit_pool_anchors_path();

            // Insert anchor_bytes → block_height into the anchors tree
            self.drive
                .grove
                .insert(
                    &anchors_path,
                    &current_anchor_bytes,
                    Element::new_item(block_height.to_be_bytes().to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(|e| Error::Drive(drive::error::Error::from(e)))?;

            // Update the most recent anchor
            self.drive
                .grove
                .insert(
                    &pool_path,
                    &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                    Element::new_item(current_anchor_bytes.to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(|e| Error::Drive(drive::error::Error::from(e)))?;
        }

        Ok(())
    }
}
