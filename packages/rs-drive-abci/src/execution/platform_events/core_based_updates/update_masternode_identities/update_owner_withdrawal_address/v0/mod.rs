use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::ProTxHash;

use dashcore_rpc::json::DMNStateDiff;
use dpp::block::block_info::BlockInfo;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyID;

use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::IdentityOperationType::{AddNewKeysToIdentity, DisableIdentityKeys};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::grovedb::Transaction;
impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn update_owner_withdrawal_address_v0(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let (pro_tx_hash, state_diff) = masternode;
        let Some(new_withdrawal_address) = state_diff.payout_address else {
            return Ok(());
        };

        let owner_identifier: [u8; 32] = pro_tx_hash.to_byte_array();

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

        let last_key_id = *old_withdrawal_identity_keys.keys().max().unwrap(); //todo

        let key_ids_to_disable: Vec<KeyID> = old_withdrawal_identity_keys
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.disabled_at().is_some() {
                    None //No need to disable it again
                } else {
                    Some(key_id)
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

        // add the new key
        let new_owner_key = Self::get_owner_identity_key(
            new_withdrawal_address,
            last_key_id + 1,
            platform_version,
        )?;

        tracing::trace!(
            identity_id = ?owner_identifier,
            withdrawal_key = ?new_owner_key,
            method = "update_owner_withdrawal_address_v0",
            "add new withdrawal key to owner identity"
        );

        drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
            identity_id: owner_identifier,
            unique_keys_to_add: vec![],
            non_unique_keys_to_add: vec![new_owner_key],
        }));

        Ok(())
    }
}
