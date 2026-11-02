//! Drive Queries
//!
//! Defines and implements in Drive functions relevant to querying.
//!

mod fetch_document_history_query;
mod query_chained_documents;
mod query_composite_documents;
/// query of the vote state
pub mod query_contested_documents_vote_state;
mod query_documents;
mod query_documents_with_flags;

/// query of the contested documents in their storage
pub mod query_contested_documents_storage;

pub use query_chained_documents::*;
pub use query_composite_documents::*;
pub use query_documents::*;
pub use query_documents_with_flags::*;

#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::block::epoch::Epoch;
#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(feature = "fixtures-and-mocks")]
use grovedb::TransactionArg;

use crate::drive::Drive;

#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use crate::error::query::QuerySyntaxError;
#[cfg(feature = "fixtures-and-mocks")]
use crate::error::Error;
#[cfg(feature = "fixtures-and-mocks")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "fixtures-and-mocks")]
use crate::query::DriveDocumentQuery;
#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use crate::verify::RootHash;
#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use dpp::data_contract::DataContract;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::version::PlatformVersion;
#[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
use dpp::version::PlatformVersionCurrentVersion;

// TODO: Not using
// /// The outcome of a query
// pub struct QueryDocumentIdsOutcome {
//     /// returned items
//     pub items: Vec<Key>,
//     /// skipped item count
//     pub skipped: u16,
//     /// the processing cost
//     pub cost: u64,
// }

/// Query serialized documents outcome
#[cfg(feature = "fixtures-and-mocks")]
/// Outcome of a serialized documents query
pub struct QuerySerializedDocumentsOutcome {
    /// returned items
    pub items: Vec<Vec<u8>>,
    /// skipped item count
    pub skipped: u16,
    /// the processing cost
    pub cost: u64,
}

impl Drive {
    // TODO: Not using
    // /// Performs and returns the result as ids of the specified query
    // /// along with skipped items and the cost.
    // pub fn query_document_ids(
    //     &self,
    //     query: DriveQuery,
    //     epoch: Option<&Epoch>,
    //     transaction: TransactionArg,
    //     protocol_version: Option<u32>,
    // ) -> Result<QueryDocumentIdsOutcome, Error> {
    //     let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
    //     let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
    //     let (items, skipped) = query.execute_no_proof_internal(
    //         self,
    //         QueryResultType::QueryKeyElementPairResultType,
    //         transaction,
    //         &mut drive_operations,
    //         platform_version,
    //     )?;
    //     let cost = if let Some(epoch) = epoch {
    //         let fee_result =
    //             Drive::calculate_fee(None, Some(drive_operations), epoch, platform_version)?;
    //         fee_result.processing_fee
    //     } else {
    //         0
    //     };
    //
    //     let keys = items
    //         .to_key_elements()
    //         .into_iter()
    //         .map(|(key, _element)| key)
    //         .collect();
    //
    //     Ok(QueryDocumentIdsOutcome {
    //         items: keys,
    //         skipped,
    //         cost,
    //     })
    // }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs and returns the result of the specified query along with skipped items and the cost.
    pub fn query_raw_documents_from_contract_cbor_using_cbor_encoded_query_with_cost(
        &self,
        query_cbor: &[u8],
        contract: &DataContract,
        document_type_name: String,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        //todo cbor cost
        let document_type = contract.document_type_for_name(document_type_name.as_str())?;

        let (items, skipped) = self.query_documents_for_cbor_query_internal(
            contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
            protocol_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, skipped, cost))
    }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs and returns the result of the specified query along with the fee.
    /// Proof is generated.
    #[allow(clippy::too_many_arguments)]
    pub fn query_proof_of_documents_using_contract_id_using_cbor_encoded_query_with_cost(
        &self,
        query_cbor: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        block_info: Option<BlockInfo>,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<(Vec<u8>, u64), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let contract = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                epoch,
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found",
            )))?;
        let document_type = contract
            .contract
            .document_type_for_name(document_type_name)?;
        let items = self.query_proof_of_documents_using_cbor_encoded_query(
            &contract.contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
            protocol_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs and returns the result of the specified query along with the fee.
    /// Proof is generated.
    pub fn query_proof_of_documents_using_cbor_encoded_query_with_cost(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<(Vec<u8>, u64), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let items = self.query_proof_of_documents_using_cbor_encoded_query(
            contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
            protocol_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs and returns the result of the specified internal query.
    /// Proof is generated.
    pub(crate) fn query_proof_of_documents_using_cbor_encoded_query(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        query_cbor: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        protocol_version: Option<u32>,
    ) -> Result<Vec<u8>, Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let query = DriveDocumentQuery::from_cbor(
            query_cbor,
            contract,
            document_type,
            &self.config,
            platform_version,
        )?;

        query.execute_with_proof_internal(self, transaction, drive_operations, platform_version)
    }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs the specified internal query and returns the root hash, values, and fee.
    pub fn query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<(RootHash, Vec<Vec<u8>>, u64), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let (root_hash, items) = self
            .query_proof_of_documents_using_cbor_encoded_query_only_get_elements_internal(
                contract,
                document_type,
                query_cbor,
                transaction,
                &mut drive_operations,
                protocol_version,
            )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((root_hash, items, cost))
    }

    /// Performs the specified internal query and returns the root hash and values.
    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    pub(crate) fn query_proof_of_documents_using_cbor_encoded_query_only_get_elements_internal(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        query_cbor: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        protocol_version: Option<u32>,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let query = DriveDocumentQuery::from_cbor(
            query_cbor,
            contract,
            document_type,
            &self.config,
            platform_version,
        )?;

        query.execute_with_proof_only_get_elements_internal(
            self,
            transaction,
            drive_operations,
            platform_version,
        )
    }

    /// Performs and returns the result of the specified query along with skipped items and the cost.
    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    pub fn query_documents_cbor_from_contract(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let (items, skipped) = self.query_documents_for_cbor_query_internal(
            contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
            protocol_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, skipped, cost))
    }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs and returns the result of the specified query along with skipped items and the cost.
    pub fn query_documents_cbor_with_document_type_lookup(
        &self,
        query_cbor: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<QuerySerializedDocumentsOutcome, Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let contract = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                epoch,
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found",
            )))?;
        let document_type = contract
            .contract
            .document_type_for_name(document_type_name)?;

        let query = DriveDocumentQuery::from_cbor(
            query_cbor,
            &contract.contract,
            document_type,
            &self.config,
            platform_version,
        )?;

        self.query_serialized_documents(query, epoch, transaction, platform_version)
    }

    #[cfg(all(feature = "fixtures-and-mocks", feature = "cbor_query"))]
    /// Performs and returns the result of the specified query along with skipped items.
    pub fn query_documents_for_cbor_query_internal(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        query_cbor: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        protocol_version: Option<u32>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
        let query = DriveDocumentQuery::from_cbor(
            query_cbor,
            contract,
            document_type,
            &self.config,
            platform_version,
        )?;

        query.execute_raw_results_no_proof_internal(
            self,
            transaction,
            drive_operations,
            platform_version,
        )
    }

    #[cfg(feature = "fixtures-and-mocks")]
    /// Performs and returns the result of the specified query along with skipped items
    /// and the cost.
    pub fn query_serialized_documents(
        &self,
        query: DriveDocumentQuery,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QuerySerializedDocumentsOutcome, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let (items, skipped) = query.execute_raw_results_no_proof_internal(
            self,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(epoch) = epoch {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };

        Ok(QuerySerializedDocumentsOutcome {
            items,
            skipped,
            cost,
        })
    }

    // TODO: Not using
    // Performs and returns the result of the specified query along with skipped items and the cost.
    // pub fn query_documents_from_contract(
    //     &self,
    //     contract: &DataContract,
    //     document_type: DocumentTypeRef,
    //     query_cbor: &[u8],
    //     block_info: Option<BlockInfo>,
    //     transaction: TransactionArg,
    //     protocol_version: Option<u32>,
    // ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
    //     let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;
    //     let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
    //     let (items, skipped) = self.query_documents_for_cbor_query_internal(
    //         contract,
    //         document_type,
    //         query_cbor,
    //         transaction,
    //         &mut drive_operations,
    //         protocol_version,
    //     )?;
    //     let cost = if let Some(block_info) = block_info {
    //         let fee_result = Drive::calculate_fee(
    //             None,
    //             Some(drive_operations),
    //             &block_info.epoch,
    //             platform_version,
    //         )?;
    //         fee_result.processing_fee
    //     } else {
    //         0
    //     };
    //     Ok((items, skipped, cost))
    // }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::option::Option::None;

    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use rand::random;

    use crate::config::DriveConfig;
    use crate::drive::document::query::{
        QueryDocumentsOutcomeV0Methods, QueryDocumentsWithFlagsOutcomeV0Methods,
    };
    use crate::drive::document::tests::setup_dashpay;
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::setup_contract;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_query_documents_dry_run() {
        let (drive, contract) = setup_dashpay("query-dry-run", true);
        let platform_version = PlatformVersion::latest();

        let sql_string = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        // Dry run should return empty results without touching storage
        let outcome = drive
            .query_documents(query, None, true, None, None)
            .expect("expected dry run query to succeed");

        assert_eq!(outcome.documents().len(), 0);
        assert_eq!(outcome.skipped(), 0);
        assert_eq!(outcome.cost(), 0);
    }

    #[test]
    fn test_query_documents_with_epoch_fee() {
        let (drive, contract) = setup_dashpay("query-epoch", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");

        let sql_string = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        // Query with epoch to exercise the fee calculation path
        let epoch = Epoch::new(0).unwrap();
        let outcome = drive
            .query_documents(query, Some(&epoch), false, None, None)
            .expect("expected query with epoch to succeed");

        assert_eq!(outcome.documents().len(), 1);
        assert!(outcome.cost() > 0, "cost should be non-zero with epoch");
    }

    #[test]
    fn test_query_documents_without_epoch_zero_cost() {
        let (drive, contract) = setup_dashpay("query-no-epoch", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");

        let sql_string = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        // Query without epoch, cost should be 0
        let outcome = drive
            .query_documents(query, None, false, None, None)
            .expect("expected query without epoch to succeed");

        assert_eq!(outcome.documents().len(), 1);
        assert_eq!(outcome.cost(), 0, "cost should be zero without epoch");
    }

    #[test]
    fn test_query_documents_with_flags() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let person_document = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        // Query documents with flags to cover the query_documents_with_flags path
        let outcome = drive
            .query_documents_with_flags(query, None, false, None, None)
            .expect("expected query_documents_with_flags to succeed");

        assert_eq!(outcome.documents().len(), 1);
        assert_eq!(outcome.cost(), 0);
        // Verify storage flags are returned
        let (_, flags) = &outcome.documents()[0];
        assert!(flags.is_some(), "storage flags should be present");
    }

    #[test]
    fn test_query_documents_with_flags_dry_run() {
        let (drive, contract) = setup_dashpay("query-flags-dry", true);
        let platform_version = PlatformVersion::latest();

        let sql_string = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        // Dry run should return empty defaults
        let outcome = drive
            .query_documents_with_flags(query, None, true, None, None)
            .expect("expected dry run query_documents_with_flags to succeed");

        assert_eq!(outcome.documents().len(), 0);
        assert_eq!(outcome.skipped(), 0);
        assert_eq!(outcome.cost(), 0);
    }

    #[test]
    fn test_query_documents_with_flags_with_epoch() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let person_document = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let epoch = Epoch::new(0).unwrap();
        let outcome = drive
            .query_documents_with_flags(query, Some(&epoch), false, None, None)
            .expect("expected query_documents_with_flags with epoch to succeed");

        assert_eq!(outcome.documents().len(), 1);
        assert!(
            outcome.cost() > 0,
            "cost should be non-zero when epoch is provided"
        );
    }
}
