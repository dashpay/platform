use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::drive::{Drive, RootTree};
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
    /// top-level properties not listed in the v1 document meta-schema, removes them,
    /// and re-serializes the contract if anything changed.
    ///
    /// Also clears the data contract cache so that subsequent fetches reload
    /// the cleaned contracts from disk.
    pub fn strip_unknown_document_schema_properties(
        &self,
        transaction: &Transaction,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // 1. Get all contract IDs stored under the DataContractDocuments root tree.
        let contracts_root_path =
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()];

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(contracts_root_path, SizedQuery::new(query, None, None));

        let (result_items, _) = self.grove_get_raw_path_query(
            &path_query,
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            drive_version,
        )?;

        let contract_ids: Vec<Vec<u8>> = result_items.to_keys();

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

            // Collect the stored bytes, element flags, the GroveDB path (as
            // Vec<Vec<u8>>) and the key where the element lives so we can
            // write it back if modified.
            let (stored_bytes, element_flags, storage_path_vec, storage_key): (
                Vec<u8>,
                _,
                Vec<Vec<u8>>,
                Vec<u8>,
            ) = match maybe_element {
                Some(Element::Item(bytes, flags)) => {
                    // Non-historical contract: stored directly at [root, id, [0]]
                    let path_vec: Vec<Vec<u8>> = contract_path.iter().map(|s| s.to_vec()).collect();
                    (bytes, flags, path_vec, vec![0u8])
                }
                Some(Element::Tree(..)) => {
                    // Historical contract: the latest version is stored
                    // behind a reference at [root, id, [0], [0]].  We must
                    // resolve it to find the actual Item element.
                    let history_path =
                        contract_keeping_history_root_path(contract_id_bytes.as_slice());
                    let maybe_history_element = self.grove_get_raw(
                        (&history_path).into(),
                        &[0],
                        DirectQueryType::StatefulDirectQuery,
                        Some(transaction),
                        &mut vec![],
                        drive_version,
                    )?;
                    let history_path_vec: Vec<Vec<u8>> =
                        history_path.iter().map(|s| s.to_vec()).collect();
                    match maybe_history_element {
                        Some(Element::Reference(ref_path, ..)) => {
                            // The reference points to a sibling key (the
                            // encoded timestamp).  Resolve it.
                            let timestamp_key = match &ref_path {
                                grovedb::reference_path::ReferencePathType::SiblingReference(
                                    key,
                                ) => key.clone(),
                                _ => {
                                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                                        format!(
                                            "Unexpected reference type in historical contract {}",
                                            hex::encode(contract_id_bytes)
                                        ),
                                    )));
                                }
                            };
                            let maybe_actual = self.grove_get_raw(
                                (&history_path).into(),
                                timestamp_key.as_slice(),
                                DirectQueryType::StatefulDirectQuery,
                                Some(transaction),
                                &mut vec![],
                                drive_version,
                            )?;
                            match maybe_actual {
                                Some(Element::Item(bytes, flags)) => {
                                    (bytes, flags, history_path_vec, timestamp_key)
                                }
                                _ => {
                                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                                        format!(
                                            "Could not resolve historical contract element for {}",
                                            hex::encode(contract_id_bytes)
                                        ),
                                    )));
                                }
                            }
                        }
                        Some(Element::Item(bytes, flags)) => {
                            // Direct item (no reference indirection)
                            (bytes, flags, history_path_vec, vec![0u8])
                        }
                        _ => {
                            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                "Unexpected element type in historical contract path for {}",
                                hex::encode(contract_id_bytes)
                            ))));
                        }
                    }
                }
                _ => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "No element or unexpected type at contract root for {}",
                        hex::encode(contract_id_bytes)
                    ))));
                }
            };

            // 3. Deserialize to DataContractInSerializationFormat
            let mut serialization_format: DataContractInSerializationFormat =
                match bincode::borrow_decode_from_slice(stored_bytes.as_slice(), bincode_config) {
                    Ok((format, _len)) => format,
                    Err(e) => {
                        return Err(Error::Drive(DriveError::CorruptedSerialization(format!(
                            "Failed to deserialize contract {} during migration: {}",
                            hex::encode(contract_id_bytes),
                            e
                        ))));
                    }
                };

            // 4. Check and strip unknown keys from each document schema
            let mut contract_modified = false;
            for (doc_type_name, schema_value) in
                serialization_format.document_schemas_mut().iter_mut()
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
                continue;
            }

            // 5. Re-serialize and write back
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
                storage_key.as_slice(),
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
        }

        // Clear the global data contract cache so that subsequent fetches
        // reload the cleaned contracts from disk rather than serving stale
        // cached versions with the unknown properties still present.
        self.cache.data_contracts.clear();

        Ok(())
    }
}
