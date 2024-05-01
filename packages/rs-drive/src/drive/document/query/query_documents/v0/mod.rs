use crate::drive::document::query::query_documents::QueryDocumentsOutcome;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::DriveQuery;
use dpp::block::epoch::Epoch;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
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

impl From<QueryDocumentsOutcomeV0> for QueryDocumentsOutcome {
    fn from(val: QueryDocumentsOutcomeV0) -> Self {
        QueryDocumentsOutcome::V0(val)
    }
}

impl Drive {
    /// Performs a specified drive query and returns the result, along with any skipped items and the cost.
    ///
    /// This function is used to execute a given [DriveQuery]. It has options to operate in a dry-run mode
    /// and supports different protocol versions. In case an epoch is specified, it calculates the fee.
    ///
    /// # Arguments
    ///
    /// * `query` - The [DriveQuery] being executed.
    /// * `epoch` - An `Option<&Epoch>`. If provided, it will be used to calculate the processing fee.
    /// * `dry_run` - If true, the function will not perform any actual operation and return a default `QueryDocumentsOutcome`.
    /// * `transaction` - The `TransactionArg` holding the transaction data.
    /// * `platform_version` - A reference to the `PlatformVersion` object specifying the version of functions to call.
    ///
    /// # Returns
    ///
    /// * `Result<QueryDocumentsOutcome, Error>` - Returns `QueryDocumentsOutcome` on success with the list of documents,
    ///    number of skipped items, and cost. If the operation fails, it returns an `Error`.
    #[inline(always)]
    pub(super) fn query_documents_v0(
        &self,
        query: DriveQuery,
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
            })
            .collect::<Result<Vec<Document>, ProtocolError>>()?;
        let cost = if let Some(epoch) = epoch {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                epoch,
                self.config.epochs_per_era,
                platform_version,
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
