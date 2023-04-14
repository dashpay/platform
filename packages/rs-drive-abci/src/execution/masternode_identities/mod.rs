use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use chrono::Utc;
use dashcore_rpc::json::{
    Masternode, MasternodeListItem, ProTxHash, QuorumMasternodeListItem, UpdatedMasternodeItem,
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
use std::collections::BTreeMap;

// TODO: clean this file up

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    // TODO: store after identity creation
    fn create_owner_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let owner_identifier = Self::get_owner_identifier(&masternode)?;
        let mut identity = Self::create_basic_identity(owner_identifier);
        identity.add_public_keys([Self::get_owner_identity_key(&masternode)?]);
        Ok(identity)
    }

    fn get_owner_identity_key(masternode: &MasternodeListItem) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW,
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(masternode.state.payout_address.clone()),
            disabled_at: None,
        })
    }

    fn create_voter_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let protx_hash = &masternode.protx_hash.0;
        let voting_identifier = Self::get_voter_identifier(&protx_hash, &masternode)?;
        let mut identity = Self::create_basic_identity(voting_identifier);
        identity.add_public_keys([self.get_voter_identity_key(&masternode)?]);
        Ok(identity)
    }

    fn get_voter_identity_key(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(masternode.state.voting_address.clone()),
            disabled_at: None,
        })
    }

    fn create_operator_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let protx_hash = &masternode.protx_hash.0;
        let operator_identifier = Self::get_operator_identifier(&protx_hash, &masternode)?;
        let mut identity = Self::create_basic_identity(operator_identifier);
        identity.add_public_keys(self.get_operator_identity_keys(&masternode)?);

        Ok(identity)
    }

    fn get_operator_identity_keys(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<Vec<IdentityPublicKey>, Error> {
        Ok(vec![
            IdentityPublicKey {
                id: 0,
                key_type: KeyType::BLS12_381,
                purpose: Purpose::AUTHENTICATION, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(masternode.state.pub_key_operator.clone()),
                disabled_at: None,
            },
            IdentityPublicKey {
                id: 1,
                // key_type: KeyType::ECDSA_HASH160,
                // TODO: commented version is the correct one, disable to get it building
                key_type: KeyType::BLS12_381,
                purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                // TODO: this should be the operator payout address
                data: BinaryData::new(masternode.state.payout_address.clone()),
                disabled_at: None,
            },
            // TODO: this public key should be optionally created
            IdentityPublicKey {
                id: 2,
                // key_type: KeyType::EDDSA_25519_HASH160,
                // TODO: commented version is the correct one, disable to get it building
                key_type: KeyType::BLS12_381,
                // purpose: Purpose::SYSTEM,
                // TODO: commented version is the correct one, disable to get it building
                purpose: Purpose::DECRYPTION,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                // TODO: this should be the node id
                data: BinaryData::new(masternode.state.payout_address.clone()),
                disabled_at: None,
            },
        ])
    }

    // TODO: this should take in a trait, so we can re-use this, right now we have to duplicate
    fn get_owner_identifier(masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        // TODO: do proper error handling
        let masternode_identifier: [u8; 32] = masternode.protx_hash.clone().0.try_into().unwrap();
        Ok(masternode_identifier)
    }

    fn get_operator_identifier(
        protx_hash: &[u8],
        masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let operator_pub_key = masternode.state.pub_key_operator.as_slice();
        let operator_identifier = Self::hash_concat_protxhash(protx_hash, operator_pub_key)?;
        Ok(operator_identifier)
    }

    fn get_voter_identifier(
        protx_hash: &[u8],
        masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let voting_address = masternode.state.voting_address.as_slice();
        let voting_identifier = Self::hash_concat_protxhash(protx_hash, voting_address)?;
        Ok(voting_identifier)
    }

    fn hash_concat_protxhash(protx_hash: &[u8], key_data: &[u8]) -> Result<[u8; 32], Error> {
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

    fn update_owner_identity(
        &self,
        masternode: &UpdatedMasternodeItem,
        block_info: &BlockInfo,
        transaction: Option<&Transaction>,
    ) -> Result<(), Error> {
        // what would cause an error here???
        // need to check if we need to update the owner identity
        if masternode.state_diff.payout_address.is_none() {
            // need better feedback, this is not enough
            return Ok(());
        }

        // there is an update
        // we need to get the public keys to disable
        // sadly can't pass the updated master node item directly, so have to generate
        // the owner identifier here again
        // TODO: fix this!!!!
        let owner_identifier: [u8; 32] = masternode.protx_hash.clone().0.try_into().unwrap();
        // we need to get the full identity
        // TODO: return an actual error if the identity is None, as it should be Some
        let owner_identity = self
            .drive
            .fetch_full_identity(owner_identifier, transaction)?
            .unwrap();
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
        // we need to build the new key
        // TODO: make generic over the masternode type
        let new_owner_key = IdentityPublicKey {
            id: new_key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW,
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(
                masternode
                    .state_diff
                    .payout_address
                    .clone()
                    .expect("confirmed not none"),
            ),
            disabled_at: None,
        };
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

    // TODO: factor out duplication
    //  there is a common thread going on here
    //  get the next key id and the next public key
    //  figure out what you want to disable
    //  to make this generic, you need to pass an optional filter function
    //  specifically for the opreator identity
    fn update_voter_identity(
        &self,
        masternode: &UpdatedMasternodeItem,
        block_info: &BlockInfo,
        transaction: Option<&Transaction>,
    ) -> Result<(), Error> {
        // what would cause an error here???
        // need to check if we need to update the owner identity
        if masternode.state_diff.voting_address.is_none() {
            // need better feedback, this is not enough
            return Ok(());
        }

        // there is an update
        // we need to get the public keys to disable
        // sadly can't pass the updated master node item directly, so have to generate
        // the owner identifier here again
        // TODO: fix this!!!!
        let owner_identifier: [u8; 32] = masternode.protx_hash.clone().0.try_into().unwrap();
        // we need to get the full identity
        // TODO: return an actual error if the identity is None, as it should be Some
        let owner_identity = self
            .drive
            .fetch_full_identity(owner_identifier, transaction)?
            .unwrap();
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
        // we need to build the new key
        // TODO: make generic over the masternode type
        let new_owner_key = IdentityPublicKey {
            id: new_key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW,
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(
                masternode
                    .state_diff
                    .payout_address
                    .clone()
                    .expect("confirmed not none"),
            ),
            disabled_at: None,
        };
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

    /// Update of the masternode identities
    pub fn update_masternode_identities(
        &self,
        previous_core_height: u32,
        current_core_height: u32,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        if previous_core_height != current_core_height {
            let masternode_list_diff = self
                .core_rpc
                .get_protx_diff_with_masternodes(previous_core_height, current_core_height)?;
            let added_masternodes = masternode_list_diff.added_mns;
            let updated_masternodes = masternode_list_diff.updated_mns;

            // for the added masternodes, we just want to create the required identities
            for masternode in added_masternodes {
                let protx_hash = hex::decode(&masternode.protx_hash.0).unwrap();

                let owner_identity = self.create_owner_identity(&masternode)?;
                let voter_identity = self.create_voter_identity(&masternode)?;
                let operator_identity = self.create_operator_identity(&masternode)?;
            }

            // to update, we need to pass in an identity and a state diff,
            // based on the state diff, we need to
            // TODO: can the owner address ever be updated?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform;
    use crate::platform::Platform;
    use crate::rpc::core::CoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dashcore_rpc::dashcore_rpc_json::MasternodeListDiffWithMasternodes;
    use dashcore_rpc::json::MasternodeType::Regular;
    use dashcore_rpc::json::{DMNState, MasternodeListItem, ProTxHash, UpdatedMasternodeItem};
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
                protx_hash: ProTxHash::from(
                    "1628e387a7badd30fd4ee391ae0cab7e3bc84e792126c6b7cccd99257dad741d",
                ),
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
                    last_paid_height: 0,
                    consecutive_payments: 0,
                    pose_penalty: 519,
                    pose_revived_height: -1,
                    pose_ban_height: 850091,
                    revocation_reason: 0,
                    owner_address: vec![1],
                    voting_address: vec![2],
                    payout_address: vec![3],
                    pub_key_operator: vec![4],
                    operator_payout_address: None,
                },
            }],
            updated_mns: vec![
                // only updates the owner identifier
                UpdatedMasternodeItem {},
            ],
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
