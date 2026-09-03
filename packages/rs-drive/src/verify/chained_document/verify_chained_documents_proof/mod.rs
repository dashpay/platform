mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_chained_document_query::{
    ChainedDocumentsResult, DriveChainedDocumentQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveChainedDocumentQuery<'_> {
    /// Verifies a chained query's single merged proof and returns
    /// `(root_hash, result)`.
    ///
    /// The verifier trusts nothing about the join, and needs nothing
    /// beyond the proof itself: a BOOTSTRAP subset pass runs the inner
    /// query alone against the merged proof and extracts candidate
    /// join values from its proven positions; the outer by-ids
    /// component is derived from those (exactly as the prover derived
    /// it from its materialization), the merged query is rebuilt, and
    /// the AUTHORITATIVE full pass verifies the whole composition —
    /// grovedb enforces the inner page's lifted per-instance limit and
    /// range completeness — with the proven outer documents required
    /// to match the proven inner join values exactly. A missing
    /// referenced document is an invalid proof (`refersTo:
    /// permanentDocument` targets cannot dangle), and so is an extra
    /// one; a proof covering only the inner half (an old node serving
    /// the plain query) fails the full pass whenever the inner page is
    /// non-empty.
    ///
    /// One proof means one root by construction; the caller combines
    /// the returned root hash with the surrounding tenderdash
    /// signature — see `rs-drive-proof-verifier` for the canonical
    /// composition.
    pub fn verify_chained_documents_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ChainedDocumentsResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .chained_document
            .verify_chained_documents_proof
        {
            0 => self.verify_chained_documents_proof_v0(proof, platform_version),
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

        let result = query.verify_chained_documents_proof(&[], &platform_version);
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnknownVersionMismatch { method, .. }))
                if method == "DriveChainedDocumentQuery::verify_chained_documents_proof"
        ));
    }
}
