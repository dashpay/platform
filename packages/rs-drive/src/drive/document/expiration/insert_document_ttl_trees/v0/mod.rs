use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_LIFETIME_STORAGE_FEE_POOLS;
use crate::drive::credit_pools::paths::pools_path;
use crate::drive::document::expiration::paths::DOCUMENTS_EXPIRATIONS_KEY;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    #[inline(always)]
    pub(super) fn insert_document_ttl_trees_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // No storage flags: the trees are system structure, and every entry under them is
        // flagless.
        self.grove_insert_if_not_exists(
            (&misc_path()).into(),
            DOCUMENTS_EXPIRATIONS_KEY,
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        // The lifetime storage fee pools, a sum tree so `Pools` counts their credits.
        self.grove_insert_if_not_exists(
            (&pools_path()).into(),
            KEY_LIFETIME_STORAGE_FEE_POOLS,
            Element::empty_sum_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        Ok(())
    }
}
