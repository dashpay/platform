use crate::drive::document::expiration::paths::DOCUMENTS_EXPIRATIONS_KEY;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    #[inline(always)]
    pub(super) fn insert_documents_expirations_tree_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // No storage flags: the tree is system structure, and every entry under it is flagless.
        self.grove_insert_if_not_exists(
            (&misc_path()).into(),
            DOCUMENTS_EXPIRATIONS_KEY,
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        Ok(())
    }
}
