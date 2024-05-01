use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

use crate::drive::Drive;
use crate::error::document::DocumentError;

use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::version::PlatformVersion;

impl Drive {
    /// Prepares the operations for deleting a document.
    #[inline(always)]
    pub(super) fn delete_document_for_contract_id_with_named_type_operations_v0(
        &self,
        document_id: [u8; 32],
        contract_id: [u8; 32],
        document_type_name: &str,
        epoch: &Epoch,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut operations = vec![];
        let Some(contract_fetch_info) = self.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            Some(epoch),
            true,
            transaction,
            &mut operations,
            platform_version,
        )?
        else {
            return Err(Error::Document(DocumentError::DataContractNotFound));
        };

        let contract = &contract_fetch_info.contract;
        let document_type = contract.document_type_for_name(document_type_name)?;
        self.delete_document_for_contract_operations(
            document_id,
            contract,
            document_type,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )
    }
}
