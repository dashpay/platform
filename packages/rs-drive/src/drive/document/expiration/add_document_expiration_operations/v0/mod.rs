use crate::drive::document::expiration::paths::{
    documents_expirations_at_time_path_vec, documents_expirations_path_vec, encode_expiration_time,
};
use crate::drive::document::expiration::DocumentExpirationEntry;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyElementInfo::{PathKeyElement, PathKeyElementSize};
use crate::util::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_document_expiration_operations_v0(
        &self,
        document_id: Option<[u8; 32]>,
        entry: &DocumentExpirationEntry,
        expires_at_ms: TimestampMillis,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let entry_bytes = entry.to_bytes();

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_document_expiration(
                expires_at_ms,
                entry_bytes.len() as u32,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        //                    Misc / E
        //              /                   \
        //       expires at t1             expires at t2
        //        /         \                    |
        //   document 1   document 2        document 3

        // The tree of the documents expiring at this time, unless a document created earlier
        // (in this block or the batch) already made it.
        let time_key = DriveKeyInfo::Key(encode_expiration_time(expires_at_ms).to_vec());
        let path_key_info =
            time_key.add_path_info::<0>(PathInfo::PathAsVec(documents_expirations_path_vec()));
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };
        self.batch_insert_empty_tree_if_not_exists(
            path_key_info,
            TreeType::NormalTree,
            None,
            apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            &platform_version.drive,
        )?;

        // The entry itself, keyed by the document id: a document id is unique, so the key is
        // free and a plain insert suffices.
        let time_path = documents_expirations_at_time_path_vec(expires_at_ms);
        let item = Element::Item(entry_bytes, None);
        let path_key_element_info: PathKeyElementInfo<'_, 0> = match document_id {
            Some(document_id) if estimated_costs_only_with_layer_info.is_none() => {
                PathKeyElement((time_path, document_id.to_vec(), item))
            }
            Some(document_id) => PathKeyElementSize((
                KeyInfoPath::from_known_owned_path(time_path),
                KeyInfo::KnownKey(document_id.to_vec()),
                item,
            )),
            None => PathKeyElementSize((
                KeyInfoPath::from_known_owned_path(time_path),
                KeyInfo::MaxKeySize {
                    unique_id: b"document_expiration_entry".to_vec(),
                    max_size: 32,
                },
                item,
            )),
        };
        self.batch_insert(
            path_key_element_info,
            batch_operations,
            &platform_version.drive,
        )
    }
}
