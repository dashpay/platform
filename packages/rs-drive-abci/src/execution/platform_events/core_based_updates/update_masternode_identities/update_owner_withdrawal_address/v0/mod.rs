use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyID;

use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;
use drive::util::batch::DriveOperation::IdentityOperation;
use drive::util::batch::IdentityOperationType::{
    AddNewKeysToIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};
impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn update_owner_withdrawal_address_v0(
        &self,
        owner_identifier: [u8; 32],
        new_withdrawal_address: [u8; 20],
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let key_request = IdentityKeysRequest {
            identity_id: owner_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let old_withdrawal_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                key_request,
                Some(transaction),
                platform_version,
            )?;

        if old_withdrawal_identity_keys.is_empty() {
            return Err(Error::Execution(ExecutionError::DriveMissingData(
                "expected masternode owner identity to be in state".to_string(),
            )));
        }

        let key_ids_to_disable: Vec<KeyID> = old_withdrawal_identity_keys
            .iter()
            .filter_map(|(key_id, key)| {
                if key.disabled_at().is_some() || key.data().as_slice() == new_withdrawal_address {
                    None //No need to disable it again or if we are adding the same key we already had
                } else {
                    Some(*key_id)
                }
            })
            .collect();

        if !key_ids_to_disable.is_empty() {
            tracing::trace!(
                identity_id = ?owner_identifier,
                keys_ids = ?key_ids_to_disable,
                disable_at = ?block_info.time_ms,
                method = "update_owner_withdrawal_address_v0",
                "disable old withdrawal keys in owner identity"
            );

            drive_operations.push(IdentityOperation(DisableIdentityKeys {
                identity_id: owner_identifier,
                keys_ids: key_ids_to_disable,
            }));
        }

        if let Some((key_id, previously_disabled_old_key)) = old_withdrawal_identity_keys
            .iter()
            .find(|(_, key)| key.data().as_slice() == new_withdrawal_address)
        {
            // there might be a situation where we should do nothing as well
            if previously_disabled_old_key.is_disabled() {
                // We need to re-enable the withdrawal key
                tracing::trace!(
                    identity_id = ?owner_identifier,
                    withdrawal_key = ?previously_disabled_old_key,
                    method = "update_owner_withdrawal_address_v0",
                    "re-enabled withdrawal key to owner identity"
                );

                drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                    identity_id: owner_identifier,
                    keys_ids: vec![*key_id],
                }));
            }
        } else {
            let last_key_id = *old_withdrawal_identity_keys.keys().max().unwrap(); //todo

            // add the new key
            let new_owner_withdrawal_key = Self::get_owner_identity_withdrawal_key(
                new_withdrawal_address,
                last_key_id + 1,
                platform_version,
            )?;

            tracing::trace!(
                identity_id = ?owner_identifier,
                withdrawal_key = ?new_owner_withdrawal_key,
                method = "update_owner_withdrawal_address_v0",
                "add new withdrawal key to owner identity"
            );

            drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
                identity_id: owner_identifier,
                unique_keys_to_add: vec![],
                non_unique_keys_to_add: vec![new_owner_withdrawal_key],
            }));
        }

        Ok(())
    }
}
