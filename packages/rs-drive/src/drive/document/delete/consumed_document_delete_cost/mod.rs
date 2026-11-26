//! Version selection for pricing the delete of a document a create consumes.

mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

impl Drive {
    /// Prices, by estimation and without reading state, the delete of one document of
    /// `document_type` that a create consumes (a `refersTo` lookup declaring `consume`), which
    /// check tx reserves for a create whose state it does not validate, as the create's
    /// conversion makes it, on its own: a block prices it inside the create's batch, sharing its
    /// layers, so at most at this. Generation 0 prices the by-id force delete, read included, of
    /// the create conversion 0 (protocol version 14); generation 1 the delete from what the
    /// lookup read, without reading the document again, of the create conversion 1 (protocol
    /// version 15, `ForceDeleteReadDocument`). A generation of that delete that costs more needs
    /// a generation here that prices it.
    ///
    /// # Parameters
    /// * `contract`: The contract holding the document type.
    /// * `document_type`: The type of the consumed document.
    /// * `block_info`: The block the delete is priced in: its epoch's fee schedule, and its time
    ///   for a type whose documents expire.
    /// * `platform_version`: Selects the method and the delete it prices.
    ///
    /// # Returns
    /// * `Ok(FeeResult)`: the estimated fee of the delete.
    /// * `Err(Error)` when the method version is unknown, or the type's documents cannot be
    ///   deleted by a consume, which registration refuses.
    pub fn consumed_document_delete_cost(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .consumed_document_delete_cost
        {
            0 => self.consumed_document_delete_cost_v0(
                contract,
                document_type,
                block_info,
                platform_version,
            ),
            1 => self.consumed_document_delete_cost_v1(
                contract,
                document_type,
                block_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "consumed_document_delete_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
