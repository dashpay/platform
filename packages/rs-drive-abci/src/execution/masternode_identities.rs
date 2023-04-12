use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::json::{ProTxHash, QuorumMasternodeListItem};
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
    fn create_owner_identity(
        &self,
        protx_hash: &[u8],
        masternode_identifier: [u8; 32],
    ) -> Result<Identity, Error> {
        let mut identity = Self::create_basic_identity(masternode_identifier);
        identity.add_public_keys(self.get_owner_identity_key(protx_hash)?);
        Ok(identity)
    }

    fn get_owner_identity_key(&self, protx_hash: &[u8]) -> Result<IdentityPublicKey, Error> {
        let full_masternode_detail = self
            .core_rpc
            .get_protx_info(&ProTxHash(protx_hash.to_vec()))?;
        Ok(IdentityPublicKey {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW,
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(full_masternode_detail.state.payout_address),
            disabled_at: None,
        })
    }

    fn create_voter_identity(
        &self,
        voting_identifier: [u8; 32],
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<Identity, Error> {
        let mut identity = Self::create_basic_identity(voting_identifier);
        identity.add_public_keys(self.get_voter_identity_key(&updated_masternode)?);
        Ok(identity)
    }

    fn get_voter_identity_key(
        &self,
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(updated_masternode.voting_address.clone()),
            disabled_at: None,
        })
    }

    fn create_operator_identity(
        &self,
        protx_hash: &[u8],
        operator_identifier: [u8; 32],
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<Identity, Error> {
        let mut identity = Self::create_basic_identity(operator_identifier);
        identity.add_public_keys(self.get_operator_identity_keys(protx_hash, &updated_masternode)?);

        Ok(identity)
    }

    fn get_operator_identity_keys(
        &self,
        protx_hash: &[u8],
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<Vec<IdentityPublicKey>, Error> {
        let full_masternode_detail = self
            .core_rpc
            .get_protx_info(&ProTxHash(protx_hash.to_vec()))?;
        // TODO: js uses something called a Script to convert this to payoutPublicKey, what is this?
        let operator_payout_address = full_masternode_detail.state.payout_address;

        Ok(vec![
            IdentityPublicKey {
                id: 0,
                key_type: KeyType::BLS12_381,
                purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(updated_masternode.pub_key_operator.clone()),
                disabled_at: None,
            },
            IdentityPublicKey {
                id: 1,
                key_type: KeyType::BLS12_381,
                purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(operator_payout_address),
                disabled_at: None,
            },
        ])
    }

    fn get_owner_identifier(
        &self,
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        // TODO: do proper error handling
        let masternode_identifier: [u8; 32] = updated_masternode
            .pro_reg_tx_hash
            .clone()
            .try_into()
            .unwrap();
        Ok(masternode_identifier)
    }

    fn get_operator_identifier(
        protx_hash: &[u8],
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let operator_pub_key = updated_masternode.pub_key_operator.as_slice();
        let operator_identifier = Self::hash_concat_protxhash(protx_hash, operator_pub_key)?;
        Ok(operator_identifier)
    }

    fn get_voter_identifier(
        protx_hash: &[u8],
        updated_masternode: &QuorumMasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let voting_address = updated_masternode.voting_address.as_slice();
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
                .get_protx_diff(previous_core_height, current_core_height)?;
            let updated_masternodes = masternode_list_diff.mn_list;

            for updated_masternode in updated_masternodes {
                // Need to get the protx hash for this
                let protx_hash = hex::decode(&updated_masternode.pro_reg_tx_hash).unwrap();

                // TODO: remove duplication
                let owner_identifier =
                    Self::get_owner_identifier(&updated_masternode)?;
                let maybe_owner_identity = self
                    .drive
                    .fetch_full_identity(owner_identifier, Some(transaction))?;
                match maybe_owner_identity {
                    None => self.create_owner_identity(protx_hash.as_slice(), owner_identifier)?,
                    Some(owner_identity) => {
                        let latest_owner_pub_key = self.get_owner_identity_key(protx_hash.as_slice())?;
                        let mut found = false;
                        let mut to_disable = vec![];
                        // TODO: clean up
                        for (key, pub_key) in owner_identity.get_public_keys() {
                            // search the public key
                            // TODO: impl eq for IdentityPublicKey
                            if pub_key.data == latest_owner_pub_key.data {
                                found = true
                            } else {
                                to_disable.push(key.clone())
                            }
                        }
                        if !found {
                            // disable all current keys
                            self.drive.disable_identity_keys(voter_identifer, to_disable, todo!(), &block_info, true, Some(transaction));
                            // add the new key to the identity
                            self.drive.add_new_non_unique_keys_to_identity(owner_identifier, vec![latest_owner_pub_key], &block_info, true, Some(transaction));
                        }
                    }
                };

                let voter_identifer =
                    Self::get_voter_identifier(protx_hash.as_slice(), &updated_masternode)?;
                let maybe_voter_identity = self
                    .drive
                    .fetch_full_identity(voter_identifer, Some(transaction))?;
                match maybe_voter_identity {
                    None => self.create_voter_identity(voting_identifier, &updated_masternode)?,
                    Some(voter_identity) => {
                        // TODO: remove duplication
                        let latest_voter_public_key = self.get_voter_identity_key(&updated_masternode)?;
                        let mut found = false;
                        let mut to_disable = vec![];
                        for (key, pub_key) in voter_identity.get_public_keys() {
                            // search the public key
                            // TODO: impl eq for IdentityPublicKey
                            if pub_key.data == latest_voter_public_key.data {
                                found = true
                            } else {
                                to_disable.push(key.clone())
                            }
                        }
                        if !found {
                            // disable all current keys
                            self.drive.disable_identity_keys(voter_identifer, to_disable, todo!(), &block_info, true, Some(transaction));
                            // add the new key to the identity
                            self.drive.add_new_non_unique_keys_to_identity(voter_identifer, vec![latest_voter_public_key], &block_info, true, Some(transaction));
                        }

                    }
                    _ => todo!(),
                };

                let operator_identifier =
                    Self::get_operator_identifier(protx_hash.as_slice(), &updated_masternode)?;
                let maybe_operator_identity = self
                    .drive
                    .fetch_full_identity(operator_identifier, Some(transaction))?;
                let operator_identity = match maybe_operator_identity {
                    None => self.create_operator_identity(
                        protx_hash.as_slice(),
                        operator_identifier,
                        &updated_masternode,
                    )?,
                    _ => todo!(),
                };

                // next up if there is an identity, we need to check if the key is still the same
                // if it is not the same, we need to disable that key and then add a new key
                // might make sense to extract the key creation functions
                // how do we check if a key has changed tho??
                // we need to filter all the keys, for non disabled, then from this new key list
                // we need to check if the key we are looking for is there
                // if it is not there tho
                // when adding a key, we need to use the next available id

                // for x in owner_identity.public_keys.values() {
                //
                // }

                // need to fetch the identities associated with this masternode
                // self.drive.fetch_full_identity();
                // assuming no fetch and just creation

                //         // from the masternode, we need to get the identifiers for each type, then check if the key is what we expect
                //         // TODO: make into owner_identifier function
                //         // let protx_hash = hex::decode(&updated_masternode.pro_reg_tx_hash).unwrap();
                //         let voting_address = updated_masternode.clone().voting_address;
                //         let mut hasher = Sha256::new();
                //         // TODO: remove clone
                //         hasher.update([protx_hash, voting_address.clone()].concat().as_slice());
                //         let voting_identifier: [u8; 32] = hasher.finalize().try_into().unwrap();
                //
                //         let previous_voter_identity = self
                //             .drive
                //             .fetch_full_identity(voting_identifier, Some(transaction))?;
                //         if previous_voter_identity.is_none() {
                //             let voter_identity = self.create_voter_identity(updated_masternode.clone())?;
                //             let _ = self.drive.add_new_identity(
                //                 voter_identity,
                //                 block_info,
                //                 true,
                //                 Some(transaction),
                //             )?;
                //         } else {
                //             // we need to confirm there has been no change in the key
                //             // previous_voter_identity.unwrap().public_keys.values().fil
                //         }
                //
                //         // TODO: get rid of clone
                //         let owner_identity = self.create_owner_identity(updated_masternode.clone())?;
                //         let voter_identity = self.create_voter_identity(updated_masternode.clone())?;
                //         let operator_identity = self.create_operator_identity(updated_masternode)?;
                //
                //         // TODO: move this into the identity creation step
                //         // TODO: what do we do with the fee result
                //         let _ = self.drive.add_new_identity(
                //             owner_identity,
                //             block_info,
                //             true,
                //             Some(transaction),
                //         )?;
                //         let _ = self.drive.add_new_identity(
                //             voter_identity,
                //             block_info,
                //             true,
                //             Some(transaction),
                //         )?;
                //         let _ = self.drive.add_new_identity(
                //             operator_identity,
                //             block_info,
                //             true,
                //             Some(transaction),
                //         )?;
            }
            //     //todo:
            //     // self.drive.fetch_full_identity()
            //     // self.drive.add_new_non_unique_keys_to_identity()
        }
        Ok(())
    }
}
