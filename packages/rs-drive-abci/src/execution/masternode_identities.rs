use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::json::{Masternode, MasternodeListItem, ProTxHash, QuorumMasternodeListItem};
use dpp::identifier::Identifier;
use dpp::identity::factory::IDENTITY_PROTOCOL_VERSION;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
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
        identity.add_public_keys([self.get_owner_identity_key(&masternode)?]);
        Ok(identity)
    }

    fn get_owner_identity_key(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<IdentityPublicKey, Error> {
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

    fn create_voter_identity(
        &self,
        protx_hash: &[u8],
        masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let voting_identifier = Self::get_voter_identifier(protx_hash, &masternode)?;
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

    fn create_operator_identity(
        &self,
        protx_hash: &[u8],
        masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let operator_identifier = Self::get_operator_identifier(protx_hash, &masternode)?;
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
                let voter_identity = self.create_voter_identity(&protx_hash, &masternode)?;
                let operator_identity = self.create_operator_identity(&protx_hash, &masternode)?;
            }

            // how do we handle updated masternodes?
            // well from the state diff, we can know which identity changed
            // if an identity has changed, we retrieve the full identity
            // next, we need to disable the part that changed, which means we need to identity that part
            // need to get the next key id, need to disable the keys that should not be there e.t.c.
            // would be better off if I can get this building and actually write some tests
            // just test to make sure that the identities are created the way we want
            // and that we are doing the updates correctly.
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore_rpc_json::MasternodeListDiffWithMasternodes;
    use dashcore_rpc::json::MasternodeType::Regular;
    use dashcore_rpc::json::{DMNState, MasternodeListItem, ProTxHash};
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
            added_mns: vec![
                MasternodeListItem{
                    node_type: Regular,
                    protx_hash: ProTxHash::from("1628e387a7badd30fd4ee391ae0cab7e3bc84e792126c6b7cccd99257dad741d"),
                    collateral_hash: hex::decode("4fde102b0c14c50d58d01cc7a53f9a73ae8283dcfe3f13685682ac6dd93f6210").unwrap().try_into().unwrap(),
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
                        owner_address: hex::decode("yT5qnTnzEe6a3qAaiaC6eG5XZgoyYErppd").unwrap(),
                        voting_address: hex::decode("yccz9JhWMQsdeNAJ8TQ5JtNFXqUxk7dAS2").unwrap(),
                        payout_address: hex::decode("yeBF5m81EKyr44JXgzFzj8ppSkS1R9vdz9").unwrap(),
                        pub_key_operator: hex::decode("b9ba4890073eda0df9987857a9ecc4a47e25d0ca475e33586fae44684ef1d703d64a33f52bb93ba0fe5e81f832469fb4").unwrap()
                        operator_payout_address: None,
                    }
                }
            ],
            updated_mns: vec![],
            removed_mns: vec![],
        }
    }

    #[test]
    fn test_owner_identity() {}
}
