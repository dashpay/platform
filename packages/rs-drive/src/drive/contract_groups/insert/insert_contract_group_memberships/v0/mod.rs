use crate::drive::contract_groups::paths::{
    contract_group_contracts_path_vec, contract_group_document_types_for_contract_path_vec,
    contract_group_document_types_path, contract_group_tokens_for_contract_path_vec,
    contract_group_tokens_path, contract_groups_members_path,
    contract_memberships_document_type_path_vec, contract_memberships_document_types_path,
    contract_memberships_groups_path_vec, contract_memberships_path,
    contract_memberships_token_path_vec, contract_memberships_tokens_path,
    CONTRACT_GROUPS_GROUPS_KEY, CONTRACT_GROUP_BACKWARDS_REFERENCE_ROOT_HEIGHT,
    CONTRACT_GROUP_CONTRACTS_KEY, CONTRACT_GROUP_DOCUMENT_TYPES_KEY, CONTRACT_GROUP_TOKENS_KEY,
    CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY, CONTRACT_MEMBERSHIPS_GROUPS_KEY,
    CONTRACT_MEMBERSHIPS_TOKENS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::{ContractGroupMember, ContractGroupMembership};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::{HashMap, HashSet};

/// A backwards reference from a member contract's index to the forward entry `forward_path`
/// (given without the root tree key) at `forward_key`. One hop resolves it.
fn backwards_reference(mut forward_path: Vec<Vec<u8>>, forward_key: &[u8]) -> Element {
    forward_path.push(forward_key.to_vec());
    Element::Reference(
        ReferencePathType::UpstreamRootHeightReference(
            CONTRACT_GROUP_BACKWARDS_REFERENCE_ROOT_HEIGHT,
            forward_path,
        ),
        Some(1),
        None,
    )
}

impl Drive {
    pub(super) fn insert_contract_group_memberships_v0(
        &self,
        contract_id: Identifier,
        memberships: &[ContractGroupMembership],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.insert_contract_group_memberships_operations_v0(
            contract_id,
            memberships,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// Inserts an empty tree at `path/key` once per call: a second membership needing the same
    /// tree reuses the first insertion, so one batch never carries two operations on one slot.
    fn insert_contract_group_tree_once(
        &self,
        created_trees: &mut HashSet<Vec<Vec<u8>>>,
        path: Vec<Vec<u8>>,
        key: &[u8],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut tree_path = path.clone();
        tree_path.push(key.to_vec());
        if !created_trees.insert(tree_path) {
            return Ok(());
        }
        self.batch_insert_empty_tree(
            path.iter().map(Vec::as_slice),
            DriveKeyInfo::KeyRef(key),
            None,
            batch_operations,
            &platform_version.drive,
        )
    }

    pub(super) fn insert_contract_group_memberships_operations_v0(
        &self,
        contract_id: Identifier,
        memberships: &[ContractGroupMembership],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if memberships.is_empty() {
            return Ok(vec![]);
        }

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_insert_contract_group_memberships(
                contract_id.to_buffer(),
                memberships,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        // Every tree keyed by the new contract id is created here, once.
        let mut created_trees: HashSet<Vec<Vec<u8>>> = HashSet::new();
        let contract_id_bytes = contract_id.to_buffer();

        // The contract's backwards index root: [ContractGroups, Members, <contract id>].
        self.insert_contract_group_tree_once(
            &mut created_trees,
            contract_groups_members_path()
                .iter()
                .map(|segment| segment.to_vec())
                .collect(),
            &contract_id_bytes,
            &mut batch_operations,
            platform_version,
        )?;

        for membership in memberships {
            let contract_group_id = membership.contract_group_id.to_buffer();
            match &membership.member {
                ContractGroupMember::Contract => {
                    // Forward: [ContractGroups, Groups, <group>, Contracts] / <contract id>
                    self.batch_insert(
                        PathKeyElement::<0>((
                            contract_group_contracts_path_vec(&contract_group_id),
                            contract_id_bytes.to_vec(),
                            Element::new_item(vec![]),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    // Backwards: [ContractGroups, Members, <contract id>, Groups] / <group>
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_memberships_path(&contract_id_bytes)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        CONTRACT_MEMBERSHIPS_GROUPS_KEY,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.batch_insert(
                        PathKeyElement::<0>((
                            contract_memberships_groups_path_vec(&contract_id_bytes),
                            contract_group_id.to_vec(),
                            backwards_reference(
                                vec![
                                    CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
                                    contract_group_id.to_vec(),
                                    CONTRACT_GROUP_CONTRACTS_KEY.to_vec(),
                                ],
                                &contract_id_bytes,
                            ),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;
                }
                ContractGroupMember::DocumentType(document_type_name) => {
                    let name_bytes = document_type_name.as_bytes();

                    // Forward: [.., <group>, DocumentTypes, <contract id>] / <name>
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_group_document_types_path(&contract_group_id)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        &contract_id_bytes,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.batch_insert(
                        PathKeyElement::<0>((
                            contract_group_document_types_for_contract_path_vec(
                                &contract_group_id,
                                &contract_id_bytes,
                            ),
                            name_bytes.to_vec(),
                            Element::new_item(vec![]),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    // Backwards: [.., Members, <contract id>, DocumentTypes, <name>] / <group>
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_memberships_path(&contract_id_bytes)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_memberships_document_types_path(&contract_id_bytes)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        name_bytes,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.batch_insert(
                        PathKeyElement::<0>((
                            contract_memberships_document_type_path_vec(
                                &contract_id_bytes,
                                name_bytes,
                            ),
                            contract_group_id.to_vec(),
                            backwards_reference(
                                vec![
                                    CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
                                    contract_group_id.to_vec(),
                                    CONTRACT_GROUP_DOCUMENT_TYPES_KEY.to_vec(),
                                    contract_id_bytes.to_vec(),
                                ],
                                name_bytes,
                            ),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;
                }
                ContractGroupMember::Token(token_position) => {
                    let position_bytes = token_position.to_be_bytes();

                    // Forward: [.., <group>, Tokens, <contract id>] / <position>
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_group_tokens_path(&contract_group_id)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        &contract_id_bytes,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.batch_insert(
                        PathKeyElement::<0>((
                            contract_group_tokens_for_contract_path_vec(
                                &contract_group_id,
                                &contract_id_bytes,
                            ),
                            position_bytes.to_vec(),
                            Element::new_item(vec![]),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    // Backwards: [.., Members, <contract id>, Tokens, <position>] / <group>
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_memberships_path(&contract_id_bytes)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        CONTRACT_MEMBERSHIPS_TOKENS_KEY,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.insert_contract_group_tree_once(
                        &mut created_trees,
                        contract_memberships_tokens_path(&contract_id_bytes)
                            .iter()
                            .map(|segment| segment.to_vec())
                            .collect(),
                        &position_bytes,
                        &mut batch_operations,
                        platform_version,
                    )?;
                    self.batch_insert(
                        PathKeyElement::<0>((
                            contract_memberships_token_path_vec(
                                &contract_id_bytes,
                                &position_bytes,
                            ),
                            contract_group_id.to_vec(),
                            backwards_reference(
                                vec![
                                    CONTRACT_GROUPS_GROUPS_KEY.to_vec(),
                                    contract_group_id.to_vec(),
                                    CONTRACT_GROUP_TOKENS_KEY.to_vec(),
                                    contract_id_bytes.to_vec(),
                                ],
                                &position_bytes,
                            ),
                        )),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;
                }
            }
        }

        Ok(batch_operations)
    }
}
