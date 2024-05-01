use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::{IdentityPublicKey, KeyID, IDENTITY_MAX_KEYS};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetch all the keys of every kind for a specific Identity
    #[inline(always)]
    pub(super) fn fetch_all_identity_keys_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_all_identity_keys_operations(
            identity_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Operations for fetching all the keys of every kind for a specific Identity
    #[inline(always)]
    pub(super) fn fetch_all_identity_keys_operations_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request =
            IdentityKeysRequest::new_all_keys_query(&identity_id, Some(IDENTITY_MAX_KEYS));
        self.fetch_identity_keys_operations(
            key_request,
            transaction,
            drive_operations,
            platform_version,
        )
    }
}
