use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;

use dashcore_rpc::json::DMNStateDiff;
use dpp::block::block_info::BlockInfo;

use dpp::identity::accessors::IdentityGettersV0;

use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDVec, KeyRequestType};
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;
use drive::util::batch::DriveOperation::IdentityOperation;
use drive::util::batch::IdentityOperationType::{
    AddNewIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// When a voter identity is updated the following events need to happen
    /// The old identity key is disabled (which might make the identity unusable)
    /// A new identity is added with the new key, this new key is a non unique key.
    pub(super) fn update_voter_identity_v0(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let (pro_tx_hash, state_diff) = masternode;
        let Some(new_voting_address) = state_diff.voting_address else {
            return Ok(());
        };

        let old_masternode = platform_state
            .full_masternode_list()
            .get(pro_tx_hash)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCachedState(format!(
                    "expected masternode {} to be in state",
                    pro_tx_hash
                )))
            })?;

        let old_voter_identifier =
            Self::get_voter_identifier_from_masternode_list_item(old_masternode, platform_version)?;

        let key_request = IdentityKeysRequest {
            identity_id: old_voter_identifier.to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let old_voter_identity_key_ids = self.drive.fetch_identity_keys::<KeyIDVec>(
            key_request,
            Some(transaction),
            platform_version,
        )?;

        if old_voter_identity_key_ids.is_empty() {
            return Err(Error::Execution(ExecutionError::DriveMissingData(
                "expected masternode voter identity to be in state".to_string(),
            )));
        }

        tracing::trace!(
            identity_id = ?old_voter_identifier,
            keys_ids = ?old_voter_identity_key_ids,
            disable_at = ?block_info.time_ms,
            method = "update_voter_identity_v0",
            "disable all voter identity keys"
        );

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: old_voter_identifier.to_buffer(),
            keys_ids: old_voter_identity_key_ids,
        }));

        // Part 2 : Create or Update Voting identity based on new key
        let new_voter_identity = Self::create_voter_identity(
            &(pro_tx_hash.to_byte_array()),
            &new_voting_address,
            platform_version,
        )?;

        // Let's check if the voting identity already exists
        let key_request = IdentityKeysRequest {
            identity_id: new_voter_identity.id().to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };
        let new_voter_identity_key_ids = self.drive.fetch_identity_keys::<KeyIDVec>(
            key_request,
            Some(transaction),
            platform_version,
        )?;

        // two possibilities
        if !new_voter_identity_key_ids.is_empty() {
            // first is that the new voter key already existed
            // if it is disabled re-enable it

            if new_voter_identity_key_ids.len() > 1 {
                return Err(Error::Execution(ExecutionError::DriveIncoherence(
                    "more than one masternode voter identity for an address and pro_tx_hash pair",
                )));
            }

            tracing::trace!(
                identity_id = ?old_voter_identifier,
                keys_ids = ?new_voter_identity_key_ids,
                method = "update_voter_identity_v0",
                "re-enable voter identity keys if they already exists and disabled"
            );

            drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                identity_id: old_voter_identifier.into_buffer(),
                keys_ids: new_voter_identity_key_ids,
            }));
        } else {
            tracing::trace!(
                identity = ?new_voter_identity,
                method = "update_voter_identity_v0",
                "create a new voter identity"
            );

            // other is that the
            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: new_voter_identity,
                is_masternode_identity: true,
            }));
        }
        Ok(())
    }
}
