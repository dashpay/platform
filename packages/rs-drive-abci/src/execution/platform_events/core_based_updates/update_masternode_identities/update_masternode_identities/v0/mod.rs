use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dashcore_rpc::dashcore::ProTxHash;
use dashcore_rpc::dashcore_rpc_json::MasternodeListDiff;
use dashcore_rpc::json::MasternodeListItem;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;

use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::IdentityOperationType::AddNewIdentity;

use drive::grovedb::Transaction;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update of the masternode identities
    pub(super) fn update_masternode_identities_v0(
        &self,
        masternode_diff: MasternodeListDiff,
        removed_masternodes: &BTreeMap<ProTxHash, MasternodeListItem>,
        block_info: &BlockInfo,
        platform_state: Option<&PlatformState>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let MasternodeListDiff {
            mut added_mns,
            mut updated_mns,
            ..
        } = masternode_diff;

        // We should don't trust the order of added mns or updated mns

        // Sort added_mns based on pro_tx_hash
        added_mns.sort_by(|a, b| a.pro_tx_hash.cmp(&b.pro_tx_hash));

        // Sort updated_mns based on pro_tx_hash (the first element of the tuple)
        updated_mns.sort_by(|a, b| a.0.cmp(&b.0));

        let mut drive_operations = vec![];

        for masternode in added_mns {
            let owner_identity = Self::create_owner_identity(&masternode, platform_version)?;
            let voter_identity = Self::create_voter_identity_from_masternode_list_item(
                &masternode,
                platform_version,
            )?;
            let operator_identity = Self::create_operator_identity(&masternode, platform_version)?;

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: owner_identity,
            }));

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: voter_identity,
            }));

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: operator_identity,
            }));
        }

        if let Some(platform_state) = platform_state {
            // On initialization there is no platform state, but we also don't need to update
            // masternode identities.
            for update in updated_mns.iter() {
                self.update_owner_withdrawal_address(
                    update,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                self.update_voter_identity(
                    update,
                    block_info,
                    platform_state,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                self.update_operator_identity(
                    update,
                    block_info,
                    platform_state,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
            }

            for masternode in removed_masternodes.values() {
                self.disable_identity_keys(
                    masternode,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
            }
        }

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            Some(transaction),
            platform_version,
        )?;

        let height = block_info.height;
        tracing::debug!(height, "Updated masternode identities");

        Ok(())
    }
}
