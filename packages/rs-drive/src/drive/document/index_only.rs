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
//! * **delete**: probe every index entry, all must exist. Every index
//!   embeds `$ownerId` (the parser enforces it), so each probe — computed
//!   with owner = signer — proves ownership as well as existence, and
//!   requiring all of them keeps the apply-side batch infallible even
//!   against values spliced from different documents.
//!
//! Path and key encoding reuse `Document::get_raw_for_document_type`,
//! the same function the index walkers key trees with — the probe cannot
//! drift from the write path.

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

    /// The first index of `document_type` that embeds `$ownerId` (as
    /// terminal or prefix property). The indexOnly parser guarantees one
    /// exists — its absence is a corrupted contract.
    pub fn index_only_owner_bearing_index<'a>(
        document_type: &'a DocumentTypeRef,
    ) -> Result<&'a Index, Error> {
        document_type
            .indexes()
            .values()
            .find(|index| {
                index.terminal.as_deref() == Some(dpp::document::property_names::OWNER_ID)
                    || index
                        .properties
                        .iter()
                        .any(|property| property.name == dpp::document::property_names::OWNER_ID)
            })
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "an indexOnly document type must have an $ownerId-bearing index; the \
                 contract parser enforces it",
            )))
    }

    /// The grove path and member key of `document`'s entry under `index`:
    /// `[DataContractDocuments, contract_id, 1, doctype, (<prop>, <value
    /// key>)*, 0]` with the terminal property's value as the key.
    pub fn index_only_entry_path_and_key(
        contract_id: Identifier,
        document_type: DocumentTypeRef,
        index: &Index,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, Vec<u8>), Error> {
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
                    "every indexOnly property must have a value: the parser requires \
                     them all and the transitions carry them all",
                )))
        };

        let mut path: Vec<Vec<u8>> = Vec::with_capacity(5 + index.properties.len() * 2);
        path.push(vec![crate::drive::RootTree::DataContractDocuments as u8]);
        path.push(contract_id.to_vec());
        path.push(vec![1]);
        path.push(document_type.name().as_bytes().to_vec());
        for property in index.properties.iter() {
            path.push(property.name.as_bytes().to_vec());
            path.push(raw_value_for(&property.name)?);
        }
        path.push(vec![0]);

        let terminal =
            index
                .terminal
                .as_deref()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "index_only_entry_path_and_key requires an indexOnly index (terminal is \
                 always Some there after parse normalization)",
                )))?;
        let member_key = raw_value_for(terminal)?;

        Ok((path, member_key))
    }

    /// Whether `document`'s entry under `index` exists AND carries the row
    /// commitment `document`'s full tuple produces. Existence alone cannot
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
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let (path, member_key) = Self::index_only_entry_path_and_key(
            contract_id,
            document_type,
            index,
            document,
            platform_version,
        )?;
        let expected_commitment = crate::drive::document::index_only_row_commitment(
            document,
            document_type,
            platform_version,
        )?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        let element = self.grove_get_raw_optional(
            path_refs.as_slice().into(),
            member_key.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        Ok(match element {
            Some(grovedb::Element::Item(payload, _)) => payload == expected_commitment,
            _ => false,
        })
    }

    /// Whether `document`'s entry under `index` exists (stateful read).
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
        let (path, member_key) = Self::index_only_entry_path_and_key(
            contract_id,
            document_type,
            index,
            document,
            platform_version,
        )?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        self.grove_has_raw(
            path_refs.as_slice().into(),
            member_key.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
