use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::DriveDocumentQuery;
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

/// The outcome of a query
#[derive(Debug, Default)]
pub struct QueryDocumentsWithFlagsOutcomeV0 {
    documents: Vec<(Document, Option<StorageFlags>)>,
    skipped: u16,
    cost: u64,
}

/// Trait defining methods associated with `QueryDocumentsOutcomeV0`.
///
/// This trait provides a set of methods to interact with and retrieve
/// details from an instance of `QueryDocumentsOutcomeV0`. These methods
/// include retrieving the documents, skipped count, and the associated cost
/// of the query.
pub trait QueryDocumentsWithFlagsOutcomeV0Methods {
    /// Returns a reference to the documents and storage flags found from the query.
    fn documents(&self) -> &Vec<(Document, Option<StorageFlags>)>;
    /// Consumes the instance to return the owned documents with storage flags.
    fn documents_owned(self) -> Vec<(Document, Option<StorageFlags>)>;
    /// Returns the count of items that were skipped during the query.
    fn skipped(&self) -> u16;
    /// Returns the processing cost associated with the query.
    fn cost(&self) -> u64;
}

impl QueryDocumentsWithFlagsOutcomeV0Methods for QueryDocumentsWithFlagsOutcomeV0 {
    fn documents(&self) -> &Vec<(Document, Option<StorageFlags>)> {
        &self.documents
    }

    fn documents_owned(self) -> Vec<(Document, Option<StorageFlags>)> {
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
    pub(super) fn query_documents_with_flags_v0(
        &self,
        query: DriveDocumentQuery,
        epoch: Option<&Epoch>,
        dry_run: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryDocumentsWithFlagsOutcomeV0, Error> {
        if dry_run {
            return Ok(QueryDocumentsWithFlagsOutcomeV0::default());
        }
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let (items, skipped) = query.execute_no_proof_internal(
            self,
            QueryResultType::QueryElementResultType,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let documents = items
            .to_elements()
            .into_iter()
            .map(|element| {
                let serialized_item = element.as_item_bytes()?;
                let document =
                    Document::from_bytes(serialized_item, query.document_type, platform_version)?;
                let storage_flags = StorageFlags::map_some_element_flags_ref(element.get_flags())?;
                Ok((document, storage_flags))
            })
            .collect::<Result<Vec<(Document, Option<StorageFlags>)>, Error>>()?;
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

        Ok(QueryDocumentsWithFlagsOutcomeV0 {
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
    fn query_documents_with_flags_v0_dry_run_returns_default() {
        // Covers the `if dry_run` short-circuit.
        let (drive, contract) = setup_dashpay("qdwf-v0-dry", true);
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
            .query_documents_with_flags_v0(query, None, true, None, platform_version)
            .expect("dry run succeeds");

        assert_eq!(outcome.documents().len(), 0);
        assert_eq!(outcome.skipped(), 0);
        assert_eq!(outcome.cost(), 0);
    }

    #[test]
    fn query_documents_with_flags_v0_empty_results() {
        // Queries a document-type with zero stored docs — exercises the
        // element-iteration loop on an empty vec and the no-epoch cost branch.
        let (drive, contract) = setup_dashpay("qdwf-v0-empty", true);
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
            .query_documents_with_flags_v0(query, None, false, None, platform_version)
            .expect("query succeeds");

        assert!(outcome.documents().is_empty());
        assert_eq!(outcome.cost(), 0);
    }

    #[test]
    fn query_documents_with_flags_outcome_methods_v0() {
        // Covers the trait accessors on the concrete struct.
        let outcome = QueryDocumentsWithFlagsOutcomeV0::default();
        assert_eq!(outcome.documents().len(), 0);
        assert_eq!(outcome.skipped(), 0);
        assert_eq!(outcome.cost(), 0);
        let taken = outcome.documents_owned();
        assert!(taken.is_empty());
    }
}
