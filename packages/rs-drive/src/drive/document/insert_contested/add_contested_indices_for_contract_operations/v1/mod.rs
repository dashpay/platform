use crate::util::grove_operations::BatchInsertTreeApplyType;

use crate::drive::Drive;
use crate::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfoV0Methods, DriveKeyInfo, PathInfo,
};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::drive::votes::paths::{
    vote_contested_resource_contract_documents_indexes_path_vec,
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
};
use crate::error::drive::DriveError;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::IndexProperty;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::{AllCountTrees, NoSumTrees};
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds contested indices for the contract operations
    /// Will return true if the contest already existed
    ///
    /// Version 1 (protocol version 14) writes the last index value, the tree holding the
    /// poll's contenders with its stored info, abstain and lock entries, as a count tree, so
    /// the number of contenders a join is checked against is one element read. A poll whose
    /// last index value already exists keeps the tree it has: a poll started before protocol
    /// version 14 goes on in its plain tree.
    #[inline(always)]
    pub(super) fn add_contested_indices_for_contract_operations_v1(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
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
            vote_contested_resource_contract_documents_indexes_path_vec(
                document_and_contract_info.contract.id_ref().as_bytes(),
                document_and_contract_info.document_type.name(),
            );

        // Every index value tree sits in a plain tree and is plain, except the last one, a
        // count tree; the poll's choices sit in that count tree and are plain
        let estimating = estimated_costs_only_with_layer_info.is_some();
        let flags_len = storage_flags
            .map(|s| s.serialized_size())
            .unwrap_or_default();
        let apply_type = |in_tree_type: TreeType, tree_type: TreeType| {
            if estimating {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type,
                    tree_type,
                    flags_len,
                }
            } else {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            }
        };
        let choice_apply_type = apply_type(TreeType::CountTree, TreeType::NormalTree);

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

        let mut contest_already_existed = true;

        let last_property_position = contested_index.properties.len().saturating_sub(1);

        // next we need to store a reference to the document for each index
        for (position, IndexProperty { name, .. }) in contested_index.properties.iter().enumerate()
        {
            let value_tree_type = if position == last_property_position {
                TreeType::CountTree
            } else {
                TreeType::NormalTree
            };

            // We on purpose do not want to put index names
            // This is different from document secondary indexes
            // The reason is that there is only one index so we already know the structure

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type, platform_version)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index on delete",
                    )));
                }

                // On this level we will have all the user defined values for the paths
                estimated_costs_only_with_layer_info.insert(
                    index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        tree_type: TreeType::NormalTree,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            if value_tree_type == TreeType::CountTree {
                                AllCountTrees
                            } else {
                                NoSumTrees
                            },
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
            let inserted = self.batch_insert_empty_tree_if_not_exists(
                document_top_field
                    .clone()
                    .add_path_info(index_path_info.clone()),
                value_tree_type,
                storage_flags,
                apply_type(TreeType::NormalTree, value_tree_type),
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;

            // if we insert anything, that means that the contest didn't already exist
            if contest_already_existed {
                contest_already_existed &= !inserted;
            }

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
                    tree_type: TreeType::CountTree,
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
            TreeType::NormalTree,
            storage_flags,
            choice_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;

        let inserted_abstain = self.batch_insert_empty_tree_if_not_exists(
            DriveKeyInfo::Key(RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec())
                .add_path_info(index_path_info.clone()),
            TreeType::NormalTree,
            storage_flags,
            choice_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;

        let inserted_lock = self.batch_insert_empty_tree_if_not_exists(
            DriveKeyInfo::Key(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec())
                .add_path_info(index_path_info.clone()),
            TreeType::NormalTree,
            storage_flags,
            choice_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;

        let mut towards_identity_index_path_info = index_path_info.clone();
        towards_identity_index_path_info.push(DriveKeyInfo::Key(owner_id.to_vec()))?;

        //                Inter-wizard championship (event type)
        //                             |
        //                       Goblet of Fire (event name)
        //                  /                    \
        //              Sam's ID                Ivan's ID  <---- We just inserted this
        //             /    \                  /      \
        //         0 (ref)   1 (sum tree)    0 (ref)   1 (sum tree) <---- We now need to insert at this level
        //

        self.add_contested_reference_and_vote_subtree_to_document_operations(
            document_and_contract_info,
            towards_identity_index_path_info,
            storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_version,
        )?;

        if inserted_abstain {
            let mut towards_abstain_index_path_info = index_path_info.clone();
            towards_abstain_index_path_info.push(DriveKeyInfo::Key(
                RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec(),
            ))?;

            self.add_contested_vote_subtree_for_non_identities_operations(
                towards_abstain_index_path_info,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                drive_version,
            )?;
        }

        if inserted_lock {
            let mut towards_lock_index_path_info = index_path_info;
            towards_lock_index_path_info.push(DriveKeyInfo::Key(
                RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec(),
            ))?;

            self.add_contested_vote_subtree_for_non_identities_operations(
                towards_lock_index_path_info,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                drive_version,
            )?;
        }

        Ok(contest_already_existed)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::votes::paths::VotePollPaths;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::add_dpns_name_contenders;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    /// A drive holding the DPNS contract
    fn drive_with_dpns() -> (Drive, DataContract) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let dpns_contract = get_dpns_data_contract_fixture(
            Some(Identifier::from([7; 32])),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        drive
            .apply_contract(
                &dpns_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the DPNS contract");
        (drive, dpns_contract)
    }

    /// The elements of the poll's index values, the first one to the last one
    fn index_value_elements(
        drive: &Drive,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
    ) -> Vec<Element> {
        let platform_version = PlatformVersion::latest();
        let mut path = vote_poll
            .contenders_path(platform_version)
            .expect("expected the choices path");
        let mut elements = vec![];
        for _ in &vote_poll.index_values {
            let key = path.pop().expect("expected an index value");
            elements.insert(
                0,
                drive
                    .grove_get_raw(
                        path.as_slice().into(),
                        &key,
                        DirectQueryType::StatefulDirectQuery,
                        None,
                        &mut vec![],
                        &platform_version.drive,
                    )
                    .expect("expected to read the index value")
                    .expect("expected the index value"),
            );
        }
        elements
    }

    /// The last index value counts the contenders plus the poll's stored info, abstain and
    /// lock entries; the values before it stay plain trees
    #[test]
    fn should_write_the_last_index_value_of_a_new_poll_as_a_count_tree() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = drive_with_dpns();
        let vote_poll = add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..4,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );

        let [parent, label]: [Element; 2] = index_value_elements(&drive, &vote_poll)
            .try_into()
            .expect("expected two index values");
        assert!(matches!(parent, Element::Tree(..)), "{parent:?}");
        assert!(matches!(label, Element::CountTree(_, 7, _)), "{label:?}");
    }

    /// A poll started before protocol version 14 goes on in its plain tree
    #[test]
    fn should_keep_the_plain_tree_of_a_poll_started_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = drive_with_dpns();
        add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..2,
            |_| 1,
            &BlockInfo::default(),
            PlatformVersion::get(13).expect("expected version 13"),
        );
        let vote_poll = add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            2..4,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );

        let [parent, label]: [Element; 2] = index_value_elements(&drive, &vote_poll)
            .try_into()
            .expect("expected two index values");
        assert!(matches!(parent, Element::Tree(..)), "{parent:?}");
        assert!(matches!(label, Element::Tree(..)), "{label:?}");
    }
}
