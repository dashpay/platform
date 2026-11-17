mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dpp::dashcore::ProTxHash;

use dpp::block::block_info::BlockInfo;
use dpp::dashcore_rpc::json::DMNStateDiff;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the voter identity in the masternode.
    ///
    /// This function updates the voter identity in the masternode. The old identity key is
    /// disabled (which might make the identity unusable), and either a new identity is added
    /// with the new key, which is a non-unique key, or, when the voter identity of the new
    /// voting address already exists, a key is re-enabled.
    ///
    /// When the masternode returns to a voting address it used before, version 1 (from
    /// protocol version 15) re-enables the key of that address's voter identity. Version 0
    /// re-enables the key IDs on the voter identity it is leaving instead, so the returning
    /// voter key stays disabled.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode to be updated.
    /// * block_info - A reference to the block information.
    /// * platform_state - A reference to the platform state.
    /// * transaction - The current groveDB transaction.
    /// * drive_operations - A mutable reference to the Drive operations vector.
    /// * platform_version - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    ///   there is a problem updating the voter identity.
    pub(in crate::execution::platform_events::core_based_updates::update_masternode_identities) fn update_voter_identity(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .update_voter_identity
        {
            0 => self.update_voter_identity_v0(
                masternode,
                block_info,
                platform_state,
                transaction,
                drive_operations,
                platform_version,
            ),
            1 => self.update_voter_identity_v1(
                masternode,
                block_info,
                platform_state,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_voter_identity".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNState, DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    pub(super) const PRO_TX_HASH: [u8; 32] = [0x61; 32];

    /// Registers the masternode with `voting_address` through a masternode list update, which
    /// creates its voter identity, and lists it in the platform state.
    pub(super) fn register_masternode(
        platform: &TempPlatform<MockCoreRPCLike>,
        voting_address: [u8; 20],
        platform_version: &PlatformVersion,
    ) {
        let masternode = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
            collateral_hash: Txid::from_byte_array([0x62; 32]),
            collateral_index: 0,
            collateral_address: Some([0x63; 20]),
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:9999".parse().expect("service address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: Some([0x64; 20]),
                voting_address,
                payout_address: Some([0x65; 20]),
                payouts: None,
                pub_key_operator: vec![0x66; 48],
                operator_payout_address: None,
                platform_node_id: None,
                #[allow(deprecated)]
                legacy_platform_p2p_port: None,
                #[allow(deprecated)]
                legacy_platform_http_port: None,
                addresses: None,
            },
        };
        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_masternode_identities(
                MasternodeListDiff {
                    base_height: 0,
                    block_height: 1,
                    added_mns: vec![masternode.clone()],
                    removed_mns: vec![],
                    updated_mns: vec![],
                },
                &BTreeMap::new(),
                &BlockInfo::default_with_time(1_000),
                None,
                &transaction,
                platform_version,
            )
            .expect("register the masternode");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit the registration");
        let mut state = platform.state.load().as_ref().clone();
        state.insert_masternode(masternode);
        platform.state.store(Arc::new(state));
    }

    /// Moves the masternode to `voting_address` through a masternode list update, then applies
    /// the same diff to the platform state's masternode list, as block execution does.
    pub(super) fn move_voting_address(
        platform: &TempPlatform<MockCoreRPCLike>,
        voting_address: [u8; 20],
        platform_version: &PlatformVersion,
    ) {
        let pro_tx_hash = ProTxHash::from_byte_array(PRO_TX_HASH);
        let state_diff = DMNStateDiff {
            voting_address: Some(voting_address),
            ..serde_json::from_str("{}").expect("empty Core state diff")
        };
        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_masternode_identities(
                MasternodeListDiff {
                    base_height: 1,
                    block_height: 2,
                    added_mns: vec![],
                    removed_mns: vec![],
                    updated_mns: vec![(pro_tx_hash, state_diff.clone())],
                },
                &BTreeMap::new(),
                &BlockInfo::default_with_time(2_000),
                Some(&platform.state.load()),
                &transaction,
                platform_version,
            )
            .expect("move the voting address");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit the voting address move");
        let mut state = platform.state.load().as_ref().clone();
        assert!(state.apply_masternode_state_diff(&pro_tx_hash, &state_diff));
        platform.state.store(Arc::new(state));
    }

    /// Whether the key of the masternode's voter identity for `voting_address` is disabled.
    pub(super) fn voter_key_is_disabled(
        platform: &TempPlatform<MockCoreRPCLike>,
        voting_address: [u8; 20],
        platform_version: &PlatformVersion,
    ) -> bool {
        let voter_identifier: Identifier =
            Identifier::create_voter_identifier(&PRO_TX_HASH, &voting_address);
        let voter_identity = platform
            .drive
            .fetch_full_identity(voter_identifier.to_buffer(), None, platform_version)
            .expect("fetch the voter identity")
            .expect("the voter identity exists");
        assert_eq!(voter_identity.public_keys().len(), 1);
        let voter_key = voter_identity
            .public_keys()
            .get(&0)
            .expect("the voter key has ID 0");
        assert_eq!(voter_key.data().as_slice(), voting_address);
        voter_key.is_disabled()
    }

    /// The voter identity of each voting address a masternode has used keeps its own key, and
    /// only the identity of the current address may sign votes. Protocol version 14 replays
    /// version 0, which leaves the key of an address the masternode returns to disabled.
    #[test]
    fn should_enable_only_the_returning_voter_key_from_protocol_version_15() {
        let address_a = [0x67; 20];
        let address_b = [0x68; 20];

        for (platform_version, expected_a_disabled, expected_b_disabled) in [
            (
                PlatformVersion::get(14).expect("protocol version 14"),
                true,
                false,
            ),
            (PlatformVersion::latest(), false, true),
        ] {
            let protocol_version = platform_version.protocol_version;
            let platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            register_masternode(&platform, address_a, platform_version);
            move_voting_address(&platform, address_b, platform_version);
            assert!(
                voter_key_is_disabled(&platform, address_a, platform_version),
                "protocol version {protocol_version}: the key of the address left is disabled"
            );
            assert!(
                !voter_key_is_disabled(&platform, address_b, platform_version),
                "protocol version {protocol_version}: the key of a new address is enabled"
            );

            move_voting_address(&platform, address_a, platform_version);
            assert_eq!(
                (
                    voter_key_is_disabled(&platform, address_a, platform_version),
                    voter_key_is_disabled(&platform, address_b, platform_version),
                ),
                (expected_a_disabled, expected_b_disabled),
                "protocol version {protocol_version}: (returning key disabled, left key disabled)"
            );
        }
    }
}
