//! indexOnly entry probes.
//!
//! An indexOnly document's entries are `[…values, 0, <terminal value>] →
//! Item` — one per index, all derived from the same value tuple and owner
//! and written/removed atomically. That atomicity gives validation two
//! cheap, sufficient probes:
//!
//! * **create**: any existing entry is a duplicate, so probe every index —
//!   a shorter index thereby doubles as a uniqueness constraint over its
//!   value projection plus owner (for Yappr's likes the `[postId]` index
//!   is the one-like-per-(post, owner) rule).
//! * **delete**: probe every surviving index entry. Only paths already
//!   drained from expired buckets are exempt. Every index
//!   embeds `$ownerId` (the parser enforces it), so each probe — computed
//!   with owner = signer — proves ownership as well as existence, and
//!   requiring all of them keeps the apply-side batch infallible even
//!   against values spliced from different documents.
//!
//! Path and key encoding reuse `Document::get_raw_for_document_type`,
//! the same function the index walkers key trees with — the probe cannot
//! drift from the write path.

use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::time_range_ttl::entry_key_bucket_start;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// The entry paths a document's values produce under one index — one per
/// containing bucket for a time-range index, exactly one otherwise —
/// paired with the shared member key (the terminal property's value).
pub type IndexOnlyEntryPathsAndKey = (Vec<Vec<Vec<u8>>>, Vec<u8>);

impl Drive {
    /// Reconstruct the document an indexOnly delete's entries were written
    /// from: the carried values plus the owner, with `$createdAt` moved
    /// from its system key in `data` back to the document's own field
    /// (where `get_raw_for_document_type` reads it).
    pub fn index_only_document_from_values(
        document_id: Identifier,
        owner_id: Identifier,
        mut data: std::collections::BTreeMap<String, dpp::platform_value::Value>,
    ) -> Result<Document, Error> {
        let created_at = data
            .remove(dpp::document::property_names::CREATED_AT)
            .map(|value| value.to_integer::<u64>())
            .transpose()
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "indexOnly values carried a non-integer $createdAt",
                ))
            })?;
        Ok(dpp::document::DocumentV0 {
            id: document_id,
            owner_id,
            properties: data,
            created_at,
            ..Default::default()
        }
        .into())
    }

    /// The grove paths and member key of `document`'s entries under
    /// `index`: each path is `[DataContractDocuments, contract_id, 1,
    /// doctype, (<level key>, <value key>)*, 0]` with the terminal
    /// property's value as the member key.
    ///
    /// A plain index produces exactly one path. A time-range (bucketed)
    /// index produces one path per bucket containing the document's
    /// timestamp: the first level's segment is the grid-qualified
    /// [`level_key`](Index::level_key) and its value keys come from
    /// [`TimeRangeTransform::entry_keys_for_raw`] — the same derivation
    /// the index walkers write with, so probe and write paths cannot
    /// drift (including the edge rules: a pre-origin timestamp produces
    /// NO entries, so it produces no probe paths either). A `skipIfAbsent`
    /// index whose trigger (first property) the document omits likewise
    /// produces NO paths (and an empty member key) — the write walkers
    /// skipped the branch, so there is nothing to probe; the zero-path
    /// case flows through both consumers with the correct semantics
    /// (vacuously consistent for the delete probe, no duplicate for the
    /// create probe).
    ///
    /// [`TimeRangeTransform::entry_keys_for_raw`]:
    /// dpp::data_contract::document_type::TimeRangeTransform::entry_keys_for_raw
    pub fn index_only_entry_paths_and_key(
        contract_id: Identifier,
        document_type: DocumentTypeRef,
        index: &Index,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<IndexOnlyEntryPathsAndKey, Error> {
        let owner_id = Some(document.owner_id().to_buffer());

        let raw_value_for = |property_name: &str| -> Result<Vec<u8>, Error> {
            document
                .get_raw_for_document_type(
                    property_name,
                    document_type,
                    owner_id,
                    platform_version,
                )?
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "every required indexOnly property must have a value: the parser \
                     requires them and the transitions carry them",
                )))
        };

        // A skipIfAbsent index participates only when the document carries
        // its trigger — the first property, which the parser guarantees is
        // the only one that may be absent. Mirror the write walkers' skip
        // exactly: no trigger, no entries.
        if index.skip_if_absent {
            let trigger = &index
                .properties
                .first()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a skipIfAbsent index has at least one property; the contract parser \
                     enforces it",
                )))?
                .name;
            if document
                .get_raw_for_document_type(trigger, document_type, owner_id, platform_version)?
                .is_none()
            {
                return Ok((Vec::new(), Vec::new()));
            }
        }

        let prefix: Vec<Vec<u8>> = vec![
            vec![crate::drive::RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1],
            document_type.name().as_bytes().to_vec(),
        ];
        let mut paths: Vec<Vec<Vec<u8>>> = vec![prefix];
        for (position, property) in index.properties.iter().enumerate() {
            let level_key = index.level_key(position, &property.name);
            let raw = raw_value_for(&property.name)?;
            // Only a time-range index's first property fans out; every
            // other level extends each path with its single value key.
            let value_keys: Vec<Vec<u8>> = match index.time_range.as_ref() {
                Some(transform) if position == 0 => transform.entry_keys_for_raw(&raw),
                _ => vec![raw],
            };
            paths = paths
                .into_iter()
                .flat_map(|base| {
                    value_keys
                        .iter()
                        .map(|value_key| {
                            let mut path = base.clone();
                            path.push(level_key.as_bytes().to_vec());
                            path.push(value_key.clone());
                            path
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        for path in paths.iter_mut() {
            path.push(vec![0]);
        }

        let terminal =
            index
                .terminal
                .as_deref()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "index_only_entry_paths_and_key requires an indexOnly index (terminal is \
                 always Some there after parse normalization)",
                )))?;
        let member_key = raw_value_for(terminal)?;

        Ok((paths, member_key))
    }

    /// Whether every surviving entry under `index` carries
    /// `expected_commitment` — the row commitment `document`'s full tuple
    /// produces (compute it ONCE per document with
    /// [`index_only_row_commitment`](crate::drive::document::index_only_row_commitment)
    /// and share it across every index probe). Existence alone cannot
    /// distinguish one document's projections from several coexisting
    /// documents' projections — the stored commitment is what binds an
    /// entry to its row, so a values tuple spliced from different creates
    /// fails this check on whichever entry belongs to the other row.
    #[allow(clippy::too_many_arguments)]
    pub fn index_only_entry_commitment_matches(
        &self,
        contract_id: Identifier,
        document_type: DocumentTypeRef,
        index: &Index,
        document: &Document,
        expected_commitment: &[u8; 32],
        block_time_ms: u64,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let (paths, member_key) = Self::index_only_entry_paths_and_key(
            contract_id,
            document_type,
            index,
            document,
            platform_version,
        )?;
        // Every surviving entry must carry the commitment. An expired
        // bucket can have lost any intermediate tree to lazy drainage;
        // only that case is exempt. A missing live path, a missing member
        // in a standing tree, or a different commitment still fails.
        // If all paths have expired and drained, this index contributes
        // no mutations to the delete and is vacuously consistent.
        for path in paths {
            let bucket_depth = CONTRACT_DOCUMENTS_PATH_HEIGHT as usize + 1;
            if index.time_range.as_ref().is_some_and(|transform| {
                path.get(bucket_depth)
                    .and_then(|key| entry_key_bucket_start(key))
                    .is_some_and(|start| transform.bucket_expired(start, block_time_ms))
            }) && !self.expired_entry_path_exists(
                &path,
                bucket_depth,
                transaction,
                platform_version,
            )? {
                continue;
            }
            let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
            let element = self.grove_get_raw_optional(
                path_refs.as_slice().into(),
                member_key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?;
            let matches = match element {
                Some(grovedb::Element::Item(payload, _)) => {
                    payload == expected_commitment.as_slice()
                }
                // Summable indexes store `ItemWithSumItem(commitment,
                // amount)`; the commitment payload plays the same binding
                // role, and the amount needs no separate check — it is one
                // of the document's properties, so it is already covered
                // by the commitment.
                Some(grovedb::Element::ItemWithSumItem(payload, _, _)) => {
                    payload == expected_commitment.as_slice()
                }
                _ => false,
            };
            if !matches {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether any of `document`'s entries under `index` exists (stateful
    /// read). For a bucketed index the entries are written atomically, so
    /// ANY existing bucket entry means the projection exists — the
    /// duplicate-detection contract the create-side probes rely on.
    #[allow(clippy::too_many_arguments)]
    pub fn has_index_only_document_entry(
        &self,
        contract_id: Identifier,
        document_type: DocumentTypeRef,
        index: &Index,
        document: &Document,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let (paths, member_key) = Self::index_only_entry_paths_and_key(
            contract_id,
            document_type,
            index,
            document,
            platform_version,
        )?;
        for path in paths {
            let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
            if self.grove_has_raw(
                path_refs.as_slice().into(),
                member_key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
