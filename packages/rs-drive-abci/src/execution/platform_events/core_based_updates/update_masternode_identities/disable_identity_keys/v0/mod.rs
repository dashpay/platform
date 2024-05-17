use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::block::block_info::BlockInfo;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Purpose::TRANSFER;
use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::IdentityOperationType::DisableIdentityKeys;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn disable_identity_keys_v0(
        &self,
        old_masternode: &MasternodeListItem,
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        tracing::trace!(
            pro_tx_hash = old_masternode.pro_tx_hash.to_hex(),
            method = "disable_identity_keys_v0",
            "masternode is removed. operator and voter keys should be disabled"
        );

        let operator_identifier = Self::get_operator_identifier_from_masternode_list_item(
            old_masternode,
            platform_version,
        )?;
        let voter_identifier =
            Self::get_voter_identifier_from_masternode_list_item(old_masternode, platform_version)?;

        let operator_key_request = IdentityKeysRequest {
            identity_id: operator_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let voter_key_request = IdentityKeysRequest {
            identity_id: voter_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let operator_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
                operator_key_request,
                Some(transaction),
                platform_version,
            )?
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.is_disabled() || key.purpose() == TRANSFER {
                    None // Don't disable withdrawal keys
                } else {
                    Some(key_id)
                }
            })
            .collect();
        let voter_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
                voter_key_request,
                Some(transaction),
                platform_version,
            )?
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.is_disabled() {
                    None //No need to disable it again
                } else {
                    Some(key_id)
                }
            })
            .collect();

        tracing::trace!(
            identity_id = ?operator_identifier,
            keys_ids = ?operator_identity_keys,
            disable_at = ?block_info.time_ms,
            method = "disable_identity_keys_v0",
            "disable all operator identity keys for removed masternode"
        );

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: operator_identifier,
            keys_ids: operator_identity_keys,
        }));

        tracing::trace!(
            identity_id = ?voter_identifier,
            keys_ids = ?voter_identity_keys,
            disable_at = ?block_info.time_ms,
            method = "disable_identity_keys_v0",
            "disable all voter identity key for removed masternode"
        );

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: voter_identifier,
            keys_ids: voter_identity_keys,
        }));

        Ok(())
    }
}
