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
    /// The verifier trusts nothing about the join. `join_values_hint`
    /// is the server's CLAIMED join-value list — untrusted bootstrap
    /// data used only to reconstruct the merged query (the outer by-ids
    /// component is derived from it, exactly as the prover derived it
    /// from its materialization). The proof is then verified against
    /// that reconstruction in ONE pass — grovedb enforces the inner
    /// page's lifted per-instance limit and range completeness — and
    /// the PROVEN inner join values are extracted and required to match
    /// the proven outer documents exactly. A hint that lies in any
    /// direction (extra, missing, or substituted ids) produces a merged
    /// query the proof cannot satisfy consistently, and verification
    /// fails: a missing referenced document is an invalid proof
    /// (`refersTo: permanentDocument` targets cannot dangle), and so is
    /// an extra one.
    ///
    /// One proof means one root by construction; the caller combines
    /// the returned root hash with the surrounding tenderdash
    /// signature — see `rs-drive-proof-verifier` for the canonical
    /// composition.
    pub fn verify_chained_documents_proof(
        &self,
        proof: &[u8],
        join_values_hint: &[dpp::identifier::Identifier],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ChainedDocumentsResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .chained_document
            .verify_chained_documents_proof
        {
            0 => self.verify_chained_documents_proof_v0(proof, join_values_hint, platform_version),
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

        let result = query.verify_chained_documents_proof(&[], &[], &platform_version);
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnknownVersionMismatch { method, .. }))
                if method == "DriveChainedDocumentQuery::verify_chained_documents_proof"
        ));
    }
}
