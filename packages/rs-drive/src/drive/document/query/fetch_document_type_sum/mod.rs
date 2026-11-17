mod v0;

use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

impl Drive {
    /// Reads the total a document type keeps of its summed property over all its documents
    /// (`documentsSummable` or `documentsAverageable`), and returns the fee of the read.
    ///
    /// State validation reads it before a document create or replace: with values that are
    /// never negative every other sum of the type is a sum over some of the same documents,
    /// so a write that keeps this total in range keeps them all in range.
    ///
    /// # Parameters
    ///
    /// * `contract` - The data contract of the document type.
    /// * `document_type` - The document type, which must keep the total.
    /// * `epoch` - The epoch the read is priced at; `None` returns no fee.
    /// * `transaction` - The transaction to read in.
    /// * `platform_version` - The platform version selecting the method.
    ///
    /// # Returns
    ///
    /// The total and the fee of reading it. An error when the type keeps no total, or its
    /// primary tree is missing.
    pub fn fetch_document_type_sum(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(i64, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_type_sum
        {
            Some(0) => self.fetch_document_type_sum_v0(
                contract,
                document_type,
                epoch,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_type_sum".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_document_type_sum".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
