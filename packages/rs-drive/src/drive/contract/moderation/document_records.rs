//! The storage of the records a moderated contract keeps by document type, then document id:
//! the removal records of the documents its moderators deleted (`[64, id, 2, 16]`) and the
//! approvals its seated team gave the deletion of settled documents (`[64, id, 2, 24]`). Both
//! are created, written, read and proved by the code here, told apart by
//! [`ContractDocumentRecords`], or by the record type ([`ContractDocumentRecord`]); the
//! versioned methods of each call it.

use crate::drive::contract::moderation::types::{
    decode_document_record_element, ContractDocumentRecord, ContractDocumentRecords,
    ContractDocumentRemovalsQuery,
};
use crate::drive::contract::paths::{
    contract_document_records_key, contract_document_records_path,
    contract_document_type_records_path, contract_other_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The trees of one kind of records: the tree of all of them when `with_root` is set, and
    /// under it one tree per document type of `document_type_names`. Unconditional, like the
    /// list trees: an insertion (re)creates the contract's root subtree in the same batch, and
    /// an update only names document types the stored contract does not have, whose trees can
    /// not exist.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_document_record_trees_operations_v0(
        &self,
        contract_id: [u8; 32],
        records: ContractDocumentRecords,
        with_root: bool,
        document_type_names: &[&str],
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_document_record_trees(
                contract_id,
                records,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        if with_root {
            self.batch_insert_empty_tree(
                contract_other_path(&contract_id),
                DriveKeyInfo::KeyRef(contract_document_records_key(records)),
                storage_flags,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        for document_type_name in document_type_names {
            self.batch_insert_empty_tree(
                contract_document_records_path(&contract_id, records),
                DriveKeyInfo::KeyRef(document_type_name.as_bytes()),
                storage_flags,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }

    /// The write of one record under its document's type, keyed by the document's id, flagged
    /// with `payer_id`: the moderator that writes it pays for it. Nothing ever deletes it. It
    /// is replaced in place when `replaces_existing`; two operations on one key would fail the
    /// batch. A replacement may change size, and its flags follow GroveDB's flag merge: a
    /// longer or a shorter record passes to the moderator that replaced it, who pays for the
    /// bytes a longer one adds, while the bytes a shorter one frees are refunded to the
    /// moderator the record named before; an equally long one keeps the earlier moderator's
    /// flags.
    ///
    /// An estimate prices a replacement as a fresh insert of the whole record: GroveDB's
    /// average-case replace assumes an item keeps its size and would price no storage for what
    /// a replacement adds, which the moderator's balance is then not checked against.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_document_record_operations_v0<T: ContractDocumentRecord>(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        record: &T,
        replaces_existing: bool,
        payer_id: Identifier,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let estimating = estimated_costs_only_with_layer_info.is_some();
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_document_record(
                contract_id.to_buffer(),
                T::RECORDS,
                document_type_name,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(payer_id.to_buffer()));

        let path_key_element = PathFixedSizeKeyRefElement((
            contract_document_type_records_path(
                contract_id.as_slice(),
                T::RECORDS,
                document_type_name,
            ),
            document_id.as_slice(),
            Element::new_item_with_flags(record.encode(), storage_flags.to_some_element_flags()),
        ));

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if replaces_existing && !estimating {
            self.batch_replace(
                path_key_element,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        } else {
            self.batch_insert(
                path_key_element,
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(batch_operations)
    }

    /// The records of one document type the query selects, in document id order. A contract or
    /// a document type that keeps none reads as none.
    pub(super) fn fetch_contract_document_records_v0<T: ContractDocumentRecord>(
        &self,
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<T::Entry>, Error> {
        Self::check_contract_document_records_query(query, T::RECORDS, platform_version)?;

        // A path query over a missing tree is an error in GroveDB, so check first. The check
        // itself reads under the contract's tree of these records, which a contract that keeps
        // none (or a contract id nobody has) does not have either: that reads as no record,
        // like the absence of the document type's own tree.
        let exists = match self.grove_has_raw(
            (&contract_document_records_path(contract_id.as_slice(), T::RECORDS)).into(),
            query.document_type_name.as_bytes(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(exists) => exists,
            Err(Error::GroveDB(error))
                if matches!(
                    *error,
                    grovedb::Error::PathParentLayerNotFound(_) | grovedb::Error::PathNotFound(_)
                ) =>
            {
                false
            }
            Err(error) => return Err(error),
        };
        if !exists {
            return Ok(vec![]);
        }

        let path_query =
            Self::contract_document_records_query(contract_id.to_buffer(), T::RECORDS, query);
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                decode_document_record_element::<T>(&key, &element).map_err(|description| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "contract {} {} {} is malformed: {}",
                        contract_id,
                        query.document_type_name,
                        T::RECORDS.record_name(),
                        description
                    )))
                })
            })
            .collect()
    }

    /// One record, read the way the transform of a moderation reads state: the operations of
    /// the read are added to `drive_operations` for billing. Also how Drive reads the owner of
    /// a removed document a derived index property reads through a `moderatedDocument`
    /// reference.
    pub(crate) fn fetch_contract_document_record_add_to_operations_v0<T: ContractDocumentRecord>(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<T>, Error> {
        let path = contract_document_type_records_path(
            contract_id.as_slice(),
            T::RECORDS,
            document_type_name,
        );
        self.grove_get_raw_optional_item(
            (&path).into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .map(|value| {
            T::decode(&value).map_err(|description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract {} {} {} of document {} is malformed: {}",
                    contract_id,
                    document_type_name,
                    T::RECORDS.record_name(),
                    document_id,
                    description
                )))
            })
        })
        .transpose()
    }

    /// The proof of the records of one document type the query selects.
    pub(super) fn prove_contract_document_records_v0(
        &self,
        contract_id: Identifier,
        records: ContractDocumentRecords,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        Self::check_contract_document_records_query(query, records, platform_version)?;
        let path_query =
            Self::contract_document_records_query(contract_id.to_buffer(), records, query);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
