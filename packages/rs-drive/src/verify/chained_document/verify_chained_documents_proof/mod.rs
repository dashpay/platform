mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_chained_document_query::{
    ChainedDocumentsResult, DriveChainedDocumentQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveChainedDocumentQuery<'_> {
    /// Verifies a chained query's two proofs as ONE composed statement
    /// and returns `(root_hash, result)`.
    ///
    /// The verifier trusts nothing about the join: it verifies the inner
    /// proof against the inner query it built itself, extracts the join
    /// values from the proven inner projections, DERIVES the outer
    /// by-ids query from them (the server never transmits it), verifies
    /// the outer proof against that derived query, requires both root
    /// hashes to be EQUAL, and requires the proven outer documents to
    /// match the derived ids exactly — a missing referenced document is
    /// an invalid proof (`refersTo: permanentDocument` targets cannot
    /// dangle), and so is an extra one.
    ///
    /// `outer_proof` must be `Some` if and only if the proven inner page
    /// is non-empty.
    ///
    /// The returned root hash is shared by both proofs; the caller
    /// combines it with the surrounding tenderdash signature — see
    /// `rs-drive-proof-verifier` for the canonical composition.
    pub fn verify_chained_documents_proof(
        &self,
        inner_proof: &[u8],
        outer_proof: Option<&[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ChainedDocumentsResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .chained_document
            .verify_chained_documents_proof
        {
            0 => self.verify_chained_documents_proof_v0(inner_proof, outer_proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveChainedDocumentQuery::verify_chained_documents_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::query::DriveDocumentQuery;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;

    #[test]
    fn test_verify_chained_documents_proof_unknown_version() {
        let platform_version = dpp::version::PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");
        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected domain document type");

        let mut platform_version = platform_version.clone();
        platform_version
            .drive
            .methods
            .verify
            .chained_document
            .verify_chained_documents_proof = 255;

        let query = DriveChainedDocumentQuery {
            inner: DriveDocumentQuery {
                contract: &contract,
                document_type,
                internal_clauses: Default::default(),
                offset: None,
                limit: Some(1),
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
                resolved_time_ranges: vec![],
            },
            join_property: "records".to_string(),
            outer_document_type: document_type,
        };

        let result = query.verify_chained_documents_proof(&[], None, &platform_version);
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnknownVersionMismatch { method, .. }))
                if method == "DriveChainedDocumentQuery::verify_chained_documents_proof"
        ));
    }
}
