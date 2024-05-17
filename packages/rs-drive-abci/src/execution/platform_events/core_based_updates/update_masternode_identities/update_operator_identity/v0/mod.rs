use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dashcore_rpc::dashcore::ProTxHash;

use dashcore_rpc::json::DMNStateDiff;
use dpp::block::block_info::BlockInfo;

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::Purpose::TRANSFER;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::IdentityOperationType::{
    AddNewIdentity, AddNewKeysToIdentity, DisableIdentityKeys,
};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn update_operator_identity_v0(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        _block_info: &BlockInfo,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
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

        let old_masternode = platform_state
            .full_masternode_list()
            .get(pro_tx_hash)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCachedState(format!(
                    "expected masternode {} to be in state",
                    pro_tx_hash
                )))
            })?;

        let old_operator_identifier = Self::get_operator_identifier_from_masternode_list_item(
            old_masternode,
            platform_version,
        )?;

        let mut new_masternode = old_masternode.clone();

        new_masternode.state.apply_diff(state_diff.clone());

        let new_operator_identifier = Self::get_operator_identifier_from_masternode_list_item(
            &new_masternode,
            platform_version,
        )?;

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
                platform_version,
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
                        if Some(key.data().as_slice())
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
                        } else if state_diff.platform_node_id.as_ref().unwrap().as_slice()
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
                    identity_id: new_operator_identifier,
                    keys_ids: old_operator_identity_key_ids_to_disable,
                }));
            }

            let mut keys_to_re_enable = vec![];
            let unique_keys_to_add = vec![];
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
                        state_diff
                            .platform_node_id
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
                if let Some(new_operator_payout_address) = state_diff
                    .operator_payout_address
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
                    identity_id: old_operator_identifier,
                    keys_ids: old_operator_identity_key_ids_to_disable,
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
            let mut identity =
                Identity::create_basic_identity(new_operator_identifier, platform_version)?;
            identity.add_public_keys(Self::get_operator_identity_keys(
                state_diff
                    .pub_key_operator
                    .as_ref()
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
        }
        Ok(())
    }
}
