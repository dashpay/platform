use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dashcore_rpc::dashcore::ProTxHash;

use dpp::dashcore::hashes::Hash;
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
    AddNewIdentity, AddNewKeysToIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn update_operator_identity_v0(
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

        let needs_change_operator_payout_address = operator_payout_address_change.is_some();
        let needs_change_platform_node_id = platform_node_id_change.is_some();

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

        let (operator_identifier, changed_identity) =
            if let Some(pub_key_operator_change) = pub_key_operator_change {
                (
                    Identifier::create_operator_identifier(
                        masternode_pro_tx_hash.as_byte_array(),
                        pub_key_operator_change,
                    ),
                    true,
                )
            } else {
                (old_operator_identifier, false)
            };

        let key_request = IdentityKeysRequest {
            identity_id: old_operator_identifier.to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let old_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                key_request,
                Some(transaction),
                platform_version,
            )?;

        // two possibilities, same identity or identity switch.
        if !changed_identity {
            // we are on same identity for platform

            let mut old_operator_node_id_to_re_enable = None;

            let mut old_operator_payout_address_to_re_enable = None;

            let last_key_id = old_identity_keys.keys().max().copied().unwrap_or_default();

            let old_operator_identity_key_ids_to_disable: Vec<KeyID> = old_identity_keys
                .into_iter()
                .filter_map(|(key_id, key)| {
                    // We can disable previous withdrawal keys as we are adding a new one
                    if needs_change_operator_payout_address {
                        if Some(key.data().as_slice())
                            == old_masternode
                                .state
                                .operator_payout_address
                                .as_ref()
                                .map(|bytes| bytes.as_slice())
                        {
                            return Some(key_id);
                        } else if let Some(operator_payout_address) =
                            operator_payout_address_change.as_ref().unwrap()
                        {
                            // an old key that we need to re-enable
                            if key.data().as_slice() == operator_payout_address.as_slice() {
                                old_operator_payout_address_to_re_enable = Some(key_id);
                            }
                        }
                    }
                    if needs_change_platform_node_id
                        && old_masternode.state.platform_node_id.is_some()
                    {
                        if key.data().as_slice()
                            == old_masternode.state.platform_node_id.as_ref().unwrap()
                        {
                            return Some(key_id);
                        } else if platform_node_id_change.as_ref().unwrap().as_slice()
                            == key.data().as_slice()
                        {
                            old_operator_node_id_to_re_enable = Some(key_id);
                        }
                    }
                    None
                })
                .collect();

            if !old_operator_identity_key_ids_to_disable.is_empty() {
                drive_operations.push(IdentityOperation(DisableIdentityKeys {
                    identity_id: operator_identifier.to_buffer(),
                    keys_ids: old_operator_identity_key_ids_to_disable,
                }));
            }

            let mut keys_to_re_enable = vec![];
            let mut non_unique_keys_to_add = vec![];

            let mut new_key_id = last_key_id + 1;

            if let Some(old_operator_pub_key_to_re_enable) = old_operator_node_id_to_re_enable {
                keys_to_re_enable.push(old_operator_pub_key_to_re_enable);
            } else if needs_change_platform_node_id {
                let key: IdentityPublicKey = IdentityPublicKeyV0 {
                    id: new_key_id,
                    key_type: KeyType::EDDSA_25519_HASH160,
                    purpose: Purpose::SYSTEM,
                    security_level: SecurityLevel::CRITICAL,
                    read_only: true,
                    data: BinaryData::new(
                        platform_node_id_change
                            .as_ref()
                            .expect("platform node id confirmed is some")
                            .to_vec(),
                    ),
                    disabled_at: None,
                    contract_bounds: None,
                }
                .into();
                non_unique_keys_to_add.push(key);
                new_key_id += 1;
            }

            if let Some(old_operator_payout_address_to_re_enable) =
                old_operator_payout_address_to_re_enable
            {
                keys_to_re_enable.push(old_operator_payout_address_to_re_enable);
            } else if needs_change_operator_payout_address {
                if let Some(new_operator_payout_address) = operator_payout_address_change
                    .as_ref()
                    .expect("operator_payout_address confirmed is some")
                {
                    let key = IdentityPublicKeyV0 {
                        id: new_key_id,
                        key_type: KeyType::ECDSA_HASH160,
                        purpose: TRANSFER,
                        security_level: SecurityLevel::CRITICAL,
                        read_only: true,
                        data: BinaryData::new(new_operator_payout_address.to_vec()),
                        disabled_at: None,
                        contract_bounds: None,
                    };
                    non_unique_keys_to_add.push(key.into());
                    // new_key_id += 1;
                }
            }

            if !keys_to_re_enable.is_empty() {
                drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                    identity_id: operator_identifier.to_buffer(),
                    keys_ids: keys_to_re_enable,
                }));
            }

            if !non_unique_keys_to_add.is_empty() {
                drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
                    identity_id: operator_identifier.to_buffer(),
                    unique_keys_to_add: vec![],
                    non_unique_keys_to_add,
                }));
            }
        } else {
            // We have changed operator keys, this means we are now on a new operator identity
            // Or as a rare case an operator identity that already existed

            let key_request = IdentityKeysRequest {
                identity_id: operator_identifier.to_buffer(),
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            // We can not disable previous withdrawal keys,
            // Let's disable other two keys
            let old_operator_identity_key_ids_to_disable: Vec<KeyID> = old_identity_keys
                .into_iter()
                .filter_map(|(key_id, key)| {
                    if key.data().as_slice() == old_masternode.state.pub_key_operator {
                        //the old key
                        return Some(key_id);
                    }
                    if old_masternode.state.platform_node_id.is_some()
                        && key.data().as_slice()
                            == old_masternode.state.platform_node_id.as_ref().unwrap()
                    {
                        return Some(key_id);
                    }
                    None
                })
                .collect();

            if !old_operator_identity_key_ids_to_disable.is_empty() {
                drive_operations.push(IdentityOperation(DisableIdentityKeys {
                    identity_id: old_operator_identifier.to_buffer(),
                    keys_ids: old_operator_identity_key_ids_to_disable,
                }));
            }

            let identity_to_enable_old_keys = self
                .drive
                .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                    key_request,
                    Some(transaction),
                    platform_version,
                )?;

            let new_payout_address =
                if let Some(operator_payout_address) = operator_payout_address_change {
                    operator_payout_address
                } else {
                    if let Some((_, found_old_key)) = identity_to_enable_old_keys
                        .iter()
                        .find(|(_, key)| key.purpose() == Purpose::TRANSFER)
                    {
                        Some(found_old_key.data().to_vec().try_into().map_err(|_| {
                            Error::Execution(ExecutionError::CorruptedDriveResponse(
                                "old payout address should be 20 bytes".to_string(),
                            ))
                        })?)
                    } else {
                        // finally we just use the old masternode payout address
                        // we need to use the old pub_key_operator
                        old_masternode.state.operator_payout_address
                    }
                };

            let new_platform_node_id = if let Some(platform_node_id) = platform_node_id_change {
                // if it changed it means it always existed
                Some(platform_node_id)
            } else {
                // we need to use the old platform_node_id, we shouldn't do the same as with the withdrawal address
                old_masternode.state.platform_node_id
            };
            if identity_to_enable_old_keys.is_empty() {
                // Now we need to create the new operator identity with the new keys
                let mut identity =
                    Identity::create_basic_identity(operator_identifier, platform_version)?;
                identity.add_public_keys(Self::get_operator_identity_keys(
                    pub_key_operator_change
                        .expect("expected a pub key operator")
                        .clone(),
                    new_payout_address,
                    new_platform_node_id,
                    platform_version,
                )?);
                drive_operations.push(IdentityOperation(AddNewIdentity {
                    identity,
                    is_masternode_identity: true,
                }));
            } else {
                let mut key_ids_to_reenable = vec![];
                let mut non_unique_keys_to_add = vec![];

                let last_key_id = identity_to_enable_old_keys
                    .keys()
                    .max()
                    .copied()
                    .unwrap_or_default();

                let mut new_key_id = last_key_id + 1;

                if let Some(new_payout_address) = new_payout_address {
                    if let Some((key_id, found_old_key)) = identity_to_enable_old_keys
                        .iter()
                        .find(|(_, key)| key.data().as_slice() == &new_payout_address)
                    {
                        if found_old_key.is_disabled() {
                            key_ids_to_reenable.push(*key_id)
                        }
                    } else {
                        let key = IdentityPublicKeyV0 {
                            id: new_key_id,
                            key_type: KeyType::ECDSA_HASH160,
                            purpose: TRANSFER,
                            security_level: SecurityLevel::CRITICAL,
                            read_only: true,
                            data: BinaryData::new(new_payout_address.to_vec()),
                            disabled_at: None,
                            contract_bounds: None,
                        };
                        non_unique_keys_to_add.push(key.into());
                        new_key_id += 1;
                    }
                }

                if let Some(new_platform_node_id) = new_platform_node_id {
                    if let Some((key_id, found_old_key)) = identity_to_enable_old_keys
                        .into_iter()
                        .find(|(_, key)| key.data().as_slice() == &new_platform_node_id)
                    {
                        if found_old_key.is_disabled() {
                            key_ids_to_reenable.push(key_id)
                        }
                    } else {
                        let key: IdentityPublicKey = IdentityPublicKeyV0 {
                            id: new_key_id,
                            key_type: KeyType::EDDSA_25519_HASH160,
                            purpose: Purpose::SYSTEM,
                            security_level: SecurityLevel::CRITICAL,
                            read_only: true,
                            data: BinaryData::new(
                                platform_node_id_change
                                    .as_ref()
                                    .expect("platform node id confirmed is some")
                                    .to_vec(),
                            ),
                            disabled_at: None,
                            contract_bounds: None,
                        }
                        .into();
                        non_unique_keys_to_add.push(key);
                    }
                }

                if !key_ids_to_reenable.is_empty() {
                    // We just need to reenable keys on the old identity
                    drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                        identity_id: operator_identifier.to_buffer(),
                        keys_ids: key_ids_to_reenable,
                    }));
                }

                if !non_unique_keys_to_add.is_empty() {
                    // We should add keys that didn't exist before
                    drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
                        identity_id: operator_identifier.to_buffer(),
                        unique_keys_to_add: vec![],
                        non_unique_keys_to_add,
                    }));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dashcore_rpc::dashcore::ProTxHash;
    use dashcore_rpc::dashcore_rpc_json::{MasternodeListItem, MasternodeType};
    use dashcore_rpc::json::DMNState;
    use dpp::block::block_info::BlockInfo;
    use dpp::bls_signatures::PrivateKey as BlsPrivateKey;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::{Identifier, Identity, IdentityPublicKey};
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;
    use std::net::SocketAddr;
    use std::ops::Deref;
    use std::str::FromStr;

    fn create_operator_identity<C>(
        platform: &TempPlatform<C>,
        rng: &mut StdRng,
    ) -> (ProTxHash, Identity, [u8; 20], Vec<u8>, [u8; 20]) {
        let platform_version = PlatformVersion::latest();

        // Create a dummy ProTxHash
        let pro_tx_hash_bytes: [u8; 32] = rng.gen();

        let pro_tx_hash = ProTxHash::from_raw_hash(Hash::from_byte_array(pro_tx_hash_bytes));

        let payout_address: [u8; 20] = rng.gen();

        let node_id_bytes: [u8; 20] = rng.gen();

        // Create a public key operator and payout address
        let private_key_operator =
            BlsPrivateKey::generate_dash(rng).expect("expected to generate a private key");
        let pub_key_operator = private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();

        let operator_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::BLS12_381,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(pub_key_operator.clone()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into();

        let withdrawal_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 1,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(payout_address.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into();

        let node_id: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 2,
            key_type: KeyType::EDDSA_25519_HASH160,
            purpose: Purpose::SYSTEM,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(node_id_bytes.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into();

        let identity: Identity = IdentityV0 {
            id: Identifier::create_operator_identifier(&pro_tx_hash_bytes, &pub_key_operator),
            public_keys: BTreeMap::from([(0, operator_key), (1, withdrawal_key), (2, node_id)]),
            balance: 0,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        (
            pro_tx_hash,
            identity,
            payout_address,
            pub_key_operator,
            node_id_bytes,
        )
    }

    #[test]
    fn test_update_operator_payout_address() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let (pro_tx_hash, _identity, operator_payout_address, pub_key_operator, node_id_bytes) =
            create_operator_identity(&platform, &mut rng);

        let new_operator_payout_address: [u8; 20] = rng.gen();

        // Create an old masternode state
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(node_id_bytes),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        // Add the old masternode to the platform state
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                None,
                Some(Some(new_operator_payout_address)),
                None,
                &platform_state,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn test_update_operator_change_back_to_previous_operator_address() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let (pro_tx_hash, _identity, operator_payout_address, pub_key_operator, node_id) =
            create_operator_identity(&platform, &mut rng);

        let new_operator_payout_address: [u8; 20] = rng.gen();

        // Create an old masternode state
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(node_id),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        // Add the old masternode to the platform state
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                None,
                Some(Some(new_operator_payout_address)),
                None,
                &platform_state,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");

        // Create an old masternode state
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: pub_key_operator.clone(),
                operator_payout_address: Some(new_operator_payout_address),
                platform_node_id: Some(node_id),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let mut re_enable_drive_operations = vec![];

        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                None,
                Some(Some(operator_payout_address)),
                None,
                &platform_state,
                &transaction,
                &mut re_enable_drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                re_enable_drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn test_update_operator_platform_node_id() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let (pro_tx_hash, _identity, operator_payout_address, pub_key_operator, node_id_bytes) =
            create_operator_identity(&platform, &mut rng);

        let new_platform_node_id: [u8; 20] = rng.gen();

        // Create an old masternode state
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(node_id_bytes),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        // Add the old masternode to the platform state
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                None,
                None,
                Some(new_platform_node_id),
                &platform_state,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn test_update_operator_change_back_to_previous_node_id() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let (pro_tx_hash, _identity, operator_payout_address, pub_key_operator, original_node_id) =
            create_operator_identity(&platform, &mut rng);

        let new_platform_node_id: [u8; 20] = rng.gen();

        // Create an old masternode state with original platform node ID
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(original_node_id),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        // Add the old masternode to the platform state
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        // Update to new node ID
        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                None,
                None,
                Some(new_platform_node_id),
                &platform_state,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");

        // Create an old masternode state
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(new_platform_node_id),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        // Change back to original node ID
        let mut re_enable_drive_operations = vec![];

        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                None,
                None,
                Some(original_node_id),
                &platform_state,
                &transaction,
                &mut re_enable_drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                re_enable_drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn test_update_operator_public_key() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let (
            pro_tx_hash,
            _identity,
            operator_payout_address,
            original_pub_key_operator,
            node_id_bytes,
        ) = create_operator_identity(&platform, &mut rng);

        // Generate a new public key operator
        let new_private_key_operator =
            BlsPrivateKey::generate_dash(&mut rng).expect("expected to generate a private key");
        let new_pub_key_operator = new_private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();

        // Create an old masternode state
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: original_pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(node_id_bytes),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        // Add the old masternode to the platform state
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        // Update the operator public key
        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                Some(&new_pub_key_operator),
                None,
                None,
                &platform_state,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn test_update_operator_change_back_to_previous_public_key() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let (
            pro_tx_hash,
            _identity,
            operator_payout_address,
            original_pub_key_operator,
            node_id_bytes,
        ) = create_operator_identity(&platform, &mut rng);

        // Generate a new public key operator
        let new_private_key_operator =
            BlsPrivateKey::generate_dash(&mut rng).expect("expected to generate a private key");
        let new_pub_key_operator = new_private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();

        // Create an old masternode state with original public key operator
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: original_pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(node_id_bytes),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        // Add the old masternode to the platform state
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        // Update to new public key operator
        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                Some(&new_pub_key_operator),
                None,
                None,
                &platform_state,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");

        // Create an old masternode state with original public key operator
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.0.1.1:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator: new_pub_key_operator.clone(),
                operator_payout_address: Some(operator_payout_address),
                platform_node_id: Some(node_id_bytes),
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        platform_state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode_list_item);

        // Change back to original public key operator
        let mut re_enable_drive_operations = vec![];

        platform
            .update_operator_identity_v0(
                &pro_tx_hash,
                Some(&original_pub_key_operator),
                None,
                None,
                &platform_state,
                &transaction,
                &mut re_enable_drive_operations,
                platform_version,
            )
            .expect("expected to update operator identity");

        platform
            .drive
            .apply_drive_operations(
                re_enable_drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }
}
