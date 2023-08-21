use crate::drive::batch::DriveOperation;
use crate::drive::grove_operations::BatchInsertApplyType::StatefulBatchInsert;
use crate::drive::grove_operations::BatchInsertTreeApplyType::StatefulBatchInsertTree;
use crate::drive::identity::contract_info::insert::DataContractApplyInfo::{
    ContractBased, ContractFamilyBased,
};
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::identity::{
    identity_contract_info_group_path_vec, identity_contract_info_root_path_vec,
    identity_key_location_within_identity_vec, identity_path_vec,
};
use crate::drive::object_size_info::{PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use costs::storage_cost::removal::Identifier;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::{BTreeMap, HashMap};

pub enum DataContractApplyInfo<'a> {
    /// The root_id is either a contract id or an owner id
    /// It is a contract id for in the case of contract bound keys or contract
    /// document bound keys
    /// In the case
    ContractBased {
        contract_id: Identifier,
        document_type_keys: BTreeMap<&'a str, Vec<KeyID>>,
        contract_keys: Vec<KeyID>,
    },
    ContractFamilyBased {
        contracts_owner_id: Identifier,
        family_keys: Vec<KeyID>,
    },
}

impl<'a> DataContractApplyInfo<'a> {
    fn root_id(&self) -> [u8; 32] {
        match self {
            ContractBased { contract_id, .. } => *contract_id,
            ContractFamilyBased {
                contracts_owner_id, ..
            } => *contracts_owner_id,
        }
    }
    fn keys(self) -> (BTreeMap<&'a str, Vec<KeyID>>, Vec<KeyID>) {
        match self {
            ContractBased {
                document_type_keys,
                contract_keys,
                ..
            } => (document_type_keys, contract_keys),
            ContractFamilyBased { family_keys, .. } => (BTreeMap::new(), family_keys),
        }
    }
    fn new_from_single_key(
        key_id: KeyID,
        contract_bounds: &ContractBounds,
        drive: &Drive,
        epoch: &Epoch,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let contract_id = contract_bounds.identifier().to_buffer();
        // we are getting with fetch info to add the cost to the drive operations
        let maybe_contract_fetch_info = drive.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            Some(epoch),
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;
        let Some(contract_fetch_info) = maybe_contract_fetch_info else {
                    return Err(Error::Identity(IdentityError::IdentityKeyBoundsError("Contract for key bounds not found")));
                };
        let contract = &contract_fetch_info.contract;
        match contract_bounds {
            ContractBounds::SingleContract { .. } => Ok(ContractBased {
                contract_id: contract.id.buffer,
                document_type_keys: Default::default(),
                contract_keys: vec![key_id],
            }),
            ContractBounds::SingleContractDocumentType { document_type, .. } => {
                let document_type = contract
                    .document_type_for_name(document_type)
                    .map_err(Error::Protocol)?;
                Ok(ContractBased {
                    contract_id: contract.id().buffer,
                    document_type_keys: BTreeMap::from([(&document_type.name, vec![key_id])]),
                    contract_keys: vec![],
                })
            }
            ContractBounds::MultipleContractsOfSameOwner { .. } => Ok(ContractFamilyBased {
                contracts_owner_id: contract.owner_id.buffer,
                family_keys: vec![key_id],
            }),
        }
    }
}

impl Drive {
    pub(crate) fn add_potential_contract_info_for_contract_bounded_key(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(contract_bounds) = &identity_key.contract_bounds {
            // We need to get the contract
            let contract_apply_info = DataContractApplyInfo::new_from_single_key(
                identity_key.id,
                contract_bounds,
                self,
                epoch,
                transaction,
                drive_operations,
                platform_version,
            )?;
            self.add_contract_info_operations(
                identity_id,
                vec![contract_apply_info],
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            )
        }
        Ok(())
    }

    /// Adds the contract info operations
    pub(crate) fn add_contract_info_operations(
        &self,
        identity_id: [u8; 32],
        contract_infos: Vec<DataContractApplyInfo>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let identity_path = identity_path_vec(identity_id.as_slice());
        // we insert the contract root tree if it doesn't exist already
        self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
            PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])),
            None,
            StatefulBatchInsertTree,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        for contract_info in contract_infos.into_iter() {
            let root_id = contract_info.root_id();
            self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                PathKeyInfo::<0>::PathKey((
                    identity_contract_info_root_path_vec(identity_id.as_slice()),
                    root_id.to_vec(),
                )),
                None,
                StatefulBatchInsertTree,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?;
            let (document_keys, contract_or_family_keys) = contract_info.keys();
            for key_id in contract_or_family_keys {
                // we need to add a reference to the key
                let key_id_bytes = key_id.encode_var_vec();
                let key_reference =
                    identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                self.batch_insert_if_not_exists(
                    PathKeyElementInfo::<0>::PathKeyRefElement((
                        identity_contract_info_group_path_vec(
                            identity_id.as_slice(),
                            root_id.as_slice(),
                        ),
                        key_id_bytes.as_slice(),
                        Element::Reference(
                            UpstreamRootHeightReference(1, key_reference),
                            Some(1),
                            None,
                        ),
                    )),
                    StatefulBatchInsert,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;
            }

            for (document_type, document_key_ids) in document_keys {
                self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                    PathKeyInfo::<0>::PathKey((
                        identity_contract_info_root_path_vec(identity_id.as_slice()),
                        root_id.to_vec(),
                    )),
                    None,
                    StatefulBatchInsertTree,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )?;
                for key_id in document_key_ids {
                    // we need to add a reference to the key
                    let key_id_bytes = key_id.encode_var_vec();
                    let key_reference =
                        identity_key_location_within_identity_vec(key_id_bytes.as_slice());

                    self.batch_insert_if_not_exists(
                        PathKeyElementInfo::<0>::PathKeyRefElement((
                            identity_contract_info_group_path_vec(
                                identity_id.as_slice(),
                                root_id.as_slice(),
                            ),
                            key_id_bytes.as_slice(),
                            Element::Reference(
                                UpstreamRootHeightReference(1, key_reference),
                                Some(1),
                                None,
                            ),
                        )),
                        StatefulBatchInsert,
                        transaction,
                        drive_operations,
                        &platform_version.drive,
                    )?;
                }
            }
        }

        Ok(())
    }
}
