use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::util::estimated_layers::EstimatedLayers;

use grovedb::{MaybeTree, TransactionArg};

use dpp::data_contract::document_type::DocumentTypeRef;

use crate::util::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};

use crate::drive::constants::DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE;
use crate::drive::document::uses_current_document_history_layout;
use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::identifier::Identifier;

use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::version::PlatformVersion;

impl Drive {
    /// Removes one entry from a document type's primary-key tree.
    ///
    /// A keep-history type's entry is the document's current pointer, a
    /// reference rather than an item, so the stateless size the dry run
    /// charges comes from the reference shape. Removing it is still removing a
    /// leaf: the revisions the pointer names live in the type's history tree,
    /// which this never touches.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_document_from_primary_storage_v1(
        &self,
        document_id: Identifier,
        document_type: DocumentTypeRef,
        contract_documents_primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let keeps_history = document_type.documents_keep_history();
        if keeps_history && !uses_current_document_history_layout(platform_version)? {
            // The legacy primary entry is the subtree holding its revisions.
            // Only the composite layout stores a leaf pointer that can be
            // removed without unlinking retained revisions.
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a keep-history primary entry can only be removed where the current pointer \
                 layout is in use",
            )));
        }

        let primary_key_tree_type = document_type.primary_key_tree_type(platform_version)?;

        let apply_type = if estimated_costs_only_with_layer_info.is_some() {
            StatelessBatchDelete {
                in_tree_type: primary_key_tree_type,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: if keeps_history {
                    DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE
                } else {
                    document_type.estimated_size(platform_version)? as u32
                },
            }
        } else {
            // The entry is a document item, or a current pointer to one; never
            // a tree.
            StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        };
        self.batch_delete(
            (&contract_documents_primary_key_path).into(),
            document_id.as_slice(),
            apply_type,
            transaction,
            batch_operations,
            &platform_version.drive,
        )?;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_remove_document_to_primary_storage(
                contract_documents_primary_key_path,
                document_type,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        Ok(())
    }
}
