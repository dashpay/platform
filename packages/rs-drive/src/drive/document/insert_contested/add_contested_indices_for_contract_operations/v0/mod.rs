use crate::drive::grove_operations::BatchInsertTreeApplyType;

use crate::drive::object_size_info::{
    DocumentAndContractInfo, DocumentInfoV0Methods, DriveKeyInfo, PathInfo,
};
use crate::drive::Drive;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::drive::defaults::{CONTESTED_DOCUMENT_REFERENCE_SIZE, DEFAULT_HASH_SIZE_U8, U8_SIZE_U8};
use crate::drive::votes::paths::vote_contested_resource_active_polls_contract_document_tree_path_vec;
use crate::error::drive::DriveError;
use dpp::data_contract::document_type::IndexProperty;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees};
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use crate::drive::document::document_reference_size;

impl Drive {
    /// Adds indices for the top index level and calls for lower levels.
    pub(crate) fn add_contested_indices_for_contract_operations_v0(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let owner_id = document_and_contract_info
            .owned_document_info
            .owner_id
            .ok_or(Error::Drive(DriveError::ContestedDocumentMissingOwnerId(
                "expecting an owner id",
            )))?;
        let contested_index = document_and_contract_info
            .document_type
            .find_contested_index()
            .ok_or(Error::Drive(DriveError::ContestedIndexNotFound(
                "a contested index is expected",
            )))?;
        let document_type = document_and_contract_info.document_type;
        let storage_flags = document_and_contract_info
            .owned_document_info
            .document_info
            .get_storage_flags_ref();

        // we need to construct the path for documents on the contract
        // the path is
        //  * Document and DataContract root tree
        //  * DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_document_type_path =
            vote_contested_resource_active_polls_contract_document_tree_path_vec(
                document_and_contract_info.contract.id_ref().as_bytes(),
                document_and_contract_info.document_type.name(),
            );

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        // at this point the contract path is to the contract documents
        // for each index the top index component will already have been added
        // when the contract itself was created
        let index_path: Vec<Vec<u8>> = contract_document_type_path.clone();

        let mut index_path_info = if document_and_contract_info
            .owned_document_info
            .document_info
            .is_document_size()
        {
            // This is a stateless operation
            PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path))
        } else {
            PathInfo::PathAsVec::<0>(index_path)
        };

        // next we need to store a reference to the document for each index
        for IndexProperty { name, .. } in &contested_index.properties {
            // We on purpose do not want to put index names
            // This is different from document secondary indexes
            // The reason is that there is only one index so we already know the structure

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index on delete",
                    )));
                }

                // On this level we will have all the user defined values for the paths
                estimated_costs_only_with_layer_info.insert(
                    index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        is_sum_tree: false,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            // with the example of the dashpay contract's first index
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    None, //we should never need this in contested documents
                    platform_version,
                )?
                .unwrap_or_default();

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.batch_insert_empty_tree_if_not_exists(
                document_top_field
                    .clone()
                    .add_path_info(index_path_info.clone()),
                false,
                storage_flags,
                apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;

            index_path_info.push(document_top_field)?;
        }

        // Under each tree we have all identifiers of identities that want the contested resource
        // Contrary to normal secondary indexes there are no property names and there is no termination key "0"
        // We get something like
        //                Inter-wizard championship (event type)
        //                             |
        //                       Goblet of Fire (event name) <---- We just inserted this
        //                  /                    \
        //              Sam's ID                Ivan's ID  <---- We now need to insert at this level
        //             /    \                  /      \
        //         0 (ref)   1 (sum tree)    0 (ref)   1 (sum tree)
        //

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have all the identities
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(16), // very seldom would more than 16 people want the resource
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        self.batch_insert_empty_tree_if_not_exists(
            DriveKeyInfo::Key(owner_id.to_vec()).add_path_info(index_path_info.clone()),
            false,
            storage_flags,
            apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;
        
        index_path_info.push(DriveKeyInfo::Key(owner_id.to_vec()))?;

        //                Inter-wizard championship (event type)
        //                             |
        //                       Goblet of Fire (event name) 
        //                  /                    \
        //              Sam's ID                Ivan's ID  <---- We just inserted this 
        //             /    \                  /      \
        //         0 (ref)   1 (sum tree)    0 (ref)   1 (sum tree) <---- We now need to insert at this level
        //
        
        self.add_contested_reference_and_vote_subtree_to_document_operations(document_and_contract_info, index_path_info, storage_flags, estimated_costs_only_with_layer_info, transaction, batch_operations, drive_version)
        
    }
}
