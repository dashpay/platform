use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::BatchInsertTreeApplyType::StatefulBatchInsertTree;
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::identity::{identity_contract_info_root_path_vec, identity_path_vec};
use crate::drive::object_size_info::PathKeyInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::identity::IdentityPublicKey;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
/// Data contract apply info
#[allow(dead_code)]
pub enum DataContractApplyInfo {
    /// Vector of Identity public keys
    Keys(Vec<IdentityPublicKey>),
}

impl Drive {
    /// Adds contract info operations to Drive.
    ///
    /// This function processes and adds the provided contract information as a series of
    /// low-level Drive operations.
    ///
    /// # Parameters
    ///
    /// * `identity_id`: A 32-byte array representing the identity ID.
    /// * `contract_infos`: A vector of tuples containing contract IDs and their corresponding application info.
    /// * `epoch`: A reference to the current epoch.
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to an optional `HashMap` containing estimated layer information.
    /// * `transaction`: The transaction arguments.
    /// * `platform_version`: A reference to the current platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of low-level Drive operations (`LowLevelDriveOperation`)
    /// or an error (`Error`) if any issue occurs during processing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let drive = Drive::new(...); // Initialize Drive
    /// let identity_id = [0u8; 32]; // Replace with actual identity ID
    /// let contract_infos = vec![...]; // Populate with actual contract info
    /// let epoch = Epoch::new(...); // Create or obtain the current epoch
    /// let mut estimated_info = Some(HashMap::new()); // Estimated layer information map
    /// let transaction = TransactionArg::new(...); // Specify the transaction arguments
    /// let platform_version = PlatformVersion::new(...); // Current Platform version
    ///
    /// let result = drive.add_contract_info_operations(
    ///     identity_id,
    ///     contract_infos,
    ///     &epoch,
    ///     &mut estimated_info,
    ///     transaction,
    ///     &platform_version,
    /// );
    /// ```
    #[allow(dead_code)]
    pub(crate) fn add_contract_info_operations(
        &self,
        identity_id: [u8; 32],
        contract_infos: Vec<([u8; 32], DataContractApplyInfo)>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        let storage_flags = StorageFlags::SingleEpoch(epoch.index);
        let identity_path = identity_path_vec(identity_id.as_slice());
        // we insert the contract root tree if it doesn't exist already
        self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])),
            Some(&storage_flags),
            StatefulBatchInsertTree,
            transaction,
            &mut None,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        let identity_contract_root_path =
            identity_contract_info_root_path_vec(identity_id.as_slice());

        for (contract_id, contract_info) in contract_infos.into_iter() {
            self.batch_insert_empty_tree_if_not_exists(
                PathKeyInfo::<0>::PathKey((
                    identity_contract_root_path.clone(),
                    contract_id.to_vec(),
                )),
                Some(&storage_flags),
                StatefulBatchInsertTree,
                transaction,
                &mut None,
                &mut batch_operations,
                &platform_version.drive,
            )?;
            match contract_info {
                DataContractApplyInfo::Keys(keys) => {
                    self.add_new_keys_to_identity_operations(
                        identity_id,
                        keys,
                        vec![],
                        false,
                        estimated_costs_only_with_layer_info,
                        transaction,
                        platform_version,
                    )?;
                }
            }
        }
        Ok(batch_operations)
    }
}
