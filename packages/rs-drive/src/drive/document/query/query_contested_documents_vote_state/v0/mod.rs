use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use grovedb::TransactionArg;

/// The outcome of a query
#[derive(Debug, Default)]
pub struct QueryContestedDocumentsVoteStateOutcomeV0 {
    contenders: Vec<ContenderWithSerializedDocument>,
    cost: u64,
}

/// Trait defining methods associated with `QueryDocumentsOutcomeV0`.
///
/// This trait provides a set of methods to interact with and retrieve
/// details from an instance of `QueryDocumentsOutcomeV0`. These methods
/// include retrieving the documents, skipped count, and the associated cost
/// of the query.
pub trait QueryContestedDocumentsVoteStateOutcomeV0Methods {
    /// Returns a reference to the contenders found from the query.
    fn contenders(&self) -> &Vec<ContenderWithSerializedDocument>;
    /// Consumes the instance to return the owned contenders.
    fn contenders_owned(self) -> Vec<ContenderWithSerializedDocument>;
    /// Returns the processing cost associated with the query.
    fn cost(&self) -> u64;
}

impl QueryContestedDocumentsVoteStateOutcomeV0Methods
    for QueryContestedDocumentsVoteStateOutcomeV0
{
    fn contenders(&self) -> &Vec<ContenderWithSerializedDocument> {
        &self.contenders
    }

    fn contenders_owned(self) -> Vec<ContenderWithSerializedDocument> {
        self.contenders
    }

    fn cost(&self) -> u64 {
        self.cost
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
    pub(super) fn query_contested_documents_v0(
        &self,
        query: ContestedDocumentVotePollDriveQuery,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryContestedDocumentsVoteStateOutcomeV0, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let contested_document_vote_poll_drive_query_execution_result =
            query.execute_no_proof(self, transaction, &mut drive_operations, platform_version)?;
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

        Ok(QueryContestedDocumentsVoteStateOutcomeV0 {
            contenders: contested_document_vote_poll_drive_query_execution_result.contenders,
            cost,
        })
    }
}
