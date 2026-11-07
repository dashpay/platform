use crate::drive::document::private_document_store::append_private_document_entries_operations;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// Operations on the store of a private document type (protocol version 15)
#[derive(Clone, Debug)]
pub enum PrivateDocumentOperationType {
    /// Append entries to the store of a private document type, in order, each after Drive's
    /// header: the block's height and time.
    AppendEntries {
        /// The contract that defines the private document type
        contract_id: [u8; 32],
        /// The private document type
        document_type_name: String,
        /// The entries, each exactly the type's `entrySize`
        entries: Vec<Vec<u8>>,
    },
}

impl DriveLowLevelOperationConverter for PrivateDocumentOperationType {
    fn into_low_level_drive_operations(
        self,
        _drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        block_info: &BlockInfo,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            PrivateDocumentOperationType::AppendEntries {
                contract_id,
                document_type_name,
                entries,
            } => append_private_document_entries_operations(
                &contract_id,
                &document_type_name,
                &entries,
                block_info,
                platform_version,
            ),
        }
    }
}
