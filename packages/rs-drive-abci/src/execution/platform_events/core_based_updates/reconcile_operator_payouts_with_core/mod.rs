use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNStateDiff, MasternodeListDiff};
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Makes the stored operator payouts, and the TRANSFER keys of each masternode's current
    /// operator identity, match Dash Core's masternode list at `core_height`. Runs once, in the
    /// first block of protocol version 15.
    ///
    /// Before protocol version 15 Platform took an operator payout only from a masternode's
    /// full state, when it first listed the masternode, so a payout set or changed in Core later
    /// never reached Platform, and a new operator inherited the previous operator's payout key.
    /// From version 15 payouts follow Core's diffs; this fixes what earlier versions left. In
    /// their history the enabled TRANSFER keys of a masternode's operator identity are exactly
    /// its stored payout, so comparing the stored payout with Core's finds every identity to
    /// fix. Each one goes through `update_operator_identity` with Core's payout, which reconciles
    /// all of the identity's TRANSFER keys with it. An identity a masternode already left is
    /// reconciled only if the masternode switches back to it.
    ///
    /// `block_platform_state` must hold Platform's copy of Core's list at `core_height`, a chain
    /// locked height every node agrees on, so every node reads the same list from its Core. A
    /// Core error, or a Core list whose masternodes are not the ones Platform lists, fails the
    /// block: both mean that this node cannot tell what the others reconcile.
    pub(in crate::execution) fn reconcile_operator_payouts_with_core(
        &self,
        block_platform_state: &mut PlatformState,
        core_height: u32,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // From the first block, so every masternode at `core_height` is in `added_mns`.
        let core_list = self
            .core_rpc
            .get_protx_diff_with_masternodes(None, core_height)
            .inspect_err(|error| {
                tracing::error!(
                    core_height,
                    ?error,
                    "could not read Dash Core's masternode list to reconcile operator payouts"
                )
            })?;
        if !core_list.removed_mns.is_empty() || !core_list.updated_mns.is_empty() {
            return Err(ExecutionError::DashCoreBadResponseError(format!(
                "the masternode list from the first block to core height {core_height} removes or updates masternodes"
            ))
            .into());
        }
        let core_payouts = core_list
            .added_mns
            .into_iter()
            .map(|masternode| {
                (
                    masternode.pro_tx_hash,
                    masternode.state.operator_payout_address,
                )
            })
            .collect::<BTreeMap<ProTxHash, Option<[u8; 20]>>>();

        let stored_list = block_platform_state.full_masternode_list();
        if !stored_list.keys().eq(core_payouts.keys()) {
            let missing_in_core = stored_list
                .keys()
                .filter(|pro_tx_hash| !core_payouts.contains_key(*pro_tx_hash))
                .count();
            let missing_in_platform = core_payouts
                .keys()
                .filter(|pro_tx_hash| !stored_list.contains_key(*pro_tx_hash))
                .count();
            return Err(ExecutionError::CorruptedCachedState(format!(
                "Platform's masternode list at core height {core_height} is not Dash Core's: \
                 {missing_in_core} listed masternodes are not in Core's list and \
                 {missing_in_platform} of Core's are not listed"
            ))
            .into());
        }

        // Both lists hold the same masternodes in the same order.
        let corrections = stored_list
            .values()
            .zip(core_payouts.values())
            .filter(|(masternode, core_payout)| {
                masternode.state.operator_payout_address != **core_payout
            })
            .map(|(masternode, core_payout)| (masternode.clone(), *core_payout))
            .collect::<Vec<_>>();

        tracing::info!(
            core_height,
            masternodes = core_payouts.len(),
            corrected = corrections.len(),
            "reconciled the operator payouts of the masternode list with Dash Core"
        );
        if corrections.is_empty() {
            return Ok(());
        }

        let mut updated_mns = Vec::with_capacity(corrections.len());
        for (mut masternode, core_payout) in corrections {
            masternode.state.operator_payout_address = core_payout;
            updated_mns.push((masternode.pro_tx_hash, operator_payout_diff(core_payout)));
            block_platform_state.insert_masternode(masternode);
        }

        // The corrected list is the state the identity updates read: its operator keys name the
        // current operator identities.
        self.update_masternode_identities(
            MasternodeListDiff {
                base_height: core_height,
                block_height: core_height,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns,
            },
            &BTreeMap::new(),
            block_info,
            Some(&*block_platform_state),
            transaction,
            platform_version,
        )
    }
}

/// A Core state diff that sets only the operator payout.
fn operator_payout_diff(operator_payout_address: Option<[u8; 20]>) -> DMNStateDiff {
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
        operator_payout_address: Some(operator_payout_address),
        platform_node_id: None,
        #[allow(deprecated)]
        legacy_platform_p2p_port: None,
        #[allow(deprecated)]
        legacy_platform_http_port: None,
        addresses: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNState, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::dashcore_rpc::Error as CoreRpcError;
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::Purpose;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use drive::grovedb::Transaction;
    use std::collections::BTreeSet;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    /// The Core height the masternodes are first listed at, and the one reconciled at.
    const LISTED_AT: u32 = 1000;
    const RECONCILED_AT: u32 = 1001;

    /// A Regular masternode named by `id` with an operator reward and `operator_payout`.
    fn masternode(id: u8, operator_payout: Option<u8>) -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array([id; 32]),
            collateral_hash: Txid::from_byte_array([id ^ 0x40; 32]),
            collateral_index: 0,
            collateral_address: Some([id ^ 0x50; 20]),
            operator_reward: 10.0,
            state: DMNState {
                service: SocketAddr::from(([192, 0, 2, id], 9999)),
                registered_height: 900,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: Some([id ^ 0x10; 20]),
                voting_address: [id ^ 0x20; 20],
                payout_address: Some([id ^ 0x30; 20]),
                payouts: None,
                pub_key_operator: vec![id; 48],
                operator_payout_address: operator_payout.map(|payout| [payout; 20]),
                platform_node_id: None,
                #[allow(deprecated)]
                legacy_platform_p2p_port: None,
                #[allow(deprecated)]
                legacy_platform_http_port: None,
                addresses: None,
            },
        }
    }

    type CoreCalls = Arc<Mutex<Vec<(Option<u32>, u32)>>>;

    /// A platform at the latest protocol version whose Core lists `listed` at `LISTED_AT` and
    /// answers `core_list` for its list at `RECONCILED_AT`, once. Returns the masternode list
    /// calls made to Core at `RECONCILED_AT`.
    fn platform_with_core(
        listed: Vec<MasternodeListItem>,
        core_list: Result<Vec<MasternodeListItem>, CoreRpcError>,
    ) -> (TempPlatform<MockCoreRPCLike>, CoreCalls) {
        let mut platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc()
            .set_genesis_state();
        let listed_diff = MasternodeListDiff {
            base_height: 1,
            block_height: LISTED_AT,
            added_mns: listed,
            removed_mns: vec![],
            updated_mns: vec![],
        };
        let core_list = Mutex::new(Some(core_list.map(|added_mns| MasternodeListDiff {
            base_height: 1,
            block_height: RECONCILED_AT,
            added_mns,
            removed_mns: vec![],
            updated_mns: vec![],
        })));
        let calls = CoreCalls::default();
        let recorded_calls = Arc::clone(&calls);
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                if block == LISTED_AT {
                    return Ok(listed_diff.clone());
                }
                recorded_calls
                    .lock()
                    .expect("expected the Core calls")
                    .push((base_block, block));
                core_list
                    .lock()
                    .expect("expected Core's list")
                    .take()
                    .expect("expected one masternode list call at the reconciled height")
            });
        (platform, calls)
    }

    /// Lists the masternodes at `LISTED_AT` as init chain does, creating their identities.
    /// Returns the platform state, with nothing pending to store.
    fn list_masternodes(
        platform: &TempPlatform<MockCoreRPCLike>,
        transaction: &Transaction,
    ) -> PlatformState {
        let mut state = platform.state.load().as_ref().clone();
        platform
            .update_masternode_list(
                None,
                &mut state,
                LISTED_AT,
                true,
                &BlockInfo {
                    height: 1,
                    core_height: LISTED_AT,
                    ..Default::default()
                },
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to list the masternodes");
        state.mark_saved();
        state
    }

    fn reconcile(
        platform: &TempPlatform<MockCoreRPCLike>,
        state: &mut PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        platform.reconcile_operator_payouts_with_core(
            state,
            RECONCILED_AT,
            &BlockInfo {
                height: 2,
                core_height: RECONCILED_AT,
                ..Default::default()
            },
            transaction,
            PlatformVersion::latest(),
        )
    }

    /// The (data, disabled) of every TRANSFER key of `masternode`'s operator identity.
    fn transfer_keys(
        platform: &TempPlatform<MockCoreRPCLike>,
        transaction: &Transaction,
        masternode: &MasternodeListItem,
    ) -> Vec<(Vec<u8>, bool)> {
        platform
            .drive
            .fetch_full_identity(
                Identifier::create_operator_identifier(
                    masternode.pro_tx_hash.as_byte_array(),
                    &masternode.state.pub_key_operator,
                )
                .to_buffer(),
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the operator identity")
            .expect("expected the operator identity")
            .public_keys()
            .values()
            .filter(|key| key.purpose() == Purpose::TRANSFER)
            .map(|key| (key.data().to_vec(), key.is_disabled()))
            .collect()
    }

    /// Stored payouts that went stale against Core's list (changed, cleared and first set after
    /// Platform listed the masternode) are corrected in the stored list and on the current
    /// operator identities, with one list call at the reconciled height. A masternode whose
    /// payout matches is left as it is.
    #[test]
    fn should_fix_stale_operator_payouts_and_the_current_identities_keys() {
        let changed = masternode(0x61, Some(0x61));
        let cleared = masternode(0x62, Some(0x62));
        let first_set = masternode(0x63, None);
        let matching = masternode(0x64, Some(0x64));
        let in_core = |masternode: &MasternodeListItem, payout: Option<u8>| {
            let mut masternode = masternode.clone();
            masternode.state.operator_payout_address = payout.map(|payout| [payout; 20]);
            masternode
        };
        let (platform, calls) = platform_with_core(
            vec![
                changed.clone(),
                cleared.clone(),
                first_set.clone(),
                matching.clone(),
            ],
            Ok(vec![
                in_core(&changed, Some(0x81)),
                in_core(&cleared, None),
                in_core(&first_set, Some(0x83)),
                matching.clone(),
            ]),
        );
        let transaction = platform.drive.grove.start_transaction();
        let mut state = list_masternodes(&platform, &transaction);

        reconcile(&platform, &mut state, &transaction).expect("expected to reconcile");

        assert_eq!(*calls.lock().unwrap(), vec![(None, RECONCILED_AT)]);
        for (masternode, payout) in [
            (&changed, Some([0x81; 20])),
            (&cleared, None),
            (&first_set, Some([0x83; 20])),
            (&matching, Some([0x64; 20])),
        ] {
            assert_eq!(
                state.full_masternode_list()[&masternode.pro_tx_hash]
                    .state
                    .operator_payout_address,
                payout,
                "{}",
                masternode.pro_tx_hash
            );
        }
        assert_eq!(
            state.masternode_changes.upserted,
            BTreeSet::from([
                changed.pro_tx_hash,
                cleared.pro_tx_hash,
                first_set.pro_tx_hash
            ])
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, &changed),
            [(vec![0x61; 20], true), (vec![0x81; 20], false)]
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, &cleared),
            [(vec![0x62; 20], true)]
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, &first_set),
            [(vec![0x83; 20], false)]
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, &matching),
            [(vec![0x64; 20], false)]
        );
    }

    /// Core listing a masternode Platform does not, or not listing one it does, means this node
    /// cannot tell what the others reconcile: the block fails, and nothing is changed.
    #[test]
    fn should_fail_when_core_lists_other_masternodes() {
        let listed = masternode(0x65, Some(0x65));
        let other = masternode(0x66, Some(0x86));
        for core_list in [
            vec![],
            vec![listed.clone(), other.clone()],
            vec![other.clone()],
        ] {
            let (platform, _) = platform_with_core(vec![listed.clone()], Ok(core_list));
            let transaction = platform.drive.grove.start_transaction();
            let mut state = list_masternodes(&platform, &transaction);
            let before = state.clone();

            assert!(matches!(
                reconcile(&platform, &mut state, &transaction),
                Err(Error::Execution(ExecutionError::CorruptedCachedState(_)))
            ));
            assert_eq!(state.full_masternode_list(), before.full_masternode_list());
            assert!(state.masternode_changes.upserted.is_empty());
        }
    }

    /// A Core error fails the block instead of skipping the reconciliation.
    #[test]
    fn should_fail_when_core_cannot_list_its_masternodes() {
        let listed = masternode(0x67, Some(0x67));
        let (platform, _) = platform_with_core(
            vec![listed.clone()],
            Err(CoreRpcError::UnexpectedStructure("refused".to_string())),
        );
        let transaction = platform.drive.grove.start_transaction();
        let mut state = list_masternodes(&platform, &transaction);

        assert!(reconcile(&platform, &mut state, &transaction).is_err());
        assert_eq!(
            state.full_masternode_list()[&listed.pro_tx_hash]
                .state
                .operator_payout_address,
            Some([0x67; 20])
        );
    }
}
