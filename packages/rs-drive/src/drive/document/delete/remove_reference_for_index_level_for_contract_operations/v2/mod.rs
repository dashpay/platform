use crate::drive::document::document_reference_size;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use dpp::data_contract::document_type::IndexLevelTypeInfo;
use dpp::version::PlatformVersion;
use grovedb::EstimatedLayerSizes::AllReference;
use grovedb::TransactionArg;

impl Drive {
    /// Removes the terminal reference.
    ///
    /// v2 (protocol version 15) is v1, describing the `[0]` layer of a non-unique, contested or
    /// null-field index for an estimate as the references it holds, keyed by document id, as the
    /// insert walker does (`add_reference_for_index_level_for_contract_operations`). v1 describes
    /// it as subtrees, so a batch that inserts a document under an index value and deletes
    /// another under the same one (a create consuming a document of its own type) described
    /// the layer two ways, and its estimate kept the cheaper subtree description.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_reference_for_index_level_for_contract_operations_v2(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        skip_missing_expired_entry: bool,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.remove_reference_for_index_level_describing_entries_as(
            document_and_contract_info,
            index_path_info,
            index_type,
            any_fields_null,
            all_fields_null,
            storage_flags,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            skip_missing_expired_entry,
            event_id,
            transaction,
            batch_operations,
            |document_type, flags_size| {
                AllReference(
                    DEFAULT_HASH_SIZE_U8,
                    document_reference_size(document_type),
                    flags_size,
                )
            },
            platform_version,
        )
    }
}
