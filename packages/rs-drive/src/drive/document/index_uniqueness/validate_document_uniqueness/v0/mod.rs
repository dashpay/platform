use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequest;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;

use dpp::validation::SimpleConsensusValidationResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Validate that a document would be unique in the state
    #[inline(always)]
    pub(super) fn validate_document_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        owner_id: Identifier,
        allow_original: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: document.id(),
            allow_original,
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            data: document.properties(),
        };
        self.validate_uniqueness_of_data(request, transaction, platform_version)
    }
}
