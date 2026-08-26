use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::DriveDocumentQuery;
use dpp::block::epoch::Epoch;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// The outcome of a query
#[derive(Debug, Default)]
pub struct QueryDocumentsOutcomeV0 {
    documents: Vec<Document>,
    skipped: u16,
    cost: u64,
}

/// Trait defining methods associated with `QueryDocumentsOutcomeV0`.
///
/// This trait provides a set of methods to interact with and retrieve
/// details from an instance of `QueryDocumentsOutcomeV0`. These methods
/// include retrieving the documents, skipped count, and the associated cost
/// of the query.
pub trait QueryDocumentsOutcomeV0Methods {
    /// Returns a reference to the documents found from the query.
    fn documents(&self) -> &Vec<Document>;
    /// Consumes the instance to return the owned documents.
    fn documents_owned(self) -> Vec<Document>;
    /// Returns the count of items that were skipped during the query.
    fn skipped(&self) -> u16;
    /// Returns the processing cost associated with the query.
    fn cost(&self) -> u64;
}

impl QueryDocumentsOutcomeV0Methods for QueryDocumentsOutcomeV0 {
    fn documents(&self) -> &Vec<Document> {
        &self.documents
    }

    fn documents_owned(self) -> Vec<Document> {
        self.documents
    }

    fn skipped(&self) -> u16 {
        self.skipped
    }

    fn cost(&self) -> u64 {
        self.cost
    }
}

impl Drive {
    /// Performs a specified drive query and returns the result, along with any skipped items and the cost.
    ///
    /// This function is used to execute a given [DriveDocumentQuery]. It has options to operate in a dry-run mode
    /// and supports different protocol versions. In case an epoch is specified, it calculates the fee.
    ///
    /// # Arguments
    ///
    /// * `query` - The [DriveDocumentQuery] being executed.
    /// * `epoch` - An `Option<&Epoch>`. If provided, it will be used to calculate the processing fee.
    /// * `dry_run` - If true, the function will not perform any actual operation and return a default `QueryDocumentsOutcome`.
    /// * `transaction` - The `TransactionArg` holding the transaction data.
    /// * `platform_version` - A reference to the `PlatformVersion` object specifying the version of functions to call.
    ///
    /// # Returns
    ///
    /// * `Result<QueryDocumentsOutcome, Error>` - Returns `QueryDocumentsOutcome` on success with the list of documents,
    ///   number of skipped items, and cost. If the operation fails, it returns an `Error`.
    #[inline(always)]
    pub(super) fn query_documents_v0(
        &self,
        query: DriveDocumentQuery,
        epoch: Option<&Epoch>,
        dry_run: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryDocumentsOutcomeV0, Error> {
        if dry_run {
            return Ok(QueryDocumentsOutcomeV0::default());
        }
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let (items, skipped) = query.execute_raw_results_no_proof_internal(
            self,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let documents = items
            .into_iter()
            .map(|serialized| {
                Document::from_bytes(serialized.as_slice(), query.document_type, platform_version)
                    .map_err(|e| {
                        Error::ProtocolWithInfoString(
                            Box::new(e),
                            format!(
                                "document bytes are {}, query is using contract {:?}",
                                hex::encode(serialized),
                                query.contract
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<Document>, Error>>()?;
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

        Ok(QueryDocumentsOutcomeV0 {
            documents,
            skipped,
            cost,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DriveConfig;
    use crate::drive::document::tests::setup_dashpay;
    use crate::query::DriveDocumentQuery;

    #[test]
    fn query_documents_v0_dry_run_returns_default() {
        // Dry run short-circuits before touching storage.
        let (drive, contract) = setup_dashpay("qd-v0-dry", true);
        let platform_version = PlatformVersion::latest();

        let sql = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("valid query");

        let outcome = drive
            .query_documents_v0(query, None, true, None, platform_version)
            .expect("dry run succeeds");

        assert_eq!(outcome.documents().len(), 0);
        assert_eq!(outcome.skipped(), 0);
        assert_eq!(outcome.cost(), 0);
    }

    #[test]
    fn query_documents_v0_returns_empty_for_unpopulated_contract() {
        // Contract inserted but no documents — the raw path query executes and
        // the Document::from_bytes conversion loop runs over an empty vec.
        let (drive, contract) = setup_dashpay("qd-v0-empty", true);
        let platform_version = PlatformVersion::latest();

        let sql = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("valid query");

        let outcome = drive
            .query_documents_v0(query, None, false, None, platform_version)
            .expect("query succeeds");

        assert!(outcome.documents().is_empty());
        assert_eq!(outcome.cost(), 0, "no epoch => cost is 0");
    }

    #[test]
    fn query_documents_outcome_methods_v0() {
        // Covers the trait-method accessor branches on the concrete struct.
        let outcome = QueryDocumentsOutcomeV0::default();
        assert_eq!(outcome.documents().len(), 0);
        assert_eq!(outcome.skipped(), 0);
        assert_eq!(outcome.cost(), 0);
        let taken: Vec<_> = outcome.documents_owned();
        assert!(taken.is_empty());
    }

    #[test]
    fn query_documents_v0_returns_inserted_document() {
        // Exercises the non-dry-run path through execute_raw_results_no_proof_internal
        // and the Document::from_bytes mapping loop.
        use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
        use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
        use crate::util::storage_flags::StorageFlags;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::json_document::json_document_to_document;

        let (drive, contract) = setup_dashpay("qd-v0-insert", true);
        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest");

        let owner_id = rand::random::<[u8; 32]>();
        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("json doc");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_id),
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
            .expect("insert");

        let sql = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("valid query");

        let outcome = drive
            .query_documents_v0(query, None, false, None, platform_version)
            .expect("query succeeds");

        // The from_bytes loop runs and produces exactly one document.
        assert_eq!(outcome.documents().len(), 1);
        assert_eq!(outcome.cost(), 0, "no epoch => cost stays zero");
    }
}
