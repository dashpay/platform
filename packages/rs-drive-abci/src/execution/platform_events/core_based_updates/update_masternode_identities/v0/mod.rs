use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformState;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::ProTxHash;
use dashcore_rpc::dashcore_rpc_json::MasternodeListDiff;
use dashcore_rpc::json::{DMNStateDiff, MasternodeListItem};
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::identity_factory::IDENTITY_PROTOCOL_VERSION;
use dpp::identity::Purpose::WITHDRAW;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;
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
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
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

    pub(crate) fn get_voter_identity_key(
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

    pub(crate) fn get_operator_identity_keys(
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

    fn get_operator_identifier(
        pro_tx_hash: &[u8; 32],
        pub_key_operator: &[u8],
    ) -> Result<[u8; 32], Error> {
        let operator_identifier = Self::hash_concat_protxhash(pro_tx_hash, pub_key_operator)?;
        Ok(operator_identifier)
    }

    pub(crate) fn get_operator_identifier_from_masternode_list_item(
        masternode: &MasternodeListItem,
    ) -> Result<[u8; 32], Error> {
        let pro_tx_hash = &masternode.pro_tx_hash.into_inner();
        Self::get_operator_identifier(pro_tx_hash, masternode.state.pub_key_operator.as_slice())
    }

    pub(crate) fn get_voter_identifier(
        pro_tx_hash: &[u8; 32],
        voting_address: &[u8; 20],
    ) -> Result<[u8; 32], Error> {
        let voting_identifier = Self::hash_concat_protxhash(pro_tx_hash, voting_address)?;
        Ok(voting_identifier)
    }

    pub(crate) fn get_voter_identifier_from_masternode_list_item(
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

    pub(crate) fn create_basic_identity(id: [u8; 32]) -> Identity {
        Identity {
            feature_version: IDENTITY_PROTOCOL_VERSION,
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
    use dashcore_rpc::dashcore::ProTxHash;
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
