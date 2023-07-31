use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::ProTxHash;
use dashcore_rpc::dashcore_rpc_json::MasternodeListDiff;
use dashcore_rpc::json::{DMNStateDiff, MasternodeListItem};
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::identity_factory::IDENTITY_PROTOCOL_VERSION;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Purpose::WITHDRAW;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::IdentityOperationType::{
    AddNewIdentity, AddNewKeysToIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyIDIdentityPublicKeyPairVec,
    KeyIDVec, KeyRequestType,
};
use drive::grovedb::Transaction;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

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

        let owner_identifier: [u8; 32] = pro_tx_hash.into_inner();

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

        let key_ids_to_disable = old_withdrawal_identity_keys
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.disabled_at().is_some() {
                    None //No need to disable it again
                } else {
                    Some(key_id)
                }
            })
            .collect();

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: owner_identifier,
            keys_ids: key_ids_to_disable,
            disable_at: block_info.time_ms,
        }));

        // add the new key
        let new_owner_key = Self::get_owner_identity_key(
            new_withdrawal_address,
            last_key_id + 1,
            platform_version,
        )?;

        drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
            identity_id: owner_identifier,
            unique_keys_to_add: vec![],
            non_unique_keys_to_add: vec![new_owner_key],
        }));

        Ok(())
    }
}
