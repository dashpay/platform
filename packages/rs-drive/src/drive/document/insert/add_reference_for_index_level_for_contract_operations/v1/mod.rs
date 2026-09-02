use super::v0::TerminalReferenceFlagsSource;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::document_type::IndexLevelTypeInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the terminal reference.
    ///
    /// v1: the terminal reference element takes the storage flags the walker
    /// passed down, not the document info's own — v0 read the latter, which
    /// diverges exactly when a walker level decides its elements carry no
    /// flags (immutable doctypes historically, TTL'd (ephemeral) sub-levels
    /// now). Ephemeral references must be flagless or their removal turns
    /// sectioned (refundable), breaking the TTL no-refunds invariant.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_reference_for_index_level_for_contract_operations_v1(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.add_reference_for_index_level_for_contract_operations_inner(
            document_and_contract_info,
            index_path_info,
            index_type,
            any_fields_null,
            all_fields_null,
            previous_batch_operations,
            storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            platform_version,
            TerminalReferenceFlagsSource::Walker,
        )
    }
}
