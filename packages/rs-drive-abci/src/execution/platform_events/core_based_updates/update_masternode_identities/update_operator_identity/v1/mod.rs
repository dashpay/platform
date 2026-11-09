use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

use dpp::dashcore::ProTxHash;
use dpp::identifier::{Identifier, MasternodeIdentifiers};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::Purpose::TRANSFER;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;
use drive::util::batch::DriveOperation::IdentityOperation;
use drive::util::batch::IdentityOperationType::{
    AddNewIdentity, AddNewKeysToMasternodeIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};

/// The key operations of one update of one operator identity.
#[derive(Default)]
struct KeyChanges {
    to_disable: Vec<KeyID>,
    to_re_enable: Vec<KeyID>,
    to_add: Vec<IdentityPublicKey>,
}

/// Whether `key` is an operator identity's platform node id key.
fn is_platform_node_id_key(key: &IdentityPublicKey) -> bool {
    key.purpose() == Purpose::SYSTEM && key.key_type() == KeyType::EDDSA_25519_HASH160
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the operator identity like version 0, except for its TRANSFER keys, which follow
    /// the operator payout Core reports and nothing else.
    ///
    /// Version 0 was written for diffs that never carry an operator payout: it disables the key
    /// of the stored payout, and gives a new identity an existing TRANSFER key or the stored
    /// payout, which belong to an earlier operator. Given a payout, this version reconciles
    /// every TRANSFER key of the identity it updates with it (`reconcile_operator_payout_keys_v1`),
    /// so applying the same payout twice changes nothing the second time. On the same identity a
    /// cleared payout counts as one, and leaves no TRANSFER key enabled.
    ///
    /// A key change without a payout means no payout: Core empties the payout whenever the
    /// operator key changes, so a new identity gets no TRANSFER key. Core takes a key change from
    /// the owner alone, though, so a key change never disables a TRANSFER key without a payout
    /// to replace it: the identity left behind keeps its TRANSFER keys, as does an existing
    /// identity the masternode switches back to, and their operators can still withdraw what was
    /// sent to them. A payout Core reports with the switch back reconciles that identity's keys,
    /// which disables a payout key an earlier operator left there.
    ///
    /// Platform node ids are handled as in version 0, except that each step touches only its own
    /// kind of key (the BLS operator key, the platform node id key, the TRANSFER keys), so one key
    /// is never disabled or re-enabled twice in one update. A key whose hash another key of the
    /// identity has, which Core's payouts and node ids allow, is added as any other
    /// (`add_new_keys_to_masternode_identity` 1).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_operator_identity_v1(
        &self,
        masternode_pro_tx_hash: &ProTxHash,
        pub_key_operator_change: Option<&Vec<u8>>,
        operator_payout_address_change: Option<Option<[u8; 20]>>,
        platform_node_id_change: Option<[u8; 20]>,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if pub_key_operator_change.is_none()
            && operator_payout_address_change.is_none()
            && platform_node_id_change.is_none()
        {
            return Ok(());
        }

        let old_masternode = platform_state
            .full_masternode_list()
            .get(masternode_pro_tx_hash)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCachedState(format!(
                    "expected masternode {} to be in state",
                    masternode_pro_tx_hash
                )))
            })?;

        let old_operator_identifier = Self::get_operator_identifier_from_masternode_list_item(
            old_masternode,
            platform_version,
        )?;

        let old_identity_keys = self.fetch_operator_identity_keys_v1(
            old_operator_identifier,
            transaction,
            platform_version,
        )?;

        let Some(pub_key_operator_change) = pub_key_operator_change else {
            // Same identity.
            let Some(mut last_key_id) = old_identity_keys.keys().max().copied() else {
                return Err(ExecutionError::DriveMissingData(format!(
                    "expected the operator identity of masternode {} to be in state",
                    masternode_pro_tx_hash
                ))
                .into());
            };
            let mut changes = KeyChanges::default();

            if let Some(platform_node_id_change) = platform_node_id_change {
                let mut old_operator_node_id_to_re_enable = None;
                if let Some(old_platform_node_id) = old_masternode.state.platform_node_id.as_ref() {
                    for (key_id, key) in old_identity_keys
                        .iter()
                        .filter(|(_, key)| is_platform_node_id_key(key))
                    {
                        if key.data().as_slice() == old_platform_node_id {
                            changes.to_disable.push(*key_id);
                        } else if key.data().as_slice() == platform_node_id_change {
                            old_operator_node_id_to_re_enable = Some(*key_id);
                        }
                    }
                }
                if let Some(key_id) = old_operator_node_id_to_re_enable {
                    changes.to_re_enable.push(key_id);
                } else {
                    changes.to_add.push(Self::platform_node_id_key_v1(
                        Self::next_operator_key_id_v1(&mut last_key_id)?,
                        platform_node_id_change,
                    ));
                }
            }

            if let Some(operator_payout_address) = operator_payout_address_change {
                Self::reconcile_operator_payout_keys_v1(
                    &old_identity_keys,
                    operator_payout_address,
                    &mut last_key_id,
                    &mut changes,
                )?;
            }

            Self::push_key_changes_v1(old_operator_identifier, changes, drive_operations);
            return Ok(());
        };

        // Identity switch: the masternode moves to the identity of its new operator key, which
        // may already exist when an earlier key comes back.
        let operator_identifier = Identifier::create_operator_identifier(
            masternode_pro_tx_hash.as_byte_array(),
            pub_key_operator_change,
        );

        // The identity left behind loses its operator and platform node id keys, and keeps its
        // TRANSFER keys.
        let old_operator_identity_key_ids_to_disable: Vec<KeyID> = old_identity_keys
            .iter()
            .filter(|(_, key)| {
                (key.key_type() == KeyType::BLS12_381
                    && key.data().as_slice() == old_masternode.state.pub_key_operator)
                    || (is_platform_node_id_key(key)
                        && old_masternode
                            .state
                            .platform_node_id
                            .is_some_and(|node_id| key.data().as_slice() == node_id))
            })
            .map(|(key_id, _)| *key_id)
            .collect();

        if !old_operator_identity_key_ids_to_disable.is_empty() {
            drive_operations.push(IdentityOperation(DisableIdentityKeys {
                identity_id: old_operator_identifier.to_buffer(),
                keys_ids: old_operator_identity_key_ids_to_disable,
            }));
        }

        let new_payout_address = operator_payout_address_change.flatten();
        let new_platform_node_id =
            platform_node_id_change.or(old_masternode.state.platform_node_id);

        let identity_keys = self.fetch_operator_identity_keys_v1(
            operator_identifier,
            transaction,
            platform_version,
        )?;

        let Some(mut last_key_id) = identity_keys.keys().max().copied() else {
            let mut identity =
                Identity::create_basic_identity(operator_identifier, platform_version)?;
            identity.add_public_keys(Self::get_operator_identity_keys(
                pub_key_operator_change.clone(),
                new_payout_address,
                new_platform_node_id,
                platform_version,
            )?);
            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity,
                is_masternode_identity: true,
            }));
            return Ok(());
        };

        // An existing identity is one an earlier operator with this key left. Core takes the key
        // change from the owner alone, so without a payout to replace them its TRANSFER keys stay
        // as they are, and that operator can still withdraw what was sent to it.
        let mut changes = KeyChanges::default();
        if let Some(new_payout_address) = new_payout_address {
            Self::reconcile_operator_payout_keys_v1(
                &identity_keys,
                Some(new_payout_address),
                &mut last_key_id,
                &mut changes,
            )?;
        }

        if let Some(new_platform_node_id) = new_platform_node_id {
            if let Some((key_id, found_old_key)) = identity_keys.iter().find(|(_, key)| {
                is_platform_node_id_key(key) && key.data().as_slice() == new_platform_node_id
            }) {
                if found_old_key.is_disabled() {
                    changes.to_re_enable.push(*key_id);
                }
            } else {
                changes.to_add.push(Self::platform_node_id_key_v1(
                    Self::next_operator_key_id_v1(&mut last_key_id)?,
                    new_platform_node_id,
                ));
            }
        }

        Self::push_key_changes_v1(operator_identifier, changes, drive_operations);
        Ok(())
    }

    /// Queues the changes that leave `operator_payout_address` the only enabled TRANSFER key
    /// among `keys`, or no TRANSFER key enabled when it is `None`: a key holding the payout is
    /// kept, an enabled one before a disabled one and lower ids first, and re-enabled if it is
    /// disabled; otherwise a key is added, with the id after `last_key_id`, which it advances.
    /// Every other enabled TRANSFER key is disabled.
    fn reconcile_operator_payout_keys_v1(
        keys: &KeyIDIdentityPublicKeyPairBTreeMap,
        operator_payout_address: Option<[u8; 20]>,
        last_key_id: &mut KeyID,
        changes: &mut KeyChanges,
    ) -> Result<(), Error> {
        let reusable_key = operator_payout_address.and_then(|address| {
            keys.iter()
                .filter(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && key.data().as_slice() == address
                })
                .min_by_key(|(key_id, key)| (key.is_disabled(), **key_id))
        });
        let reused_key_id = reusable_key.map(|(key_id, _)| *key_id);
        changes.to_disable.extend(
            keys.iter()
                .filter(|(key_id, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && !key.is_disabled()
                        && reused_key_id != Some(**key_id)
                })
                .map(|(key_id, _)| *key_id),
        );
        match (reusable_key, operator_payout_address) {
            (Some((key_id, key)), _) => {
                if key.is_disabled() {
                    changes.to_re_enable.push(*key_id);
                }
            }
            (None, Some(address)) => {
                changes.to_add.push(
                    IdentityPublicKeyV0 {
                        id: Self::next_operator_key_id_v1(last_key_id)?,
                        key_type: KeyType::ECDSA_HASH160,
                        purpose: TRANSFER,
                        security_level: SecurityLevel::CRITICAL,
                        read_only: true,
                        data: BinaryData::new(address.to_vec()),
                        disabled_at: None,
                        contract_bounds: None,
                    }
                    .into(),
                );
            }
            (None, None) => {}
        }
        Ok(())
    }

    fn fetch_operator_identity_keys_v1(
        &self,
        identity_id: Identifier,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<KeyIDIdentityPublicKeyPairBTreeMap, Error> {
        Ok(self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                IdentityKeysRequest {
                    identity_id: identity_id.to_buffer(),
                    request_type: KeyRequestType::AllKeys,
                    limit: None,
                    offset: None,
                },
                Some(transaction),
                platform_version,
            )?)
    }

    /// The id for a key added after `last_key_id`, which becomes it. Only an added key takes an
    /// id, so an identity at the last id can still have keys reused, disabled or re-enabled.
    fn next_operator_key_id_v1(last_key_id: &mut KeyID) -> Result<KeyID, Error> {
        *last_key_id = last_key_id.checked_add(1).ok_or_else(|| {
            Error::from(ExecutionError::Overflow(
                "masternode operator identity key id exhausted",
            ))
        })?;
        Ok(*last_key_id)
    }

    fn platform_node_id_key_v1(key_id: KeyID, platform_node_id: [u8; 20]) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id: key_id,
            key_type: KeyType::EDDSA_25519_HASH160,
            purpose: Purpose::SYSTEM,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(platform_node_id.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into()
    }

    /// Queues `changes` on `identity_id`: disables first, then re-enables, then additions, as
    /// version 0 orders them.
    fn push_key_changes_v1(
        identity_id: Identifier,
        changes: KeyChanges,
        drive_operations: &mut Vec<DriveOperation>,
    ) {
        let KeyChanges {
            to_disable,
            to_re_enable,
            to_add,
        } = changes;
        if !to_disable.is_empty() {
            drive_operations.push(IdentityOperation(DisableIdentityKeys {
                identity_id: identity_id.to_buffer(),
                keys_ids: to_disable,
            }));
        }
        if !to_re_enable.is_empty() {
            drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                identity_id: identity_id.to_buffer(),
                keys_ids: to_re_enable,
            }));
        }
        if !to_add.is_empty() {
            drive_operations.push(IdentityOperation(AddNewKeysToMasternodeIdentity {
                identity_id: identity_id.to_buffer(),
                non_unique_keys_to_add: to_add,
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityV0, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::{Identifier, Identity, IdentityPublicKey};
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::net::SocketAddr;

    const PRO_TX_HASH: [u8; 32] = [0x41; 32];
    const PUB_KEY_OPERATOR: [u8; 48] = [0x42; 48];
    const NODE_ID: [u8; 20] = [0x43; 20];

    /// A platform listing one masternode whose stored operator payout is `stored_payout`, and
    /// whose operator identity holds the operator key, the platform node id key and a TRANSFER
    /// key for each `(data, disabled)` of `transfer_keys`, with ids from 2 on. With
    /// `create_identity` false the identity is not created.
    fn platform_with_operator(
        stored_payout: Option<[u8; 20]>,
        transfer_keys: &[([u8; 20], bool)],
        create_identity: bool,
    ) -> (TempPlatform<MockCoreRPCLike>, PlatformState) {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        if create_identity {
            add_operator_identity(&platform, &PUB_KEY_OPERATOR, transfer_keys);
        }
        let mut platform_state = platform.state.load().as_ref().clone();
        platform_state.full_masternode_list_mut().insert(
            ProTxHash::from_byte_array(PRO_TX_HASH),
            MasternodeListItem {
                node_type: MasternodeType::Regular,
                pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
                collateral_hash: Txid::from_byte_array([0x44; 32]),
                collateral_index: 0,
                collateral_address: Some([0x45; 20]),
                operator_reward: 10.0,
                state: DMNState {
                    service: SocketAddr::from(([192, 0, 2, 1], 9999)),
                    registered_height: 0,
                    pose_revived_height: None,
                    pose_ban_height: None,
                    revocation_reason: 0,
                    owner_address: Some([0x46; 20]),
                    voting_address: [0x47; 20],
                    payout_address: Some([0x48; 20]),
                    payouts: None,
                    pub_key_operator: PUB_KEY_OPERATOR.to_vec(),
                    operator_payout_address: stored_payout,
                    platform_node_id: Some(NODE_ID),
                    #[allow(deprecated)]
                    legacy_platform_p2p_port: None,
                    #[allow(deprecated)]
                    legacy_platform_http_port: None,
                    addresses: None,
                },
            },
        );
        (platform, platform_state)
    }

    /// Adds the operator identity of the masternode for operator key `pub_key_operator`, with
    /// that key, the platform node id key and a TRANSFER key for each `(data, disabled)` of
    /// `transfer_keys`, with ids from 2 on.
    fn add_operator_identity(
        platform: &TempPlatform<MockCoreRPCLike>,
        pub_key_operator: &[u8],
        transfer_keys: &[([u8; 20], bool)],
    ) {
        let key = |id: KeyID, key_type, purpose, data: &[u8], disabled: bool| {
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(data.to_vec()),
                disabled_at: disabled.then_some(1),
                contract_bounds: None,
            }
            .into()
        };
        let mut public_keys = BTreeMap::from([
            (
                0,
                key(
                    0,
                    KeyType::BLS12_381,
                    Purpose::SYSTEM,
                    pub_key_operator,
                    false,
                ),
            ),
            (
                1,
                key(
                    1,
                    KeyType::EDDSA_25519_HASH160,
                    Purpose::SYSTEM,
                    &NODE_ID,
                    false,
                ),
            ),
        ]);
        for (key_id, (data, disabled)) in (2..).zip(transfer_keys) {
            public_keys.insert(
                key_id,
                key(
                    key_id,
                    KeyType::ECDSA_HASH160,
                    Purpose::TRANSFER,
                    data,
                    *disabled,
                ),
            );
        }
        let identity: Identity = IdentityV0 {
            id: Identifier::create_operator_identifier(&PRO_TX_HASH, pub_key_operator),
            public_keys,
            balance: 0,
            revision: 0,
        }
        .into();
        platform
            .drive
            .add_new_identity(
                identity,
                true,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to add the operator identity");
    }

    fn operator_identifier() -> Identifier {
        Identifier::create_operator_identifier(&PRO_TX_HASH, &PUB_KEY_OPERATOR)
    }

    /// Runs `update_operator_identity_v1` for the masternode with `pub_key_operator`, `payout`
    /// and `platform_node_id` and applies what it queued. Returns how many operations it queued.
    fn update(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        pub_key_operator: Option<&Vec<u8>>,
        payout: Option<Option<[u8; 20]>>,
        platform_node_id: Option<[u8; 20]>,
    ) -> Result<usize, Error> {
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();
        let mut drive_operations = vec![];
        platform.update_operator_identity_v1(
            &ProTxHash::from_byte_array(PRO_TX_HASH),
            pub_key_operator,
            payout,
            platform_node_id,
            platform_state,
            &transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let queued = drive_operations.len();
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to apply the key changes");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
        Ok(queued)
    }

    /// The (id, purpose, data, disabled) of every key of the operator identity for operator key
    /// `pub_key_operator`; `None` without that identity.
    fn keys_of(
        platform: &TempPlatform<MockCoreRPCLike>,
        pub_key_operator: &[u8],
    ) -> Option<Vec<(KeyID, Purpose, Vec<u8>, bool)>> {
        platform
            .drive
            .fetch_full_identity(
                Identifier::create_operator_identifier(&PRO_TX_HASH, pub_key_operator).to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the operator identity")
            .map(|identity| {
                identity
                    .public_keys()
                    .iter()
                    .map(|(id, key)| (*id, key.purpose(), key.data().to_vec(), key.is_disabled()))
                    .collect()
            })
    }

    /// The (purpose, data, disabled) of every key of the operator identity, by key id.
    fn keys(platform: &TempPlatform<MockCoreRPCLike>) -> Vec<(Purpose, Vec<u8>, bool)> {
        platform
            .drive
            .fetch_full_identity(
                operator_identifier().to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the operator identity")
            .expect("expected the operator identity")
            .public_keys()
            .values()
            .map(|key| (key.purpose(), key.data().to_vec(), key.is_disabled()))
            .collect()
    }

    /// The payout's key is the only TRANSFER key left enabled, whatever the stored payout says:
    /// a disabled key holding it is re-enabled, and every other enabled TRANSFER key disabled.
    /// The second identical update queues nothing.
    #[test]
    fn should_leave_only_the_payout_key_enabled_and_change_nothing_the_second_time() {
        let (platform, platform_state) = platform_with_operator(
            Some([0x51; 20]),
            &[([0x51; 20], false), ([0x52; 20], false), ([0x53; 20], true)],
            true,
        );
        assert!(
            update(
                &platform,
                &platform_state,
                None,
                Some(Some([0x53; 20])),
                None
            )
            .unwrap()
                > 0
        );
        assert_eq!(
            keys(&platform)[2..],
            [
                (Purpose::TRANSFER, vec![0x51; 20], true),
                (Purpose::TRANSFER, vec![0x52; 20], true),
                (Purpose::TRANSFER, vec![0x53; 20], false),
            ]
        );
        assert_eq!(
            update(
                &platform,
                &platform_state,
                None,
                Some(Some([0x53; 20])),
                None
            )
            .unwrap(),
            0
        );
    }

    /// A cleared payout leaves no TRANSFER key enabled, and adds none.
    #[test]
    fn should_disable_every_transfer_key_when_the_payout_is_cleared() {
        let (platform, platform_state) = platform_with_operator(
            Some([0x51; 20]),
            &[([0x51; 20], false), ([0x52; 20], false)],
            true,
        );
        update(&platform, &platform_state, None, Some(None), None).unwrap();
        assert_eq!(
            keys(&platform)[2..],
            [
                (Purpose::TRANSFER, vec![0x51; 20], true),
                (Purpose::TRANSFER, vec![0x52; 20], true),
            ]
        );
        assert_eq!(
            update(&platform, &platform_state, None, Some(None), None).unwrap(),
            0
        );
    }

    /// An update without a payout, here a platform node id change, leaves the TRANSFER keys as
    /// they are, and handles the node id as version 0 does.
    #[test]
    fn should_leave_the_transfer_keys_of_an_update_without_a_payout() {
        let (platform, platform_state) = platform_with_operator(
            Some([0x51; 20]),
            &[([0x51; 20], false), ([0x52; 20], false)],
            true,
        );
        update(&platform, &platform_state, None, None, Some([0x54; 20])).unwrap();
        assert_eq!(
            keys(&platform),
            [
                (Purpose::SYSTEM, PUB_KEY_OPERATOR.to_vec(), false),
                (Purpose::SYSTEM, NODE_ID.to_vec(), true),
                (Purpose::TRANSFER, vec![0x51; 20], false),
                (Purpose::TRANSFER, vec![0x52; 20], false),
                (Purpose::SYSTEM, vec![0x54; 20], false),
            ]
        );
    }

    /// Every listed masternode has an operator identity, so a missing one is corrupted state:
    /// the update fails rather than add keys to an identity that does not exist.
    #[test]
    fn should_fail_when_the_operator_identity_is_missing() {
        let (platform, platform_state) = platform_with_operator(Some([0x51; 20]), &[], false);
        assert!(matches!(
            update(
                &platform,
                &platform_state,
                None,
                Some(Some([0x52; 20])),
                None
            ),
            Err(Error::Execution(ExecutionError::DriveMissingData(_)))
        ));
    }

    /// A platform node id and a payout that each need a new key get two keys with different ids.
    #[test]
    fn should_give_distinct_ids_to_the_keys_one_update_adds() {
        let (platform, platform_state) =
            platform_with_operator(Some([0x51; 20]), &[([0x51; 20], false)], true);
        update(
            &platform,
            &platform_state,
            None,
            Some(Some([0x52; 20])),
            Some([0x54; 20]),
        )
        .unwrap();
        assert_eq!(
            keys_of(&platform, &PUB_KEY_OPERATOR).expect("expected the operator identity")[1..],
            [
                (1, Purpose::SYSTEM, NODE_ID.to_vec(), true),
                (2, Purpose::TRANSFER, vec![0x51; 20], true),
                (3, Purpose::SYSTEM, vec![0x54; 20], false),
                (4, Purpose::TRANSFER, vec![0x52; 20], false),
            ]
        );
    }

    /// Core empties the operator payout whenever the operator key changes, so a key change
    /// without a payout creates the new operator identity without a TRANSFER key, never with the
    /// stored payout, which is the previous operator's. The identity left behind keeps it.
    #[test]
    fn should_create_a_new_operator_identity_without_the_previous_operators_payout_key() {
        let (platform, platform_state) =
            platform_with_operator(Some([0x51; 20]), &[([0x51; 20], false)], true);
        let new_key = vec![0x62; 48];
        update(&platform, &platform_state, Some(&new_key), None, None).unwrap();
        let new_identity = keys_of(&platform, &new_key).expect("expected the new identity");
        assert!(
            new_identity
                .iter()
                .all(|(_, purpose, ..)| *purpose != Purpose::TRANSFER),
            "{new_identity:?}"
        );
        assert_eq!(
            keys(&platform),
            [
                (Purpose::SYSTEM, PUB_KEY_OPERATOR.to_vec(), true),
                (Purpose::SYSTEM, NODE_ID.to_vec(), true),
                (Purpose::TRANSFER, vec![0x51; 20], false),
            ]
        );
    }

    /// A key change to an operator identity that already exists, an earlier operator's,
    /// reconciles its TRANSFER keys with a payout: the earlier operator's key is disabled and
    /// the payout's added, with a new platform node id key under its own id. Without a payout,
    /// cleared or absent, its TRANSFER keys stay as they are, so the earlier operator can still
    /// withdraw what was sent to it.
    #[test]
    fn should_reconcile_the_transfer_keys_of_an_existing_identity_a_key_change_moves_to() {
        for (payout, expected_transfer_keys) in [
            (
                Some(Some([0x56; 20])),
                vec![
                    (2, Purpose::TRANSFER, vec![0x55; 20], true),
                    (3, Purpose::TRANSFER, vec![0x56; 20], false),
                ],
            ),
            (
                Some(None),
                vec![(2, Purpose::TRANSFER, vec![0x55; 20], false)],
            ),
            (None, vec![(2, Purpose::TRANSFER, vec![0x55; 20], false)]),
        ] {
            let (platform, platform_state) =
                platform_with_operator(Some([0x51; 20]), &[([0x51; 20], false)], true);
            let earlier_key = vec![0x63; 48];
            add_operator_identity(&platform, &earlier_key, &[([0x55; 20], false)]);
            update(
                &platform,
                &platform_state,
                Some(&earlier_key),
                payout,
                Some([0x57; 20]),
            )
            .unwrap();
            let identity = keys_of(&platform, &earlier_key).expect("expected the identity");
            assert_eq!(
                identity
                    .iter()
                    .filter(|(_, purpose, ..)| *purpose == Purpose::TRANSFER)
                    .cloned()
                    .collect::<Vec<_>>(),
                expected_transfer_keys,
                "payout {payout:?}"
            );
            let node_id_keys = identity
                .iter()
                .filter(|(_, _, data, _)| *data == vec![0x57; 20])
                .collect::<Vec<_>>();
            assert_eq!(node_id_keys.len(), 1, "payout {payout:?}: {identity:?}");
            assert!(
                identity.iter().all(|(id, ..)| identity
                    .iter()
                    .filter(|(other, ..)| other == id)
                    .count()
                    == 1),
                "payout {payout:?}: {identity:?}"
            );
        }
    }

    /// Only an added key takes a new id, so an operator identity whose last key id is the
    /// largest one can still have its payout key reused or its other keys disabled.
    #[test]
    fn should_reconcile_without_a_new_key_id_when_no_key_is_added() {
        let (platform, platform_state) = platform_with_operator(Some([0x51; 20]), &[], false);
        let key = |id: KeyID, data: [u8; 20], disabled: bool| -> IdentityPublicKey {
            IdentityPublicKeyV0 {
                id,
                key_type: KeyType::ECDSA_HASH160,
                purpose: Purpose::TRANSFER,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(data.to_vec()),
                disabled_at: disabled.then_some(1),
                contract_bounds: None,
            }
            .into()
        };
        let identity: Identity = IdentityV0 {
            id: operator_identifier(),
            public_keys: BTreeMap::from([
                (0, key(0, [0x51; 20], false)),
                (KeyID::MAX, key(KeyID::MAX, [0x52; 20], true)),
            ]),
            balance: 0,
            revision: 0,
        }
        .into();
        platform
            .drive
            .add_new_identity(
                identity,
                true,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to add the operator identity");

        update(
            &platform,
            &platform_state,
            None,
            Some(Some([0x52; 20])),
            None,
        )
        .expect("expected to reuse the payout key");
        assert_eq!(
            keys(&platform),
            [
                (Purpose::TRANSFER, vec![0x51; 20], true),
                (Purpose::TRANSFER, vec![0x52; 20], false),
            ]
        );
    }
}
