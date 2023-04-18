use crate::abci::AbciError;
use crate::error::Error;
use crate::error::Error::Abci;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use chrono::Utc;
use dashcore::hashes::Hash;
use dashcore::ProTxHash;
use dashcore_rpc::json::{
    Masternode, MasternodeListDiffWithMasternodes, MasternodeListItem, QuorumMasternodeListItem,
    RemovedMasternodeItem, UpdatedMasternodeItem,
};
use dpp::identifier::Identifier;
use dpp::identity::factory::IDENTITY_PROTOCOL_VERSION;
use dpp::identity::{
    Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel, TimestampMillis,
};
use dpp::platform_value::BinaryData;
use drive::drive::block_info::BlockInfo;
use drive::grovedb::Transaction;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update of the masternode identities
    pub fn update_masternode_identities(
        &self,
        previous_core_height: u32,
        current_core_height: u32,
        masternode_diff: MasternodeListDiffWithMasternodes,
        block_info: &BlockInfo,
        state: &PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        if previous_core_height != current_core_height {
            let MasternodeListDiffWithMasternodes {
                added_mns,
                updated_mns,
                removed_mns,
                ..
            } = masternode_diff;

            for masternode in added_mns {
                let owner_identity = self.create_owner_identity(&masternode)?;
                let voter_identity = self.create_voter_identity(&masternode)?;
                let operator_identity = self.create_operator_identity(&masternode)?;

                // TODO: can this be batched?
                self.drive.add_new_identity(
                    owner_identity,
                    &block_info,
                    true,
                    Some(&transaction),
                )?;
                self.drive.add_new_identity(
                    voter_identity,
                    &block_info,
                    true,
                    Some(&transaction),
                )?;
                self.drive.add_new_identity(
                    operator_identity,
                    &block_info,
                    true,
                    Some(&transaction),
                )?;
            }

            for masternode in updated_mns {
                self.update_owner_identity(&masternode, &block_info, Some(&transaction))?;
                self.update_voter_identity(&masternode, &block_info, state, Some(&transaction))?;
                self.update_operator_identity(&masternode, &block_info, state, Some(&transaction))?;
            }

            for masternode in removed_mns {
                self.disable_identity_keys(&masternode, &block_info, state, Some(&transaction))?;
            }
        }

        Ok(())
    }

    fn update_owner_identity(
        &self,
        masternode: &UpdatedMasternodeItem,
        block_info: &BlockInfo,
        transaction: Option<&Transaction>,
    ) -> Result<(), Error> {
        if masternode.state_diff.payout_address.is_none() {
            return Ok(());
        }

        let owner_identifier: [u8; 32] = masternode.protx_hash.clone().into_inner();
        let owner_identity = self
            .drive
            .fetch_full_identity(owner_identifier, transaction)?
            .ok_or_else(|| {
                Error::Abci(AbciError::InvalidState(
                    "expected identity to be in state".to_string(),
                ))
            })?;

        // TODO: extract the diff function
        // now we need to figure out which of the keys to disable
        let new_key_id: KeyID = owner_identity
            .public_keys
            .last_key_value()
            .map(|(last_key_id, _)| last_key_id + 1)
            .unwrap_or(0);
        let to_disable = owner_identity
            .public_keys
            .iter()
            .filter(|(_, pk)| pk.disabled_at.is_none())
            .map(|(id, _)| id.clone())
            .collect::<Vec<KeyID>>();

        let new_owner_key = Self::get_owner_identity_key(
            masternode
                .state_diff
                .payout_address
                .expect("confirmed is some"),
            0,
        )?;
        let current_time = Utc::now().timestamp_millis() as TimestampMillis;

        self.drive.disable_identity_keys(
            owner_identifier,
            to_disable,
            current_time,
            block_info,
            true,
            transaction,
        );
        // add the new key
        self.drive.add_new_non_unique_keys_to_identity(
            owner_identifier,
            vec![new_owner_key],
            block_info,
            true,
            transaction,
        );
        Ok(())
    }

    fn update_voter_identity(
        &self,
        masternode: &UpdatedMasternodeItem,
        block_info: &BlockInfo,
        state: &PlatformState,
        transaction: Option<&Transaction>,
    ) -> Result<(), Error> {
        if masternode.state_diff.voting_address.is_none() {
            return Ok(());
        }

        let protx_hash: &ProTxHash = &masternode.protx_hash;
        let old_masternode = state.full_masternode_list.get(protx_hash).ok_or_else(|| {
            Error::Abci(AbciError::InvalidState(
                "expected masternode to be in state".to_string(),
            ))
        })?;

        let voter_identifier = Self::get_voter_identifier(&old_masternode)?;

        let voter_identity = self
            .drive
            .fetch_full_identity(voter_identifier, transaction)?
            .ok_or_else(|| {
                Error::Abci(AbciError::InvalidState(
                    "expected identity to be in state".to_string(),
                ))
            })?;

        // TODO: extract the diff function
        // now we need to figure out which of the keys to disable
        let new_key_id: KeyID = voter_identity
            .public_keys
            .last_key_value()
            .map(|(last_key_id, _)| last_key_id + 1)
            .unwrap_or(0);
        let to_disable = voter_identity
            .public_keys
            .iter()
            .filter(|(_, pk)| pk.disabled_at.is_none())
            .map(|(id, _)| id.clone())
            .collect::<Vec<KeyID>>();

        // we need to build the new key
        let new_voter_key = Self::get_voter_identity_key(
            masternode
                .state_diff
                .voting_address
                .expect("confirmed is some"),
            new_key_id,
        )?;

        let current_time = Utc::now().timestamp_millis() as TimestampMillis;

        self.drive.disable_identity_keys(
            voter_identifier,
            to_disable,
            current_time,
            block_info,
            true,
            transaction,
        );
        // add the new key
        self.drive.add_new_non_unique_keys_to_identity(
            voter_identifier,
            vec![new_voter_key],
            block_info,
            true,
            transaction,
        );
        Ok(())
    }

    fn update_operator_identity(
        &self,
        masternode: &UpdatedMasternodeItem,
        block_info: &BlockInfo,
        state: &PlatformState,
        transaction: Option<&Transaction>,
    ) -> Result<(), Error> {
        // TODO: key type seems fragile might be better to use purpose

        if masternode.state_diff.pub_key_operator.is_none()
            && masternode.state_diff.operator_payout_address.is_none()
            && masternode.state_diff.platform_node_id.is_none()
        {
            return Ok(());
        }

        // we will perform at least one update, proceed to get the current identity
        let protx_hash: &ProTxHash = &masternode.protx_hash;
        // TODO: masternode is not really in state right, this error is not appropriate
        let old_masternode = state.full_masternode_list.get(protx_hash).ok_or_else(|| {
            Error::Abci(AbciError::InvalidState(
                "expected masternode to be in state".to_string(),
            ))
        })?;
        let operator_identifier = Self::get_operator_identifier(&old_masternode)?;

        let operator_identity = self
            .drive
            .fetch_full_identity(operator_identifier, transaction)?
            .ok_or_else(|| {
                Error::Abci(AbciError::InvalidState(
                    "expected identity to be in state".to_string(),
                ))
            })?;

        let mut new_key_id: KeyID = operator_identity
            .public_keys
            .last_key_value()
            .map(|(last_key_id, _)| last_key_id + 1)
            .unwrap_or(0);

        let mut keys_to_disable: HashSet<KeyID> = HashSet::new();
        let mut keys_to_create: Vec<IdentityPublicKey> = Vec::new();

        // now we need to handle each key
        if masternode.state_diff.pub_key_operator.is_some() {
            // we need to get the keys to disable
            let to_disable = operator_identity
                .public_keys
                .iter()
                .filter(|(_, pk)| pk.disabled_at.is_none() && pk.key_type == KeyType::BLS12_381)
                .map(|(id, _)| id.clone())
                .collect::<Vec<KeyID>>();
            keys_to_disable.extend(to_disable);

            let new_key = IdentityPublicKey {
                id: new_key_id,
                key_type: KeyType::BLS12_381,
                purpose: Purpose::AUTHENTICATION, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(
                    masternode
                        .state_diff
                        .pub_key_operator
                        .clone()
                        .expect("confirmed is some"),
                ),
                disabled_at: None,
            };
            keys_to_create.push(new_key);
            new_key_id = new_key_id + 1;
        }

        if masternode.state_diff.operator_payout_address.is_some() {
            let to_disable = operator_identity
                .public_keys
                .iter()
                .filter(|(_, pk)| pk.disabled_at.is_none() && pk.key_type == KeyType::ECDSA_HASH160)
                .map(|(id, _)| id.clone())
                .collect::<Vec<KeyID>>();
            keys_to_disable.extend(to_disable);

            let new_key = IdentityPublicKey {
                id: new_key_id,
                // key_type: KeyType::ECDSA_HASH160,
                // TODO: commented version is the correct one, disable to get it building
                key_type: KeyType::ECDSA_HASH160,
                purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                // TODO: can this be Some(None)
                data: BinaryData::new(
                    masternode
                        .state_diff
                        .operator_payout_address
                        .expect("confirmed is some")
                        .unwrap()
                        .to_vec(),
                ),
                disabled_at: None,
            };
            keys_to_create.push(new_key);
            new_key_id = new_key_id + 1;
        }

        if masternode.state_diff.platform_node_id.is_some() {
            let to_disable = operator_identity
                .public_keys
                .iter()
                .filter(|(_, pk)| {
                    pk.disabled_at.is_none() && pk.key_type == KeyType::ECDSA_SECP256K1
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<KeyID>>();
            keys_to_disable.extend(to_disable);

            let new_key = IdentityPublicKey {
                id: new_key_id,
                // key_type: KeyType::EDDSA_25519_HASH160,
                // TODO: commented version is the correct one, disable to get it building
                key_type: KeyType::ECDSA_SECP256K1,
                // purpose: Purpose::SYSTEM,
                // TODO: commented version is the correct one, disable to get it building
                purpose: Purpose::DECRYPTION,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                // TODO: this should be the node id
                data: BinaryData::new(
                    masternode
                        .state_diff
                        .payout_address
                        .expect("confirmed is some")
                        .to_vec(),
                ),
                disabled_at: None,
            };
            keys_to_create.push(new_key);
            new_key_id = new_key_id + 1;
        }

        let current_time = Utc::now().timestamp_millis() as TimestampMillis;

        self.drive.disable_identity_keys(
            operator_identifier,
            keys_to_disable.into_iter().collect(),
            current_time,
            block_info,
            true,
            transaction,
        );
        // add the new keys
        self.drive.add_new_non_unique_keys_to_identity(
            operator_identifier,
            keys_to_create,
            block_info,
            true,
            transaction,
        );

        Ok(())
    }

    fn disable_identity_keys(
        &self,
        masternode: &RemovedMasternodeItem,
        block_info: &BlockInfo,
        state: &PlatformState,
        transaction: Option<&Transaction>,
    ) -> Result<(), Error> {
        let protx_hash: &ProTxHash = &masternode.protx_hash;
        let old_masternode = state.full_masternode_list.get(protx_hash).ok_or_else(|| {
            Error::Abci(AbciError::InvalidState(
                "expected masternode to be in state".to_string(),
            ))
        })?;

        let owner_identifier = Self::get_owner_identifier(&old_masternode)?;
        let operator_identifier = Self::get_operator_identifier(&old_masternode)?;
        let voter_identifer = Self::get_voter_identifier(&old_masternode)?;

        let owner_identity = self
            .drive
            .fetch_full_identity(owner_identifier, transaction)?
            .unwrap();
        let operator_identity = self
            .drive
            .fetch_full_identity(operator_identifier, transaction)?
            .unwrap();
        let voter_identity = self
            .drive
            .fetch_full_identity(voter_identifer, transaction)?
            .unwrap();

        let mut keys_to_disable = HashSet::new();
        keys_to_disable.extend(
            owner_identity
                .public_keys
                .iter()
                .filter(|(_, pk)| pk.disabled_at.is_none())
                .map(|(id, _)| id.clone()),
        );
        keys_to_disable.extend(
            operator_identity
                .public_keys
                .iter()
                .filter(|(_, pk)| pk.disabled_at.is_none())
                .map(|(id, _)| id.clone()),
        );
        keys_to_disable.extend(
            voter_identity
                .public_keys
                .iter()
                .filter(|(_, pk)| pk.disabled_at.is_none())
                .map(|(id, _)| id.clone()),
        );

        let current_time = Utc::now().timestamp_millis() as TimestampMillis;

        self.drive.disable_identity_keys(
            operator_identifier,
            keys_to_disable.into_iter().collect(),
            current_time,
            block_info,
            true,
            transaction,
        );

        Ok(())
    }

    fn create_owner_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let owner_identifier = Self::get_owner_identifier(&masternode)?;
        let mut identity = Self::create_basic_identity(owner_identifier);
        identity.add_public_keys([Self::get_owner_identity_key(
            masternode.state.payout_address.clone(),
            0,
        )?]);
        Ok(identity)
    }

    fn create_voter_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let voting_identifier = Self::get_voter_identifier(&masternode)?;
        let mut identity = Self::create_basic_identity(voting_identifier);
        identity.add_public_keys([Self::get_voter_identity_key(
            masternode.state.voting_address.clone(),
            0,
        )?]);
        Ok(identity)
    }

    fn create_operator_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let operator_identifier = Self::get_operator_identifier(&masternode)?;
        let mut identity = Self::create_basic_identity(operator_identifier);
        identity.add_public_keys(self.get_operator_identity_keys(
            masternode.state.pub_key_operator.clone(),
            masternode.state.operator_payout_address.clone(),
            masternode.state.platform_node_id.clone(),
        )?);

        Ok(identity)
    }

    fn get_owner_identity_key(
        payout_address: [u8; 20],
        key_id: KeyID,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW,
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(payout_address.to_vec()),
            disabled_at: None,
        })
    }

    fn get_voter_identity_key(
        voting_address: [u8; 20],
        key_id: KeyID,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(voting_address.to_vec()),
            disabled_at: None,
        })
    }

    fn get_operator_identity_keys(
        &self,
        pub_key_operator: Vec<u8>,
        operator_payout_address: Option<[u8; 20]>,
        platform_node_id: Option<[u8; 20]>,
    ) -> Result<Vec<IdentityPublicKey>, Error> {
        let mut identity_public_keys = vec![IdentityPublicKey {
            id: 0,
            key_type: KeyType::BLS12_381,
            purpose: Purpose::AUTHENTICATION, // todo: is this purpose correct??
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(pub_key_operator),
            disabled_at: None,
        }];
        if let Some(operator_payout_address) = operator_payout_address {
            identity_public_keys.push(IdentityPublicKey {
                id: 1,
                // key_type: KeyType::ECDSA_HASH160,
                // TODO: commented version is the correct one, disable to get it building
                key_type: KeyType::ECDSA_HASH160,
                purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                // TODO: this should be the operator payout address
                data: BinaryData::new(operator_payout_address.to_vec()),
                disabled_at: None,
            });
        }
        if let Some(node_id) = platform_node_id {
            identity_public_keys.push(IdentityPublicKey {
                id: 2,
                // key_type: KeyType::EDDSA_25519_HASH160,
                // TODO: commented version is the correct one, disable to get it building
                key_type: KeyType::ECDSA_SECP256K1,
                // purpose: Purpose::SYSTEM,
                // TODO: commented version is the correct one, disable to get it building
                purpose: Purpose::DECRYPTION,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(node_id.to_vec()),
                disabled_at: None,
            });
        }

        Ok(identity_public_keys)
    }

    fn get_owner_identifier(masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        let masternode_identifier: [u8; 32] = masternode.protx_hash.clone().into_inner();
        Ok(masternode_identifier)
    }

    fn get_operator_identifier(masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        let protx_hash = &masternode.protx_hash.into_inner();
        let operator_pub_key = masternode.state.pub_key_operator.as_slice();
        let operator_identifier = Self::hash_concat_protxhash(protx_hash, operator_pub_key)?;
        Ok(operator_identifier)
    }

    fn get_voter_identifier(masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        let protx_hash = &masternode.protx_hash.into_inner();
        let voting_address = masternode.state.voting_address.as_slice();
        let voting_identifier = Self::hash_concat_protxhash(protx_hash, voting_address)?;
        Ok(voting_identifier)
    }

    fn hash_concat_protxhash(protx_hash: &[u8; 32], key_data: &[u8]) -> Result<[u8; 32], Error> {
        let mut hasher = Sha256::new();
        hasher.update(protx_hash);
        hasher.update(key_data);
        // TODO: handle unwrap, use custom error
        Ok(hasher.finalize().try_into().unwrap())
    }

    fn create_basic_identity(id: [u8; 32]) -> Identity {
        Identity {
            protocol_version: IDENTITY_PROTOCOL_VERSION,
            id: Identifier::new(id),
            revision: 1,
            balance: 0,
            asset_lock_proof: None,
            metadata: None,
            public_keys: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform;
    use crate::platform::Platform;
    use crate::rpc::core::CoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dashcore::ProTxHash;
    use dashcore_rpc::dashcore_rpc_json::MasternodeListDiffWithMasternodes;
    use dashcore_rpc::json::MasternodeType::Regular;
    use dashcore_rpc::json::{DMNState, MasternodeListItem, UpdatedMasternodeItem};
    use std::net::SocketAddr;
    use std::str::FromStr;

    // thinking of creating a function that returns identity creation instructions based on the masternode list diff
    // this way I can confirm that it is doing things correctly on the test level
    // maybe two functions, 1 for the creation, another for update and another for deletion
    // but don't think this is the best approach as the list might be very long and we don't want to
    // store too much information in ram
    // what should the result of an update function look like?
    // it should return the key id's to disable and the new set of public keys to add.
    // alright, let's focus on creation first
    // we need to pass it the list of added master nodes
    // we run into the batching problem with that, what we really want is a function that takes
    // a sinlge masternode list item and then returns the correct identity.
    // update also works for a very specific identity, hence we are testing on the specific identity level
    // so create_owner_id ...
    // update_owner_id ...
    // we currently have the creation function, but it needs the identifier, is this the case anymore?
    // we needed to remove the identifier because we had to retrieve before we knew if it was an update or not
    // but this is no longer the case, so we can just combine it into one step

    fn get_masternode_list_diff() -> MasternodeListDiffWithMasternodes {
        // TODO: eventually generate this from json
        MasternodeListDiffWithMasternodes {
            base_height: 850000,
            block_height: 867165,
            added_mns: vec![MasternodeListItem {
                node_type: Regular,
                protx_hash: ProTxHash::from_str(
                    "1628e387a7badd30fd4ee391ae0cab7e3bc84e792126c6b7cccd99257dad741d",
                )
                .expect("expected pro_tx_hash"),
                collateral_hash: hex::decode(
                    "4fde102b0c14c50d58d01cc7a53f9a73ae8283dcfe3f13685682ac6dd93f6210",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                collateral_index: 1,
                operator_reward: 0,
                state: DMNState {
                    service: SocketAddr::from_str("1.2.3.4:1234").unwrap(),
                    registered_height: 0,
                    pose_revived_height: 0,
                    pose_ban_height: 850091,
                    revocation_reason: 0,
                    owner_address: [0; 20],
                    voting_address: [0; 20],
                    payout_address: [0; 20],
                    pub_key_operator: [0; 48].to_vec(),
                    operator_payout_address: None,
                    platform_node_id: None,
                },
            }],
            updated_mns: vec![],
            removed_mns: vec![],
        }
    }

    #[test]
    fn test_owner_identity() {
        // todo: get rid of the multiple configs
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let mn_diff = get_masternode_list_diff();
        let added_mn_one = &mn_diff.added_mns[0];
        let owner_identity = platform.create_owner_identity(added_mn_one).unwrap();

        dbg!(owner_identity);
        // TODO: perform proper assertions when you have correct data
        //  just adding this test to guide development and make sure things
        //  are semi working
    }

    #[test]
    fn test_voting_identity() {
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let mn_diff = get_masternode_list_diff();
        let added_mn_one = &mn_diff.added_mns[0];
        let voter_identity = platform.create_voter_identity(added_mn_one).unwrap();

        dbg!(voter_identity);
    }

    #[test]
    fn test_operator_identity() {
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let mn_diff = get_masternode_list_diff();
        let added_mn_one = &mn_diff.added_mns[0];
        let operator_identity = platform.create_operator_identity(added_mn_one).unwrap();

        dbg!(operator_identity);
    }

    #[test]
    fn test_update_owner_identity() {}
}
