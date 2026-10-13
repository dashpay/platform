use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::{MasternodeListDiff, MasternodeListItem};
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;

mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update of the masternode identities
    ///
    /// Version 0 adds an owner identity for every added masternode. Version 1 (from protocol
    /// version 14) adds none for a masternode `create_owner_identity` gives none, such as a
    /// shared masternode.
    pub(in crate::execution) fn update_masternode_identities(
        &self,
        masternode_diff: MasternodeListDiff,
        removed_masternodes: &BTreeMap<ProTxHash, MasternodeListItem>,
        block_info: &BlockInfo,
        platform_state: Option<&PlatformState>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .update_masternode_identities
        {
            0 => self.update_masternode_identities_v0(
                masternode_diff,
                removed_masternodes,
                block_info,
                platform_state,
                transaction,
                platform_version,
            ),
            1 => self.update_masternode_identities_v1(
                masternode_diff,
                removed_masternodes,
                block_info,
                platform_state,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_masternode_identities".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, ProTxHash, PubkeyHash, ScriptBuf, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNPayout, DMNState, DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyID, Purpose};
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// A payout change of an extended-address masternode reaches Drive only as `payouts`,
    /// never as `payout_address`. The historical updater leaves these keys unchanged.
    /// Protocol version 13 is pinned to preserve replay of its payout-list policy.
    #[test]
    fn should_not_change_owner_identity_keys_on_a_payout_list_change() {
        assert_owner_identity_keys_unchanged_by_payout_list_changes(
            PlatformVersion::get(13).expect("expected protocol version 13"),
        );
    }

    fn assert_owner_identity_keys_unchanged_by_payout_list_changes(
        platform_version: &PlatformVersion,
    ) {
        let protocol_version = platform_version.protocol_version;
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let pro_tx_hash = ProTxHash::from_byte_array([0x51; 32]);
        let owner_address = [0x52; 20];
        let payout_address = [0x53; 20];
        let masternode = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0x54; 32]),
            collateral_index: 0,
            collateral_address: Some([0x55; 20]),
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:9999".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: Some(owner_address),
                voting_address: [0x56; 20],
                payout_address: Some(payout_address),
                payouts: None,
                pub_key_operator: vec![0x57; 48],
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
                &BlockInfo::default(),
                None,
                &transaction,
                platform_version,
            )
            .expect("expected to create the masternode identities");

        let mut platform_state = PlatformState::default_with_protocol_versions(
            protocol_version,
            protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        platform_state.insert_masternode(masternode);

        let apply_update = |state_diff: DMNStateDiff| {
            platform
                .update_masternode_identities(
                    MasternodeListDiff {
                        base_height: 1,
                        block_height: 2,
                        added_mns: vec![],
                        removed_mns: vec![],
                        updated_mns: vec![(pro_tx_hash, state_diff)],
                    },
                    &BTreeMap::new(),
                    &BlockInfo::default(),
                    Some(&platform_state),
                    &transaction,
                    platform_version,
                )
                .expect("expected to process the masternode update");
        };
        let owner_identity_keys = || -> BTreeMap<KeyID, (Purpose, Vec<u8>, bool)> {
            platform
                .drive
                .fetch_full_identity(
                    pro_tx_hash.to_byte_array(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to fetch the owner identity")
                .expect("expected the owner identity to exist")
                .public_keys()
                .iter()
                .map(|(key_id, key)| {
                    (
                        *key_id,
                        (key.purpose(), key.data().to_vec(), key.is_disabled()),
                    )
                })
                .collect()
        };
        let keys_before = BTreeMap::from([
            (0, (Purpose::TRANSFER, payout_address.to_vec(), false)),
            (1, (Purpose::OWNER, owner_address.to_vec(), false)),
        ]);
        assert_eq!(
            owner_identity_keys(),
            keys_before,
            "protocol version {protocol_version}"
        );

        let payout = |address: [u8; 20], reward: u16| DMNPayout {
            address,
            script: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(address)),
            reward,
        };
        for payouts in [
            // The payout address moved into a list, as Core does when a masternode moves to
            // extended addresses.
            vec![payout(payout_address, 10000)],
            // Another single payout address.
            vec![payout([0x58; 20], 10000)],
            // Several payouts.
            vec![payout([0x58; 20], 2500), payout([0x59; 20], 7500)],
        ] {
            apply_update(DMNStateDiff {
                payouts: Some(payouts.clone()),
                ..empty_state_diff()
            });
            assert_eq!(
                owner_identity_keys(),
                keys_before,
                "protocol version {protocol_version}, payouts {payouts:?}"
            );
        }

        // The same processing does rotate the TRANSFER key on a payout address change.
        apply_update(DMNStateDiff {
            payout_address: Some([0x5a; 20]),
            ..empty_state_diff()
        });
        assert_eq!(
            owner_identity_keys(),
            BTreeMap::from([
                (0, (Purpose::TRANSFER, payout_address.to_vec(), true)),
                (1, (Purpose::OWNER, owner_address.to_vec(), false)),
                (2, (Purpose::TRANSFER, vec![0x5a; 20], false)),
            ]),
            "protocol version {protocol_version}"
        );
    }

    fn empty_state_diff() -> DMNStateDiff {
        DMNStateDiff {
            service: None,
            registered_height: None,
            last_paid_height: None,
            consecutive_payments: None,
            pose_penalty: None,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: None,
            owner_address: None,
            voting_address: None,
            payout_address: None,
            payouts: None,
            pub_key_operator: None,
            operator_payout_address: None,
            platform_node_id: None,
            #[allow(deprecated)]
            legacy_platform_p2p_port: None,
            #[allow(deprecated)]
            legacy_platform_http_port: None,
            addresses: None,
        }
    }
}
