use crate::drive::shielded::paths::{
    shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
    shielded_credit_pool_path, SHIELDED_MOST_RECENT_ANCHOR_KEY, SHIELDED_NOTES_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, Transaction};

impl Drive {
    /// Version 0 implementation of recording the shielded pool anchor.
    ///
    /// Reads the current Sinsemilla anchor from the CommitmentTree at
    /// `[AddressBalances, "s", [1]]`. If it differs from the most recent
    /// anchor (stored at `[AddressBalances, "s", [7]]`), inserts:
    /// - `anchor_bytes -> block_height.to_be_bytes()` into anchors tree `[..., [6]]`
    /// - `block_height.to_be_bytes() -> anchor_bytes` into anchors-by-height tree `[..., [8]]`
    /// - Updates the most recent anchor item
    pub(in crate::drive) fn record_shielded_pool_anchor_if_changed_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let grove_version = &platform_version.drive.grove_version;
        let pool_path = shielded_credit_pool_path();

        // 1. Read current anchor from CommitmentTree
        let current_anchor = self
            .grove
            .commitment_tree_anchor(
                &pool_path,
                &[SHIELDED_NOTES_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(Error::from)?;

        let current_anchor_bytes: [u8; 32] = current_anchor.to_bytes();

        // 2. Read most recent anchor from the dedicated element
        let most_recent_anchor: [u8; 32] = self
            .grove
            .get(
                &pool_path,
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(Error::from)
            .and_then(|element| {
                if let Element::Item(value, _) = element {
                    value.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "most recent anchor is not 32 bytes",
                        ))
                    })
                } else {
                    Err(Error::Drive(DriveError::CorruptedElementType(
                        "most recent anchor element is not an Item",
                    )))
                }
            })?;

        // 3. Only store if different (skip zero anchor from empty tree)
        let should_store =
            current_anchor_bytes != most_recent_anchor && current_anchor_bytes != [0u8; 32];

        if should_store {
            let anchors_path = shielded_credit_pool_anchors_path();

            // Insert anchor_bytes -> block_height into the anchors tree
            self.grove
                .insert(
                    &anchors_path,
                    &current_anchor_bytes,
                    Element::new_item(block_height.to_be_bytes().to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;

            // Insert block_height -> anchor_bytes into the anchors-by-height tree (for pruning)
            let anchors_by_height_path = shielded_credit_pool_anchors_by_height_path();
            self.grove
                .insert(
                    &anchors_by_height_path,
                    &block_height.to_be_bytes(),
                    Element::new_item(current_anchor_bytes.to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;

            // Update the most recent anchor
            self.grove
                .insert(
                    &pool_path,
                    &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                    Element::new_item(current_anchor_bytes.to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;
        }

        Ok(())
    }
}
