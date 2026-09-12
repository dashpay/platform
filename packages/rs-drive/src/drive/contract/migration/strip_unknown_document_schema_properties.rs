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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::Identity;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::PlatformVersion;

    const OWNER_BALANCE: u64 = 5_000_000;

    /// The protocol 12 schema migration rewrote every stored user contract
    /// whose document schemas carried top-level properties the v1 document
    /// meta-schema forbids. It kept each element's storage flags and applied
    /// the rewrite with a discarded cost vector, so the storage fee share of
    /// the stripped bytes was never refunded to the contract owner and no
    /// pending epoch refund was recorded. That is what every node executed
    /// at the protocol 12 activation, and replay from genesis must reproduce
    /// it byte for byte: the stripped byte counts were never recorded, so a
    /// retroactive settlement is impossible, and the migration never runs
    /// again, so there is nothing at a later version left to correct. The
    /// storage refund invariant that applies from protocol version 15 names
    /// this migration as its recorded historical exception; this test pins
    /// the shipped behaviour so the exception stays exactly what it was.
    #[test]
    fn should_keep_the_protocol_12_schema_strip_frozen_without_refunding_stripped_bytes() {
        let platform_version = PlatformVersion::get(12).expect("protocol version 12");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        // The contract owner exists with a balance, so a refund would be
        // observable as a balance change.
        let mut owner =
            Identity::random_identity(2, Some(12), platform_version).expect("expected an identity");
        owner.set_balance(OWNER_BALANCE);
        drive
            .add_new_identity(
                owner.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert the owner");
        drive
            .add_to_system_credits(OWNER_BALANCE, Some(&transaction), platform_version)
            .expect("expected to record the system credits");

        // A user contract stored the way protocol version 12 stores it: the
        // element carries the owner's storage flags.
        let contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract.json",
            None,
            Some(owner.id()),
            false,
            platform_version,
        )
        .expect("expected the family contract");
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert the contract");

        let contract_id = contract.id();
        let contract_path = contract_root_path(contract_id.as_slice());
        let stored_element = drive
            .grove_get_raw(
                (&contract_path).into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to read the contract element")
            .expect("expected the contract element to exist");
        let (clean_bytes, flags) = match stored_element {
            Element::Item(bytes, flags) => (bytes, flags),
            other => panic!("expected an item, got {:?}", other),
        };
        let owner_flags = StorageFlags::from_element_flags_ref(
            flags
                .as_ref()
                .expect("a user contract carries storage flags"),
        )
        .expect("expected valid flags")
        .expect("expected owner flags");
        assert_eq!(
            owner_flags.owner_id(),
            Some(&owner.id().to_buffer()),
            "the contract bytes are attributed to the owner"
        );

        // Rewrite the stored bytes the way a pre-v12 contract could look:
        // with a top-level document schema property the v1 meta-schema does
        // not allow. The element keeps its flags and grows.
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let (mut serialization_format, _): (DataContractInSerializationFormat, usize) =
            bincode::borrow_decode_from_slice(&clean_bytes, bincode_config)
                .expect("expected to decode the stored contract");
        let person_schema = serialization_format
            .document_schemas_mut()
            .get_mut("person")
            .expect("expected the person document type");
        person_schema
            .insert("legacyUnknownProperty".to_string(), Value::Bool(true))
            .expect("expected to add the unknown property");
        let inflated_bytes = bincode::encode_to_vec(&serialization_format, bincode_config)
            .expect("expected to encode the inflated contract");
        assert!(inflated_bytes.len() > clean_bytes.len());
        drive
            .grove_insert(
                (&contract_path).into(),
                &[0],
                Element::Item(inflated_bytes.clone(), flags.clone()),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to store the inflated contract");
        drive.cache.data_contracts.clear();

        let pending_refunds_before = drive
            .fetch_pending_epoch_refunds(Some(&transaction), &platform_version.drive)
            .expect("expected the pending refunds");
        assert!(pending_refunds_before.is_empty());

        // The migration, exactly as the protocol 12 activation ran it.
        drive
            .strip_unknown_document_schema_properties(&transaction, &platform_version.drive)
            .expect("expected the migration to succeed");

        let migrated_element = drive
            .grove_get_raw(
                (&contract_path).into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to read the migrated element")
            .expect("expected the migrated element to exist");
        let (migrated_bytes, migrated_flags) = match migrated_element {
            Element::Item(bytes, flags) => (bytes, flags),
            other => panic!("expected an item, got {:?}", other),
        };
        assert!(
            migrated_bytes.len() < inflated_bytes.len(),
            "the migration strips the unknown property, so the element shrinks"
        );
        assert_eq!(
            migrated_bytes, clean_bytes,
            "the migration restores the bytes the contract had without the property"
        );
        assert_eq!(
            migrated_flags, flags,
            "the element keeps the owner's storage flags byte for byte"
        );

        // The stripped bytes were owner-paid, yet nothing was refunded: the
        // owner's balance and the pending epoch refunds are untouched and the
        // credit sum stays balanced.
        assert_eq!(
            drive
                .fetch_identity_balance(
                    owner.id().to_buffer(),
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the owner's balance"),
            Some(OWNER_BALANCE)
        );
        assert!(drive
            .fetch_pending_epoch_refunds(Some(&transaction), &platform_version.drive)
            .expect("expected the pending refunds")
            .is_empty());
        let total = drive
            .calculate_total_credits_balance(Some(&transaction), &platform_version.drive)
            .expect("expected the total credits balance");
        assert!(
            total.ok().expect("expected a well-formed balance"),
            "no credits move during the migration: {:?}",
            total
        );
    }
}
