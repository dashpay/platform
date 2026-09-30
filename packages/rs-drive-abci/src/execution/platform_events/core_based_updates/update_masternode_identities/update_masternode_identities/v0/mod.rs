use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dpp::block::block_info::BlockInfo;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListDiff;
use dpp::dashcore_rpc::json::MasternodeListItem;

use dpp::version::PlatformVersion;

use drive::util::batch::DriveOperation::IdentityOperation;
use drive::util::batch::IdentityOperationType::AddNewIdentity;

use crate::platform_types::platform_state::PlatformStateV0Methods;

use dpp::dashcore::hashes::Hash;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;
use tracing::Level;

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

        let span = tracing::span!(Level::TRACE, "update_masternode_identities");
        let _enter = span.enter();

        // We should don't trust the order of added mns or updated mns

        // Sort added_mns based on pro_tx_hash
        added_mns.sort_by_key(|mn| mn.pro_tx_hash);

        // Sort updated_mns based on pro_tx_hash (the first element of the tuple)
        updated_mns.sort_by_key(|mn| mn.0);

        let mut drive_operations = vec![];

        for masternode in added_mns {
            let owner_identity = Self::create_owner_identity(&masternode, platform_version)?;

            tracing::trace!(
                identity = ?owner_identity,
                method = "update_masternode_identities_v0",
                "create owner identity"
            );

            let voter_identity = Self::create_voter_identity_from_masternode_list_item(
                &masternode,
                platform_version,
            )?;

            tracing::trace!(
                identity = ?voter_identity,
                method = "update_masternode_identities_v0",
                "create voter identity"
            );

            let operator_identity = Self::create_operator_identity(&masternode, platform_version)?;

            tracing::trace!(
                identity = ?operator_identity,
                method = "update_masternode_identities_v0",
                "create operator identity"
            );

            // A masternode with none of the keys an owner identity holds, such as a shared
            // masternode, has no owner identity. Only an added masternode without an owner
            // address, or with `payouts` instead of `payoutAddress`, gets none, and no earlier
            // binary can deserialize a masternode list holding one, so no committed block
            // skipped an owner identity here.
            if let Some(owner_identity) = owner_identity {
                drive_operations.push(IdentityOperation(AddNewIdentity {
                    identity: owner_identity,
                    is_masternode_identity: true,
                }));
            }

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: voter_identity,
                is_masternode_identity: true,
            }));

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: operator_identity,
                is_masternode_identity: true,
            }));
        }

        if let Some(platform_state) = platform_state {
            // On initialization there is no platform state, but we also don't need to update
            // masternode identities.
            for update in updated_mns.iter() {
                let (pro_tx_hash, state_diff) = update;
                if let Some(new_withdrawal_address) = state_diff.payout_address {
                    let owner_identifier: [u8; 32] = pro_tx_hash.to_byte_array();
                    self.update_owner_withdrawal_address(
                        owner_identifier,
                        new_withdrawal_address,
                        block_info,
                        transaction,
                        &mut drive_operations,
                        platform_version,
                    )?;
                };

                self.update_voter_identity(
                    update,
                    block_info,
                    platform_state,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                if state_diff.platform_node_id.is_some()
                    || state_diff.operator_payout_address.is_some()
                    || state_diff.pub_key_operator.is_some()
                {
                    self.update_operator_identity(
                        pro_tx_hash,
                        state_diff.pub_key_operator.as_ref(),
                        state_diff.operator_payout_address,
                        state_diff.platform_node_id,
                        platform_state,
                        transaction,
                        &mut drive_operations,
                        platform_version,
                    )?;
                }
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

        let previous_fee_versions = platform_state.map(|state| state.previous_fee_versions());
        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            Some(transaction),
            platform_version,
            previous_fee_versions,
        )?;

        let height = block_info.height;
        tracing::debug!(height, "Updated masternode identities");

        Ok(())
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
    /// never as `payout_address`. Only a `payout_address` change rotates the owner identity's
    /// TRANSFER key; acting on `payouts` as well would change what a block does to owner
    /// identities, which needs a new method version.
    #[test]
    fn should_not_change_owner_identity_keys_on_a_payout_list_change() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
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
            platform_version.protocol_version,
            platform_version.protocol_version,
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
        assert_eq!(owner_identity_keys(), keys_before);

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
            assert_eq!(owner_identity_keys(), keys_before, "payouts {payouts:?}");
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
            ])
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
