use std::collections::BTreeMap;
use grovedb::TransactionArg;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap};
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Fetch all the current keys of every kind for a specific Identity
    pub(super) fn fetch_all_current_identity_keys(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_all_current_identity_keys_operations(
            identity_id,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }

    /// Operations for fetching all the current keys of every kind for a specific Identity
    pub(super) fn fetch_all_current_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request = IdentityKeysRequest::new_all_current_keys_query(identity_id);
        self.fetch_identity_keys_operations::<KeyIDIdentityPublicKeyPairBTreeMap>(
            key_request,
            transaction,
            drive_operations,
            drive_version,
        )
    }
}