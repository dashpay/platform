use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::identity::IdentityPublicKey;
use crate::drive::Drive;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::BatchInsertTreeApplyType::StatefulBatchInsertTree;
use crate::drive::identity::{identity_contract_info_root_path_vec, identity_path_vec};
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::object_size_info::PathKeyInfo;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee_pools::epochs::Epoch;

pub enum ContractApplyInfo {
    Keys(Vec<IdentityPublicKey>)
}

impl Drive {

    pub(crate) fn add_contract_info_operations(
        &self,
        identity_id: [u8; 32],
        contract_infos: Vec<([u8; 32], ContractApplyInfo)>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let storage_flags = StorageFlags::SingleEpoch(epoch.index);
        let identity_path = identity_path_vec(identity_id.as_slice());
        // we insert the contract root tree if it doesn't exist already
        self.batch_insert_empty_tree_if_not_exists(PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])), Some(&storage_flags), StatefulBatchInsertTree, transaction, &mut None,  &mut batch_operations)?;

        let identity_contract_root_path = identity_contract_info_root_path_vec(identity_id.as_slice());

        for (contract_id, contract_info) in contract_infos.into_iter() {
            self.batch_insert_empty_tree_if_not_exists(PathKeyInfo::<0>::PathKey((identity_contract_root_path.clone(), contract_id.to_vec())), Some(&storage_flags), StatefulBatchInsertTree, transaction, &mut None,  &mut batch_operations)?;
            match contract_info {
                ContractApplyInfo::Keys(keys) => {
                    self.add_new_keys_to_identity_operations(identity_id, keys, false, epoch, estimated_costs_only_with_layer_info, transaction)?;
                }
            }
        }
        Ok(batch_operations)
    }
}