use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;

use dpp::block::block_info::BlockInfo;
use dpp::dashcore_rpc::json::DMNStateDiff;

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
    ///
    /// Unlike version 0, when the voter identity of the new voting address already exists
    /// (the masternode returns to an address it used before), its keys are re-enabled on that
    /// identity. Version 0 re-enabled them on the voter identity it had just disabled, which
    /// left the returning voter key disabled and the masternode unable to vote.
    pub(super) fn update_voter_identity_v1(
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
            method = "update_voter_identity_v1",
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
                identity_id = ?new_voter_identity.id(),
                keys_ids = ?new_voter_identity_key_ids,
                method = "update_voter_identity_v1",
                "re-enable voter identity keys if they already exists and disabled"
            );

            drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                identity_id: new_voter_identity.id().to_buffer(),
                keys_ids: new_voter_identity_key_ids,
            }));
        } else {
            tracing::trace!(
                identity = ?new_voter_identity,
                method = "update_voter_identity_v1",
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

#[cfg(test)]
mod tests {
    use super::super::tests::{
        move_voting_address, register_masternode, voter_key_is_disabled, PRO_TX_HASH,
    };
    use crate::execution::validation::state_transition::tests::{
        create_dpns_identity_name_contest, perform_vote,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::IdentityPublicKey;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;

    /// A masternode that moves its voting address away and back votes with the key of the
    /// address it returned to, while the voter identity of the address it left stays disabled.
    #[tokio::test]
    async fn should_let_a_masternode_vote_after_it_returns_to_a_previous_voting_address() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let (contender, _, dpns_contract) = create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            7,
            "quantum",
            platform_version,
        )
        .await;

        let mut rng = StdRng::seed_from_u64(61);
        let (voting_key, voting_private_key) =
            IdentityPublicKey::random_voting_key_with_rng(0, &mut rng, platform_version)
                .expect("voting key pair");
        let mut signer = SimpleSigner::default();
        signer.add_identity_public_key(voting_key.clone(), voting_private_key);
        let address_a = voting_key.public_key_hash().expect("voting address");
        let address_b = [0x68; 20];

        register_masternode(&platform, address_a, platform_version);
        move_voting_address(&platform, address_b, platform_version);
        move_voting_address(&platform, address_a, platform_version);

        assert!(
            !voter_key_is_disabled(&platform, address_a, platform_version),
            "the key of the address the masternode returned to is enabled"
        );
        assert!(
            voter_key_is_disabled(&platform, address_b, platform_version),
            "the key of the address the masternode left is disabled"
        );

        let platform_state = platform.state.load();
        perform_vote(
            &mut platform,
            &platform_state,
            dpns_contract.as_ref(),
            ResourceVoteChoice::TowardsIdentity(contender.id()),
            "quantum",
            &signer,
            PRO_TX_HASH.into(),
            &voting_key,
            1,
            None,
            platform_version,
        )
        .await;
    }
}
