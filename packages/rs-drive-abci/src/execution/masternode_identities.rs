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
    fn create_owner_identity(
        &self,
        masternode_identifier: [u8; 32],
        updated_masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let mut identity = Self::create_basic_identity(masternode_identifier);
        identity.add_public_keys([self.get_owner_identity_key(&updated_masternode)?]);
        Ok(identity)
    }

    fn get_owner_identity_key(
        &self,
        updated_masternode: &MasternodeListItem,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW,
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(updated_masternode.state.payout_address.clone()),
            disabled_at: None,
        })
    }

    fn create_voter_identity(
        &self,
        voting_identifier: [u8; 32],
        updated_masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let mut identity = Self::create_basic_identity(voting_identifier);
        identity.add_public_keys([self.get_voter_identity_key(&updated_masternode)?]);
        Ok(identity)
    }

    fn get_voter_identity_key(
        &self,
        updated_masternode: &MasternodeListItem,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKey {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::WITHDRAW, // todo: is this purpose correct??
            security_level: SecurityLevel::MASTER,
            read_only: true,
            data: BinaryData::new(updated_masternode.state.voting_address.clone()),
            disabled_at: None,
        })
    }

    fn create_operator_identity(
        &self,
        operator_identifier: [u8; 32],
        updated_masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let mut identity = Self::create_basic_identity(operator_identifier);
        identity.add_public_keys(self.get_operator_identity_keys(&updated_masternode)?);

        Ok(identity)
    }

    fn get_operator_identity_keys(
        &self,
        updated_masternode: &MasternodeListItem,
    ) -> Result<Vec<IdentityPublicKey>, Error> {
        Ok(vec![
            IdentityPublicKey {
                id: 0,
                key_type: KeyType::BLS12_381,
                purpose: Purpose::AUTHENTICATION, // todo: is this purpose correct??
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(updated_masternode.state.pub_key_operator.clone()),
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
                data: BinaryData::new(updated_masternode.state.payout_address.clone()),
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
                data: BinaryData::new(updated_masternode.state.payout_address.clone()),
                disabled_at: None,
            },
        ])
    }

    // TODO: this should take in a trait, so we can re-use this, right now we have to duplicate
    fn get_owner_identifier(updated_masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        // TODO: do proper error handling
        let masternode_identifier: [u8; 32] =
            updated_masternode.protx_hash.clone().0.try_into().unwrap();
        Ok(masternode_identifier)
    }

    fn get_operator_identifier(
        protx_hash: &[u8],
        updated_masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let operator_pub_key = updated_masternode.state.pub_key_operator.as_slice();
        let operator_identifier = Self::hash_concat_protxhash(protx_hash, operator_pub_key)?;
        Ok(operator_identifier)
    }

    fn get_voter_identifier(
        protx_hash: &[u8],
        updated_masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let voting_address = updated_masternode.state.voting_address.as_slice();
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

            // TODO: remove duplication
            // for the added masternodes, we just want to create the required identities
            for masternode in added_masternodes {
                let protx_hash = hex::decode(&masternode.protx_hash.0).unwrap();

                // create the owner identity
                let owner_identifier = Self::get_owner_identifier(&masternode)?;
                let owner_identity = self.create_owner_identity(owner_identifier, &masternode)?;
                // store the owner identity

                // create the voter identity
                let voter_identifier = Self::get_voter_identifier(&protx_hash, &masternode)?;
                let voter_identity = self.create_voter_identity(voter_identifier, &masternode)?;

                // create the operator identity
                let operator_identifier = Self::get_operator_identifier(&protx_hash, &masternode)?;
                let operator_identity = self.create_operator_identity(operator_identifier, &masternode)?;
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
