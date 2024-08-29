use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::{IdentityPublicKey, KeyID};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetch all the current keys of every kind for a specific Identity
    #[inline(always)]
    pub(super) fn fetch_all_current_identity_keys_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_all_current_identity_keys_operations(
            identity_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Operations for fetching all the current keys of every kind for a specific Identity
    pub(super) fn fetch_all_current_identity_keys_operations_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request = IdentityKeysRequest::new_all_current_keys_query(identity_id);
        self.fetch_identity_keys_operations::<KeyIDIdentityPublicKeyPairBTreeMap>(
            key_request,
            transaction,
            drive_operations,
            platform_version,
        )
    }
}
