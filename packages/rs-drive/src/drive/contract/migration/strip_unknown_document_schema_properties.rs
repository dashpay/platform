use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::QueryResultType;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::document_type::schema::allowed_top_level_properties::strip_unknown_properties_from_document_schema;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, PathQuery, Query, SizedQuery, Transaction};
use grovedb_path::SubtreePath;

impl Drive {
    /// Iterates every data contract in state, checks each document type schema for
    /// top-level properties that pre-v12 contracts were not permitted to declare
    /// (i.e. anything outside `ALLOWED_DOCUMENT_SCHEMA_PRE_V12_PROPERTIES`), removes
    /// them, and re-serializes the contract if anything changed. This includes
    /// v12-introduced flags such as `documentsCountable` / `rangeCountable`, which
    /// the v2 parser would otherwise revive on already-stored contracts and
    /// reinterpret as a count tree mismatched with the underlying `NormalTree`.
    ///
    /// For historical contracts, all stored revisions are cleaned (not just the latest).
    ///
    /// Also clears the data contract cache so that subsequent fetches reload
    /// the cleaned contracts from disk.
    pub fn strip_unknown_document_schema_properties(
        &self,
        transaction: &Transaction,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // 1. Fetch all contract IDs.
        let contract_ids =
            self.fetch_contract_ids_v0(None, u16::MAX, Some(transaction), drive_version)?;

        tracing::debug!(
            contract_count = contract_ids.len(),
            "Checking contracts for unknown document schema properties"
        );

        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        // 2. For each contract, read the raw element, check, and possibly update.
        for contract_id_bytes in &contract_ids {
            let contract_path = contract_root_path(contract_id_bytes.as_slice());

            // Try reading the element at key [0] under the contract root.
            let maybe_element = self.grove_get_raw(
                (&contract_path).into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                Some(transaction),
                &mut vec![],
                drive_version,
            )?;

            match maybe_element {
                Some(Element::Item(bytes, flags)) => {
                    // Non-historical contract: stored directly at [root, id, [0]]
                    let path_vec: Vec<Vec<u8>> = contract_path.iter().map(|s| s.to_vec()).collect();
                    self.strip_and_rewrite_contract_element(
                        &bytes,
                        flags,
                        &path_vec,
                        &[0],
                        contract_id_bytes,
                        bincode_config,
                        transaction,
                        drive_version,
                    )?;
                }
                Some(Element::Tree(..)) => {
                    // Historical contract: iterate ALL revisions in the history
                    // subtree at [root, id, 0]. Each revision is an Item keyed
                    // by an encoded timestamp. Key [0] is a Reference to the
                    // latest — we skip references and only process Items.
                    let history_path =
                        contract_keeping_history_root_path(contract_id_bytes.as_slice());
                    let history_path_vec: Vec<Vec<u8>> =
                        history_path.iter().map(|s| s.to_vec()).collect();

                    let mut query = Query::new();
                    query.insert_all();
                    let path_query = PathQuery::new(
                        history_path_vec.clone(),
                        SizedQuery::new(query, None, None),
                    );

                    let (result_items, _) = self.grove_get_raw_path_query(
                        &path_query,
                        Some(transaction),
                        QueryResultType::QueryKeyElementPairResultType,
                        &mut vec![],
                        drive_version,
                    )?;

                    for (key, element) in result_items.to_key_elements() {
                        match element {
                            Element::Item(bytes, flags) => {
                                self.strip_and_rewrite_contract_element(
                                    &bytes,
                                    flags,
                                    &history_path_vec,
                                    &key,
                                    contract_id_bytes,
                                    bincode_config,
                                    transaction,
                                    drive_version,
                                )?;
                            }
                            Element::Reference(..) => {
                                // The [0] reference to the latest revision — skip it.
                            }
                            _ => {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(
                                    format!(
                                        "Unexpected element type in historical contract {} at key {}",
                                        hex::encode(contract_id_bytes),
                                        hex::encode(&key)
                                    ),
                                )));
                            }
                        }
                    }
                }
                _ => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "No element or unexpected type at contract root for {}",
                        hex::encode(contract_id_bytes)
                    ))));
                }
            }
        }

        // Clear the global data contract cache so that subsequent fetches
        // reload the cleaned contracts from disk rather than serving stale
        // cached versions with the unknown properties still present.
        self.cache.data_contracts.clear();

        Ok(())
    }

    /// Deserializes a contract element, strips unknown top-level properties
    /// from its document schemas, and writes back if anything changed.
    #[allow(clippy::too_many_arguments)]
    fn strip_and_rewrite_contract_element(
        &self,
        stored_bytes: &[u8],
        element_flags: Option<grovedb::ElementFlags>,
        storage_path_vec: &[Vec<u8>],
        storage_key: &[u8],
        contract_id_bytes: &[u8; 32],
        bincode_config: impl bincode::config::Config,
        transaction: &Transaction,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let mut serialization_format: DataContractInSerializationFormat =
            match bincode::borrow_decode_from_slice(stored_bytes, bincode_config) {
                Ok((format, _len)) => format,
                Err(e) => {
                    return Err(Error::Drive(DriveError::CorruptedSerialization(format!(
                        "Failed to deserialize contract {} during migration: {}",
                        hex::encode(contract_id_bytes),
                        e
                    ))));
                }
            };

        let mut contract_modified = false;
        for (doc_type_name, schema_value) in serialization_format.document_schemas_mut().iter_mut()
        {
            if strip_unknown_properties_from_document_schema(schema_value) {
                tracing::info!(
                    contract_id = hex::encode(contract_id_bytes),
                    document_type = %doc_type_name,
                    "Stripped unknown top-level properties from document schema"
                );
                contract_modified = true;
            }
        }

        if !contract_modified {
            return Ok(());
        }

        let new_bytes =
            bincode::encode_to_vec(&serialization_format, bincode_config).map_err(|e| {
                Error::Drive(DriveError::CorruptedSerialization(format!(
                    "Failed to re-serialize contract {}: {}",
                    hex::encode(contract_id_bytes),
                    e
                )))
            })?;

        let new_element = Element::Item(new_bytes, element_flags);

        let path_slices: Vec<&[u8]> = storage_path_vec.iter().map(|v| v.as_slice()).collect();
        self.grove_insert(
            SubtreePath::from(path_slices.as_slice()),
            storage_key,
            new_element,
            Some(transaction),
            None,
            &mut vec![],
            drive_version,
        )?;

        tracing::info!(
            contract_id = hex::encode(contract_id_bytes),
            "Updated contract after stripping unknown document schema properties"
        );

        Ok(())
    }
}
