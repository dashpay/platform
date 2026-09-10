use std::collections::HashMap;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::query_result_type::QueryResultType;
use grovedb::{
    Element, EstimatedLayerInformation, MaybeTree, PathQuery, Query, SizedQuery, TransactionArg,
    TreeType,
};

use crate::drive::document::lifecycle::{DocumentLifecycleRecord, DOCUMENT_LIFECYCLE_RECORD_SIZE};
use crate::drive::document::paths::{document_history_path, document_lifecycle_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};
use crate::util::grove_operations::{DirectQueryType, QueryTarget};
use crate::util::object_size_info::PathKeyElementInfo::{PathKeyElement, PathKeyElementSize};
use crate::util::storage_flags::StorageFlags;

/// Length of a revision key: a block timestamp followed by a history sequence.
const REVISION_KEY_LENGTH: usize = 16;

impl Drive {
    /// Prepares the operations for removing a bounded chunk of the retained
    /// revisions of an already deleted document.
    ///
    /// Revisions go newest first, so the remnant a partial erasure leaves behind
    /// is the document's oldest content and the retained sequence stays
    /// contiguous from one. The chunk is bounded by
    /// `max_document_revisions_erased_per_transition`, and the enumeration asks
    /// for one revision more than it may remove so that it knows, before
    /// emitting anything, whether this chunk is the last one.
    ///
    /// A terminal chunk also removes the lifecycle record and the now empty
    /// history subtree; GroveDB establishes that the subtree is empty from the
    /// deletes already in this batch and refuses the removal otherwise. A
    /// non-terminal first chunk instead overwrites the record with the erasure
    /// it authorizes, which is what lets any identity finish the work later.
    /// The two never happen together, so one batch never carries two operations
    /// on the record's key.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn erase_document_for_contract_operations_v0(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if !document_type.documents_keep_history() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "only a document type that keeps history has revisions to erase",
            )));
        }
        let chunk = platform_version
            .system_limits
            .max_document_revisions_erased_per_transition
            .ok_or(Error::Drive(DriveError::NotSupported(
                "erasing retained revisions is not available at this protocol version",
            )))?;

        let history_path = document_history_path(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
            document_id.as_slice(),
        );
        let lifecycle_path =
            document_lifecycle_path(contract.id_ref().as_bytes(), document_type.name().as_str());
        // Worst-case sizing: a chunk may hold the type's largest documents, and
        // the admission estimate has to cover that whatever this document
        // actually stores.
        let revision_size = document_type.max_size(platform_version)? as u32;

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        if let Some(layers) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_erase_document(
                document_id,
                contract,
                document_type,
                layers,
                platform_version,
            )?;
            return self.estimated_erase_document_operations(
                document_id,
                &history_path,
                &lifecycle_path,
                chunk,
                revision_size,
                block_info,
                batch_operations,
                transaction,
                platform_version,
            );
        }

        let removable = self.enumerate_newest_revision_keys(
            &history_path,
            chunk,
            &mut batch_operations,
            transaction,
            platform_version,
        )?;
        if removable.is_empty() {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "a document with a lifecycle record retains no revisions".to_string(),
            )));
        }
        let terminal = removable.len() <= chunk as usize;
        let to_remove = if terminal {
            removable.as_slice()
        } else {
            &removable[..chunk as usize]
        };

        // A leaf delete does not consult the batch it joins, so each one is
        // generated against an empty view and appended: building the whole
        // running batch once per revision would be quadratic in the chunk size.
        let mut single_operation = Vec::with_capacity(2);
        for key in to_remove {
            single_operation.clear();
            self.batch_delete(
                history_path.as_slice().into(),
                key,
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                &mut single_operation,
                &platform_version.drive,
            )?;
            batch_operations.append(&mut single_operation);
        }

        if terminal {
            self.batch_delete(
                lifecycle_path.as_slice().into(),
                document_id.as_slice(),
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                &mut batch_operations,
                &platform_version.drive,
            )?;
            let mut history_root = history_path;
            history_root.pop();
            // Every revision has already been deleted into this batch, which is
            // how GroveDB establishes that the subtree is empty. If any survived
            // it refuses the removal rather than orphaning their storage.
            self.batch_delete(
                history_root.as_slice().into(),
                document_id.as_slice(),
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(
                        TreeType::ProvableCountTree,
                    )),
                },
                transaction,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        } else {
            let (record, flags) = self.fetch_lifecycle_record_with_flags(
                document_id,
                &lifecycle_path,
                &mut batch_operations,
                transaction,
                platform_version,
            )?;
            if record.is_erasing() {
                // A continuation: the record already carries the erasure this
                // chunk is finishing, and nothing but an authorized first chunk
                // may write those fields.
                return Ok(batch_operations);
            }
            let (time_ms, revision) = Self::decode_revision_key(&removable[0])?;
            // Equal in size to the record it replaces, and carrying the same
            // flags, so the deleter stays the beneficiary of its bytes.
            self.batch_insert::<0>(
                PathKeyElement((
                    lifecycle_path,
                    document_id.to_vec(),
                    Element::Item(
                        record
                            .starting_erase_at(block_info.time_ms, time_ms, revision)
                            .serialize(),
                        flags,
                    ),
                )),
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(batch_operations)
    }

    /// Reads the revision keys of one document newest first, one more than a
    /// chunk may remove so the caller can tell a terminal chunk from a partial
    /// one before it emits anything.
    ///
    /// The keys are what this needs, but GroveDB's query surface has no
    /// key-only result shape over a range: every result type it exposes carries
    /// elements. The revision bodies therefore come back and are dropped, and
    /// the estimate below prices that read for what it is. What this does avoid
    /// is the running-batch snapshot the general delete-by-query helper rebuilds
    /// for every element, which is quadratic in the chunk size.
    fn enumerate_newest_revision_keys(
        &self,
        history_path: &[Vec<u8>],
        chunk: u16,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let mut query = Query::new_with_direction(false);
        // Exactly the revision key space: the current pointer lives in the
        // primary-key tree and is not in this subtree at all.
        query.insert_range_from(encode_u64(0)..);
        let path_query = PathQuery::new(
            history_path.to_vec(),
            SizedQuery::new(query, Some(chunk.saturating_add(1)), None),
        );
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            batch_operations,
            &platform_version.drive,
        )?;
        Ok(results
            .to_key_elements()
            .into_iter()
            .map(|(key, _)| key)
            .collect())
    }

    /// Reads one lifecycle record together with the flags naming the identity
    /// its bytes were charged to.
    fn fetch_lifecycle_record_with_flags(
        &self,
        document_id: Identifier,
        lifecycle_path: &[Vec<u8>],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(DocumentLifecycleRecord, Option<Vec<u8>>), Error> {
        let element = self
            .grove_get_raw(
                lifecycle_path.into(),
                document_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "a document being erased has no lifecycle record".to_string(),
            )))?;
        let Element::Item(bytes, flags) = element else {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "a lifecycle record is not an item",
            )));
        };
        Ok((DocumentLifecycleRecord::deserialize(&bytes)?, flags))
    }

    /// Splits a revision key into the block time and history sequence it
    /// carries.
    fn decode_revision_key(key: &[u8]) -> Result<(u64, u64), Error> {
        if key.len() != REVISION_KEY_LENGTH {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "a retained revision key does not carry a time and a sequence".to_string(),
            )));
        }
        let decode = |bytes: &[u8]| -> u64 {
            let mut buffer = [0u8; 8];
            buffer.copy_from_slice(bytes);
            // The encoder flips the sign bit so that the ordering of the keys
            // matches the ordering of the values.
            u64::from_be_bytes(buffer) ^ (1 << 63)
        };
        Ok((decode(&key[..8]), decode(&key[8..])))
    }

    /// Prices a full chunk without reading any state.
    ///
    /// The dry run cannot know how many revisions a document retains, so it
    /// charges for a whole chunk of the type's largest documents plus both
    /// endings: the record write of a first chunk and the subtree removal of a
    /// terminal one. Every erase is therefore admitted against the same
    /// worst-case estimate whatever the document's actual history length.
    ///
    /// The enumeration is priced at one revision more than a chunk removes,
    /// because that is what it reads, and at the full body size, because the
    /// query surface returns bodies whether or not the caller wants them. The
    /// record read a non-terminal chunk performs is priced too.
    #[allow(clippy::too_many_arguments)]
    fn estimated_erase_document_operations(
        &self,
        document_id: Identifier,
        history_path: &[Vec<u8>],
        lifecycle_path: &[Vec<u8>],
        chunk: u16,
        revision_size: u32,
        block_info: &BlockInfo,
        mut batch_operations: Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // The enumeration reads one revision more than the chunk removes, and
        // reads each of them whole.
        for sequence in 1..=chunk as u64 + 1 {
            let mut key = encode_u64(block_info.time_ms);
            key.extend(encode_u64(sequence));
            self.grove_get_raw(
                history_path.into(),
                &key,
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: TreeType::ProvableCountTree,
                    query_target: QueryTarget::QueryTargetValue(revision_size),
                },
                transaction,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        // A non-terminal chunk reads the record before overwriting it.
        self.grove_get_raw(
            lifecycle_path.into(),
            document_id.as_slice(),
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTarget::QueryTargetValue(DOCUMENT_LIFECYCLE_RECORD_SIZE),
            },
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        let mut single_operation = Vec::with_capacity(2);
        for sequence in 1..=chunk as u64 {
            // Distinct synthetic keys of the real shape: the dry run's batch is
            // checked for consistency like any other, so a repeated key would be
            // rejected rather than priced.
            let mut key = encode_u64(block_info.time_ms);
            key.extend(encode_u64(sequence));
            single_operation.clear();
            self.batch_delete(
                history_path.into(),
                &key,
                StatelessBatchDelete {
                    in_tree_type: TreeType::ProvableCountTree,
                    estimated_key_size: REVISION_KEY_LENGTH as u32,
                    estimated_value_size: revision_size,
                },
                transaction,
                &mut single_operation,
                &platform_version.drive,
            )?;
            batch_operations.append(&mut single_operation);
        }

        // A record of the right shape rather than the real one: the dry run
        // cannot know the deletion time it carries or the identity it is
        // flagged to, only that both are there and how many bytes they take.
        let flags_len = StorageFlags::approximate_size(true, None) as usize;
        self.batch_insert::<0>(
            PathKeyElementSize((
                KeyInfoPath::from_known_owned_path(lifecycle_path.to_vec()),
                KeyInfo::KnownKey(document_id.to_vec()),
                Element::Item(
                    vec![0u8; DOCUMENT_LIFECYCLE_RECORD_SIZE as usize],
                    Some(vec![0u8; flags_len]),
                ),
            )),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        let mut history_root = history_path.to_vec();
        history_root.pop();
        self.batch_delete(
            history_root.as_slice().into(),
            document_id.as_slice(),
            StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: 32,
                estimated_value_size: TreeType::ProvableCountTree.inner_node_type().cost() + 3,
            },
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
