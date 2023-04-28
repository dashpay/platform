use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use chrono::Utc;
use dashcore::hashes::Hash;
use dashcore::ProTxHash;
use dashcore_rpc::dashcore_rpc_json::MasternodeType::{HighPerformance, Regular};
use dashcore_rpc::dashcore_rpc_json::{MasternodeListDiff, MasternodeType};
use dashcore_rpc::json::{
    DMNStateDiff, MasternodeListDiffWithMasternodes, MasternodeListItem, RemovedMasternodeItem,
    UpdatedMasternodeItem,
};
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::factory::IDENTITY_PROTOCOL_VERSION;
use dpp::identity::Purpose::WITHDRAW;
use dpp::identity::{
    Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel, TimestampMillis,
};
use dpp::platform_value::BinaryData;
use drive::drive::batch::DriveOperation;
use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::IdentityOperationType::{
    AddNewIdentity, AddNewKeysToIdentity, DisableIdentityKeys, ReEnableIdentityKeys,
};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyIDIdentityPublicKeyPairVec,
    KeyIDVec, KeyRequestType,
};
use drive::grovedb::Transaction;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update of the masternode identities
    pub fn update_masternode_identities(
        &self,
        masternode_diff: MasternodeListDiff,
        removed_masternodes: &BTreeMap<ProTxHash, MasternodeListItem>,
        block_info: &BlockInfo,
        platform_state: Option<&PlatformState>,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let MasternodeListDiff {
            mut added_mns,
            mut updated_mns,
            ..
        } = masternode_diff;

        // We should don't trust the order of added mns or updated mns

        // Sort added_mns based on pro_tx_hash
        added_mns.sort_by(|a, b| a.pro_tx_hash.cmp(&b.pro_tx_hash));

        // Sort updated_mns based on pro_tx_hash (the first element of the tuple)
        updated_mns.sort_by(|a, b| a.0.cmp(&b.0));

        let mut drive_operations = vec![];

        for masternode in added_mns {
            let owner_identity = self.create_owner_identity(&masternode)?;
            let voter_identity =
                self.create_voter_identity_from_masternode_list_item(&masternode)?;
            let operator_identity = self.create_operator_identity(&masternode)?;

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: owner_identity,
            }));

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: voter_identity,
            }));

            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: operator_identity,
            }));
        }

        if let Some(platform_state) = platform_state {
            // On initialization there is no platform state, but we also don't need to update
            // masternode identities.
            for update in updated_mns.iter() {
                self.update_owner_withdrawal_address(
                    update,
                    block_info,
                    transaction,
                    &mut drive_operations,
                )?;
                self.update_voter_identity(
                    update,
                    block_info,
                    platform_state,
                    transaction,
                    &mut drive_operations,
                )?;
                self.update_operator_identity(
                    update,
                    block_info,
                    platform_state,
                    transaction,
                    &mut drive_operations,
                )?;
            }

            for masternode in removed_masternodes.values() {
                self.disable_identity_keys(
                    masternode,
                    block_info,
                    transaction,
                    &mut drive_operations,
                )?;
            }
        }

        self.drive
            .apply_drive_operations(drive_operations, true, block_info, Some(transaction))?;

        Ok(())
    }

    fn update_owner_withdrawal_address(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let (pro_tx_hash, state_diff) = masternode;
        let Some(new_withdrawal_address) = state_diff.payout_address else {
            return Ok(());
        };

        let owner_identifier: [u8; 32] = pro_tx_hash.into_inner();

        let key_request = IdentityKeysRequest {
            identity_id: owner_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let old_withdrawal_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                key_request,
                Some(transaction),
            )?;

        if old_withdrawal_identity_keys.is_empty() {
            return Err(Error::Execution(ExecutionError::DriveMissingData(
                "expected masternode owner identity to be in state".to_string(),
            )));
        }

        let last_key_id = *old_withdrawal_identity_keys.keys().max().unwrap(); //todo

        let key_ids_to_disable = old_withdrawal_identity_keys
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.disabled_at.is_some() {
                    None //No need to disable it again
                } else {
                    Some(key_id)
                }
            })
            .collect();

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: owner_identifier,
            keys_ids: key_ids_to_disable,
            disable_at: block_info.time_ms,
        }));

        // add the new key
        let new_owner_key = Self::get_owner_identity_key(new_withdrawal_address, last_key_id + 1)?;

        drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
            identity_id: owner_identifier,
            unique_keys_to_add: vec![],
            non_unique_keys_to_add: vec![new_owner_key],
        }));

        Ok(())
    }

    /// When a voter identity is updated the following events need to happen
    /// The old identity key is disabled (which might make the identity unusable)
    /// A new identity is added with the new key, this new key is a non unique key.
    fn update_voter_identity(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let (pro_tx_hash, state_diff) = masternode;
        let Some(new_voting_address) = state_diff.voting_address else {
            return Ok(());
        };

        let old_masternode = platform_state
            .full_masternode_list
            .get(pro_tx_hash)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCachedState(
                    "expected masternode to be in state",
                ))
            })?;

        let old_voter_identifier =
            Self::get_voter_identifier_from_masternode_list_item(old_masternode)?;

        let key_request = IdentityKeysRequest {
            identity_id: old_voter_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let old_voter_identity_key_ids = self
            .drive
            .fetch_identity_keys::<KeyIDVec>(key_request, Some(transaction))?;

        if old_voter_identity_key_ids.is_empty() {
            return Err(Error::Execution(ExecutionError::DriveMissingData(
                "expected masternode voter identity to be in state".to_string(),
            )));
        }

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: old_voter_identifier,
            keys_ids: old_voter_identity_key_ids,
            disable_at: block_info.time_ms,
        }));

        // Part 2 : Create or Update Voting identity based on new key
        let new_voter_identity =
            Self::create_voter_identity(pro_tx_hash.as_inner(), &new_voting_address)?;

        // Let's check if the voting identity already exists
        let key_request = IdentityKeysRequest {
            identity_id: new_voter_identity.id.to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };
        let new_voter_identity_key_ids = self
            .drive
            .fetch_identity_keys::<KeyIDVec>(key_request, Some(transaction))?;

        // two possibilities
        if !new_voter_identity_key_ids.is_empty() {
            // first is that the new voter key already existed
            // if it is disabled re-enable it

            if new_voter_identity_key_ids.len() > 1 {
                return Err(Error::Execution(ExecutionError::DriveIncoherence(
                    "more than one masternode voter identity for an address and pro_tx_hash pair",
                )));
            }

            drive_operations.push(IdentityOperation(ReEnableIdentityKeys {
                identity_id: old_voter_identifier,
                keys_ids: new_voter_identity_key_ids,
            }));
        } else {
            // other is that the
            drive_operations.push(IdentityOperation(AddNewIdentity {
                identity: new_voter_identity,
            }));
        }
        Ok(())
    }

    fn update_operator_identity(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let (pro_tx_hash, state_diff) = masternode;

        if state_diff.pub_key_operator.is_none()
            && state_diff.operator_payout_address.is_none()
            && state_diff.platform_node_id.is_none()
        {
            return Ok(());
        }

        let needs_change_operator_payout_address = state_diff.operator_payout_address.is_some();
        let needs_change_platform_node_id = state_diff.platform_node_id.is_some();

        let mut old_masternode = platform_state
            .full_masternode_list
            .get(pro_tx_hash)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCachedState(
                    "expected masternode to be in state",
                ))
            })?;

        let old_operator_identifier =
            Self::get_operator_identifier_from_masternode_list_item(old_masternode)?;

        let mut new_masternode = old_masternode.clone();

        new_masternode.state.apply_diff(state_diff.clone());

        let new_operator_identifier =
            Self::get_operator_identifier_from_masternode_list_item(&new_masternode)?;

        let key_request = IdentityKeysRequest {
            identity_id: old_operator_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let old_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                key_request,
                Some(transaction),
            )?;

        // two possibilities, same identity or identity switch.
        if new_operator_identifier == old_operator_identifier {
            // we are on same identity for platform

            let mut old_operator_node_id_to_re_enable = None;

            let mut old_operator_payout_address_to_re_enable = None;

            let last_key_id = *old_identity_keys.keys().max().unwrap(); //todo

            let old_operator_identity_key_ids_to_disable: Vec<KeyID> = old_identity_keys
                .into_iter()
                .filter_map(|(key_id, key)| {
                    // We can disable previous withdrawal keys as we are adding a new one
                    if needs_change_operator_payout_address {
                        if Some(key.data.as_slice())
                            == old_masternode
                                .state
                                .operator_payout_address
                                .as_ref()
                                .map(|bytes| bytes.as_slice())
                        {
                            return Some(key_id);
                        } else if let Some(operator_payout_address) =
                            state_diff.operator_payout_address.as_ref().unwrap()
                        {
                            // an old key that we need to re-enable
                            if key.data.as_slice() == operator_payout_address.as_slice() {
                                old_operator_payout_address_to_re_enable = Some(key_id);
                            }
                        }
                    }
                    if needs_change_platform_node_id
                        && old_masternode.state.platform_node_id.is_some()
                    {
                        if key.data.as_slice()
                            == old_masternode.state.platform_node_id.as_ref().unwrap()
                        {
                            return Some(key_id);
                        } else if state_diff.platform_node_id.as_ref().unwrap().as_slice()
                            == key.data.as_slice()
                        {
                            old_operator_node_id_to_re_enable = Some(key_id);
                        }
                    }
                    None
                })
                .collect();

            if !old_operator_identity_key_ids_to_disable.is_empty() {
                drive_operations.push(IdentityOperation(DisableIdentityKeys {
                    identity_id: new_operator_identifier,
                    keys_ids: old_operator_identity_key_ids_to_disable,
                    disable_at: block_info.time_ms,
                }));
            }

            let mut keys_to_re_enable = vec![];
            let mut unique_keys_to_add = vec![];
            let mut non_unique_keys_to_add = vec![];

            let mut new_key_id = last_key_id + 1;

            if let Some(old_operator_pub_key_to_re_enable) = old_operator_node_id_to_re_enable {
                keys_to_re_enable.push(old_operator_pub_key_to_re_enable);
            } else if needs_change_platform_node_id {
                let key = IdentityPublicKey {
                    id: new_key_id,
                    key_type: KeyType::EDDSA_25519_HASH160,
                    purpose: Purpose::SYSTEM,
                    security_level: SecurityLevel::CRITICAL,
                    read_only: true,
                    data: BinaryData::new(
                        state_diff
                            .platform_node_id
                            .as_ref()
                            .expect("platform node id confirmed is some")
                            .to_vec(),
                    ),
                    disabled_at: None,
                };
                non_unique_keys_to_add.push(key);
                new_key_id += 1;
            }

            if let Some(old_operator_payout_address_to_re_enable) =
                old_operator_payout_address_to_re_enable
            {
                keys_to_re_enable.push(old_operator_payout_address_to_re_enable);
            } else if needs_change_operator_payout_address {
                if let Some(new_operator_payout_address) = state_diff
                    .operator_payout_address
                    .as_ref()
                    .expect("operator_payout_address confirmed is some")
                {
                    let key = IdentityPublicKey {
                        id: new_key_id,
                        key_type: KeyType::ECDSA_HASH160,
                        purpose: WITHDRAW,
                        security_level: SecurityLevel::CRITICAL,
                        read_only: true,
                        data: BinaryData::new(new_operator_payout_address.to_vec()),
                        disabled_at: None,
                    };
                    non_unique_keys_to_add.push(key);
                    new_key_id += 1;
                }
            }

            drive_operations.push(IdentityOperation(AddNewKeysToIdentity {
                identity_id: new_operator_identifier,
                unique_keys_to_add,
                non_unique_keys_to_add,
            }));
        } else {
            // We can not disable previous withdrawal keys,
            // Let's disable other two keys
            let old_operator_identity_key_ids_to_disable: Vec<KeyID> = old_identity_keys
                .into_iter()
                .filter_map(|(key_id, key)| {
                    if key.data.as_slice() == old_masternode.state.pub_key_operator {
                        //the old key
                        return Some(key_id);
                    }
                    if old_masternode.state.platform_node_id.is_some() {
                        if key.data.as_slice()
                            == old_masternode.state.platform_node_id.as_ref().unwrap()
                        {
                            return Some(key_id);
                        }
                    }
                    None
                })
                .collect();

            if !old_operator_identity_key_ids_to_disable.is_empty() {
                drive_operations.push(IdentityOperation(DisableIdentityKeys {
                    identity_id: old_operator_identifier,
                    keys_ids: old_operator_identity_key_ids_to_disable,
                    disable_at: block_info.time_ms,
                }));
            }
            let new_payout_address =
                if let Some(operator_payout_address) = state_diff.operator_payout_address {
                    operator_payout_address
                } else {
                    // we need to use the old pub_key_operator
                    old_masternode.state.operator_payout_address
                };

            let new_platform_node_id = if let Some(platform_node_id) = state_diff.platform_node_id {
                // if it changed it means it always existed
                Some(platform_node_id)
            } else {
                // we need to use the old pub_key_operator
                old_masternode.state.platform_node_id
            };
            // Now we need to create the new operator identity with the new keys
            let mut identity = Self::create_basic_identity(new_operator_identifier);
            identity.add_public_keys(
                self.get_operator_identity_keys(
                    state_diff
                        .pub_key_operator
                        .as_ref()
                        .expect("expected a pub key operator")
                        .clone(),
                    new_payout_address,
                    new_platform_node_id,
                )?,
            );
            drive_operations.push(IdentityOperation(AddNewIdentity { identity }));
        }
        Ok(())
    }

    fn disable_identity_keys(
        &self,
        old_masternode: &MasternodeListItem,
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let operator_identifier =
            Self::get_operator_identifier_from_masternode_list_item(old_masternode)?;
        let voter_identifier =
            Self::get_voter_identifier_from_masternode_list_item(old_masternode)?;

        let operator_key_request = IdentityKeysRequest {
            identity_id: operator_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let voter_key_request = IdentityKeysRequest {
            identity_id: voter_identifier,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let operator_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
                operator_key_request,
                Some(transaction),
            )?
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.is_disabled() {
                    None //No need to disable it again
                } else if key.purpose == WITHDRAW {
                    None //Don't disable withdrawal keys
                } else {
                    Some(key_id)
                }
            })
            .collect();
        let voter_identity_keys = self
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(
                voter_key_request,
                Some(transaction),
            )?
            .into_iter()
            .filter_map(|(key_id, key)| {
                if key.is_disabled() {
                    None //No need to disable it again
                } else {
                    Some(key_id)
                }
            })
            .collect();

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: operator_identifier,
            keys_ids: operator_identity_keys,
            disable_at: block_info.time_ms,
        }));

        drive_operations.push(IdentityOperation(DisableIdentityKeys {
            identity_id: operator_identifier,
            keys_ids: voter_identity_keys,
            disable_at: block_info.time_ms,
        }));

        Ok(())
    }

    fn create_owner_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Self::create_basic_identity(owner_identifier);
        identity.add_public_keys([Self::get_owner_identity_key(
            masternode.state.payout_address,
            0,
        )?]);
        Ok(identity)
    }

    fn create_voter_identity(
        pro_tx_hash: &[u8; 32],
        voting_key: &[u8; 20],
    ) -> Result<Identity, Error> {
        let voting_identifier = Self::get_voter_identifier(pro_tx_hash, voting_key)?;
        let mut identity = Self::create_basic_identity(voting_identifier);
        identity.add_public_keys([Self::get_voter_identity_key(*voting_key, 0)?]);
        Ok(identity)
    }

    fn create_voter_identity_from_masternode_list_item(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        Self::create_voter_identity(
            masternode.pro_tx_hash.as_inner(),
            &masternode.state.voting_address,
        )
    }

    fn create_operator_identity(&self, masternode: &MasternodeListItem) -> Result<Identity, Error> {
        let operator_identifier =
            Self::get_operator_identifier_from_masternode_list_item(masternode)?;
        let mut identity = Self::create_basic_identity(operator_identifier);
        identity.add_public_keys(self.get_operator_identity_keys(
            masternode.state.pub_key_operator.clone(),
            masternode.state.operator_payout_address,
            masternode.state.platform_node_id,
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
            security_level: SecurityLevel::CRITICAL,
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
            purpose: Purpose::VOTING,
            security_level: SecurityLevel::HIGH,
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
            purpose: Purpose::SYSTEM,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(pub_key_operator),
            disabled_at: None,
        }];
        if let Some(operator_payout_address) = operator_payout_address {
            identity_public_keys.push(IdentityPublicKey {
                id: 1,
                key_type: KeyType::ECDSA_HASH160,
                purpose: Purpose::WITHDRAW,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(operator_payout_address.to_vec()),
                disabled_at: None,
            });
        }
        if let Some(node_id) = platform_node_id {
            identity_public_keys.push(IdentityPublicKey {
                id: 2,
                key_type: KeyType::EDDSA_25519_HASH160,
                purpose: Purpose::SYSTEM,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(node_id.to_vec()),
                disabled_at: None,
            });
        }

        Ok(identity_public_keys)
    }

    fn get_owner_identifier(masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        let masternode_identifier: [u8; 32] = masternode.pro_tx_hash.into_inner();
        Ok(masternode_identifier)
    }

    fn get_operator_identifier(
        pro_tx_hash: &[u8; 32],
        pub_key_operator: &[u8],
    ) -> Result<[u8; 32], Error> {
        let operator_identifier = Self::hash_concat_protxhash(pro_tx_hash, pub_key_operator)?;
        Ok(operator_identifier)
    }

    fn get_operator_identifier_from_masternode_list_item(
        masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let pro_tx_hash = &masternode.pro_tx_hash.into_inner();
        Self::get_operator_identifier(pro_tx_hash, masternode.state.pub_key_operator.as_slice())
    }

    fn get_voter_identifier(
        pro_tx_hash: &[u8; 32],
        voting_address: &[u8; 20],
    ) -> Result<[u8; 32], Error> {
        let voting_identifier = Self::hash_concat_protxhash(pro_tx_hash, voting_address)?;
        Ok(voting_identifier)
    }

    fn get_voter_identifier_from_masternode_list_item(
        masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let pro_tx_hash = &masternode.pro_tx_hash.into_inner();
        let voting_address = &masternode.state.voting_address;
        Self::get_voter_identifier(pro_tx_hash, voting_address)
    }

    fn hash_concat_protxhash(pro_tx_hash: &[u8; 32], key_data: &[u8]) -> Result<[u8; 32], Error> {
        // todo: maybe change hash functions
        let mut hasher = Sha256::new();
        hasher.update(pro_tx_hash);
        hasher.update(key_data);
        // TODO: handle unwrap, use custom error
        Ok(hasher
            .finalize()
            .try_into()
            .expect("expected a 32 byte hash"))
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

/*
#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dashcore::ProTxHash;
    use dashcore_rpc::dashcore_rpc_json::MasternodeListDiffWithMasternodes;
    use dashcore_rpc::json::MasternodeType::Regular;
    use dashcore_rpc::json::{DMNState, MasternodeListItem};
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
                pro_tx_hash: ProTxHash::from_str(
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
*/
