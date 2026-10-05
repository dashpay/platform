use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dpp::block::block_info::BlockInfo;
use dpp::dashcore::{ProTxHash, PubkeyHash, ScriptBuf};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNPayout, MasternodeListDiff};
use dpp::dashcore_rpc::json::MasternodeListItem;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{KeyType, Purpose};

use dpp::version::PlatformVersion;

use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::util::batch::DriveOperation;
use drive::util::batch::DriveOperation::IdentityOperation;
use drive::util::batch::IdentityOperationType::{
    AddNewIdentity, AddNewKeysToIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};

use crate::platform_types::platform_state::PlatformStateV0Methods;

use dpp::dashcore::hashes::Hash;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;
use tracing::Level;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the masternode identities like version 0, except that an added masternode may
    /// have no owner identity.
    ///
    /// Version 0 is paired with `create_owner_identity` 0 and 1, which create an owner identity
    /// for every masternode or fail. `create_owner_identity` 2 creates none for a masternode
    /// without an owner address, such as a shared masternode, and this version then adds only
    /// its voter and operator identities. Legacy payout-address changes and removals follow
    /// version 0. Payout-list changes reconcile TRANSFER keys: only a sole supported P2PKH
    /// recipient retains withdrawal authority.
    pub(super) fn update_masternode_identities_v1(
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

        if updated_mns
            .iter()
            .any(|(_, diff)| diff.payout_address.is_some() && diff.payouts.is_some())
        {
            return Err(ExecutionError::DashCoreBadResponseError(
                "masternode diff contains both a payout address and a payout list".to_string(),
            )
            .into());
        }

        let mut drive_operations = vec![];

        for masternode in added_mns {
            let owner_identity = Self::create_owner_identity(&masternode, platform_version)?;

            tracing::trace!(
                identity = ?owner_identity,
                method = "update_masternode_identities_v1",
                "create owner identity"
            );

            let voter_identity = Self::create_voter_identity_from_masternode_list_item(
                &masternode,
                platform_version,
            )?;

            tracing::trace!(
                identity = ?voter_identity,
                method = "update_masternode_identities_v1",
                "create voter identity"
            );

            let operator_identity = Self::create_operator_identity(&masternode, platform_version)?;

            tracing::trace!(
                identity = ?operator_identity,
                method = "update_masternode_identities_v1",
                "create operator identity"
            );

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
                } else if let Some(payouts) = state_diff.payouts.as_deref() {
                    self.reconcile_owner_payout_keys(
                        pro_tx_hash.to_byte_array(),
                        payouts,
                        transaction,
                        &mut drive_operations,
                        platform_version,
                    )?;
                }

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

    /// A sole P2PKH recipient can spend the owner's balance. Other payout shapes must
    /// retire that authority without changing the OWNER key or the accumulated credits.
    fn reconcile_owner_payout_keys(
        &self,
        owner_identifier: [u8; 32],
        payouts: &[DMNPayout],
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let authorized_address = match payouts {
            [payout]
                if payout.script
                    == ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(payout.address)) =>
            {
                Some(payout.address)
            }
            _ => None,
        };
        let keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                IdentityKeysRequest {
                    identity_id: owner_identifier,
                    request_type: KeyRequestType::AllKeys,
                    limit: None,
                    offset: None,
                },
                Some(transaction),
                platform_version,
            )?;
        let Some((&last_key_id, _)) = keys.last_key_value() else {
            return Err(ExecutionError::DriveMissingData(
                "expected masternode owner identity to be in state".to_string(),
            )
            .into());
        };
        // Prefer an enabled matching key, then a disabled one; numeric IDs break ties.
        let reusable_key = authorized_address.and_then(|address| {
            keys.iter()
                .filter(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && key.data().as_slice() == address
                })
                .min_by_key(|(key_id, key)| (key.is_disabled(), **key_id))
        });
        let new_key = match (authorized_address, reusable_key) {
            (Some(address), None) => {
                let key_id = last_key_id.checked_add(1).ok_or(ExecutionError::Overflow(
                    "masternode owner identity key id exhausted",
                ))?;
                Some(Self::get_owner_identity_withdrawal_key(
                    address,
                    key_id,
                    platform_version,
                )?)
            }
            _ => None,
        };
        let keys_ids = keys
            .iter()
            .filter_map(|(key_id, key)| {
                (key.purpose() == Purpose::TRANSFER
                    && !key.is_disabled()
                    && reusable_key.map(|(reused_id, _)| reused_id) != Some(key_id))
                .then_some(*key_id)
            })
            .collect::<Vec<_>>();
        if !keys_ids.is_empty() {
            drive_operations.push(IdentityOperation(DisableIdentityKeys {
                identity_id: owner_identifier,
                keys_ids,
            }));
        }
        if let Some((key_id, key)) = reusable_key {
            if key.is_disabled() {
                drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                    identity_id: owner_identifier,
                    keys_ids: vec![*key_id],
                }));
            }
        }
        if let Some(key) = new_key {
            drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
                identity_id: owner_identifier,
                unique_keys_to_add: vec![],
                non_unique_keys_to_add: vec![key],
            }));
        }
        Ok(())
    }
}

#[cfg(test)]
mod payout_tests;

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNState, DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const PRO_TX_HASH: [u8; 32] = [0x61; 32];
    const VOTING_ADDRESS: [u8; 20] = [0x62; 20];
    const PUB_KEY_OPERATOR: [u8; 48] = [0x63; 48];

    /// A shared masternode as Core lists it: no owner, payout or collateral address.
    fn shared_masternode() -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
            collateral_hash: Txid::from_byte_array([0x64; 32]),
            collateral_index: 0,
            collateral_address: None,
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:9999".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: None,
                voting_address: VOTING_ADDRESS,
                payout_address: None,
                payouts: None,
                pub_key_operator: PUB_KEY_OPERATOR.to_vec(),
                operator_payout_address: None,
                platform_node_id: None,
                #[allow(deprecated)]
                legacy_platform_p2p_port: None,
                #[allow(deprecated)]
                legacy_platform_http_port: None,
                addresses: None,
            },
        }
    }

    fn masternode_list_diff(
        added_mns: Vec<MasternodeListItem>,
        removed_mns: Vec<ProTxHash>,
        updated_mns: Vec<(ProTxHash, DMNStateDiff)>,
    ) -> MasternodeListDiff {
        MasternodeListDiff {
            base_height: 0,
            block_height: 1,
            added_mns,
            removed_mns,
            updated_mns,
        }
    }

    /// A shared masternode has no owner address, so `create_owner_identity` gives it no owner
    /// identity. Its voter and operator identities do not depend on the owner and are added as
    /// for any masternode.
    #[test]
    fn should_add_the_voter_and_operator_identities_but_no_owner_identity_for_a_shared_masternode()
    {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_masternode_identities(
                masternode_list_diff(vec![shared_masternode()], vec![], vec![]),
                &BTreeMap::new(),
                &BlockInfo::default(),
                None,
                &transaction,
                platform_version,
            )
            .expect("expected to add the masternode identities");

        let identity_exists = |identity_id: Identifier| {
            platform
                .drive
                .fetch_full_identity(
                    identity_id.to_buffer(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to fetch an identity")
                .is_some()
        };
        assert!(
            !identity_exists(PRO_TX_HASH.into()),
            "a shared masternode has no owner identity"
        );
        assert!(
            identity_exists(Identifier::create_voter_identifier(
                &PRO_TX_HASH,
                &VOTING_ADDRESS
            )),
            "the voter identity must be added"
        );
        assert!(
            identity_exists(Identifier::create_operator_identifier(
                &PRO_TX_HASH,
                &PUB_KEY_OPERATOR
            )),
            "the operator identity must be added"
        );
    }

    /// Only a `payout_address` change reads the owner identity, and Core prints none for a
    /// shared masternode. A change of its voting address and operator key, and its removal,
    /// update its voter and operator identities as for any masternode and do not fail for want
    /// of an owner identity.
    #[test]
    fn should_update_and_remove_a_shared_masternode_without_an_owner_identity() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let masternode = shared_masternode();
        let pro_tx_hash = masternode.pro_tx_hash;
        platform
            .update_masternode_identities(
                masternode_list_diff(vec![masternode.clone()], vec![], vec![]),
                &BTreeMap::new(),
                &BlockInfo::default(),
                None,
                &transaction,
                platform_version,
            )
            .expect("expected to add the masternode identities");

        let mut platform_state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        platform_state.insert_masternode(masternode.clone());

        let new_voting_address = [0x65; 20];
        let new_pub_key_operator = [0x66; 48];
        let state_diff = DMNStateDiff {
            service: None,
            registered_height: None,
            last_paid_height: None,
            consecutive_payments: None,
            pose_penalty: None,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: None,
            owner_address: None,
            voting_address: Some(new_voting_address),
            payout_address: None,
            payouts: None,
            pub_key_operator: Some(new_pub_key_operator.to_vec()),
            operator_payout_address: None,
            platform_node_id: None,
            #[allow(deprecated)]
            legacy_platform_p2p_port: None,
            #[allow(deprecated)]
            legacy_platform_http_port: None,
            addresses: None,
        };
        platform
            .update_masternode_identities(
                masternode_list_diff(vec![], vec![], vec![(pro_tx_hash, state_diff.clone())]),
                &BTreeMap::new(),
                &BlockInfo::default(),
                Some(&platform_state),
                &transaction,
                platform_version,
            )
            .expect("expected to update the identities of the shared masternode");

        let mut updated_masternode = masternode;
        updated_masternode.state.apply_diff(state_diff);
        platform_state.insert_masternode(updated_masternode.clone());
        platform
            .update_masternode_identities(
                masternode_list_diff(vec![], vec![pro_tx_hash], vec![]),
                &BTreeMap::from([(pro_tx_hash, updated_masternode)]),
                &BlockInfo::default(),
                Some(&platform_state),
                &transaction,
                platform_version,
            )
            .expect("expected to remove the shared masternode");

        let fetch_identity = |identity_id: Identifier| -> Option<Identity> {
            platform
                .drive
                .fetch_full_identity(
                    identity_id.to_buffer(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to fetch an identity")
        };
        let has_an_enabled_key = |identity_id: Identifier| {
            fetch_identity(identity_id)
                .expect("expected the identity to exist")
                .public_keys()
                .values()
                .any(|key| !key.is_disabled())
        };
        assert_eq!(fetch_identity(PRO_TX_HASH.into()), None);
        assert!(
            !has_an_enabled_key(Identifier::create_voter_identifier(
                &PRO_TX_HASH,
                &new_voting_address
            )),
            "the removed masternode's voter identity keys must be disabled"
        );
        assert!(
            !has_an_enabled_key(Identifier::create_operator_identifier(
                &PRO_TX_HASH,
                &new_pub_key_operator
            )),
            "the removed masternode's operator identity keys must be disabled"
        );
    }
}
