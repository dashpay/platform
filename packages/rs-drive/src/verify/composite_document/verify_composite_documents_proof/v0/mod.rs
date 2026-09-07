use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::drive_composite_document_query::{
    CompositeDocumentsResult, DriveCompositeDocumentQuery, PresentTrio, ProvedTrio,
};
use crate::verify::RootHash;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveCompositeDocumentQuery<'_> {
    /// v0 of the composite proof verification — see the versioned
    /// wrapper for the trust model.
    #[inline(always)]
    pub(super) fn verify_composite_documents_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, CompositeDocumentsResult), Error> {
        self.validate(platform_version)?;
        let grove_version = &platform_version.drive.grove_version;

        let present = |trios: Vec<ProvedTrio>| {
            trios
                .into_iter()
                .filter_map(|(path, key, element)| element.map(|element| (path, key, element)))
                .collect::<Vec<PresentTrio>>()
        };

        // BOOTSTRAP PASS: the page alone against the merged proof (subset
        // verification — succinctness off, so the sub-query branches'
        // extra coverage is tolerated), decoded into candidate documents.
        // Candidates only reconstruct the merged query; the full pass
        // below is the authority.
        let page_path_query = self.page_path_query(platform_version)?;
        let direction = page_path_query.query.query.left_to_right;
        let (_, page_trios) = GroveDb::verify_subset_query(proof, &page_path_query, grove_version)?;
        let bootstrap_page =
            Self::decode_document_trios(&self.page, present(page_trios), platform_version)?;

        // Derive every sub-query in order. A sub-query that feeds a later
        // binding is itself bootstrapped by a subset pass, so the later
        // binding has candidates to derive from.
        let mut derived = Vec::with_capacity(self.sub_queries.len());
        let mut bootstrap_sub_documents: Vec<Option<Vec<Document>>> =
            vec![None; self.sub_queries.len()];
        for (index, sub_query) in self.sub_queries.iter().enumerate() {
            let values = self.derive_for(sub_query, &bootstrap_page, |source| {
                bootstrap_sub_documents
                    .get(source)
                    .and_then(|documents| documents.as_deref())
            })?;
            if self.is_binding_source(index) {
                let documents = if sub_query.binding.is_some() && values.is_empty() {
                    Vec::new()
                } else {
                    let path_query = self.sub_query_proof_path_query(
                        sub_query,
                        &values,
                        direction,
                        platform_version,
                    )?;
                    let (_, trios) =
                        GroveDb::verify_subset_query(proof, &path_query, grove_version)?;
                    self.decode_sub_query_document_trios(
                        sub_query,
                        &values,
                        direction,
                        present(trios),
                        platform_version,
                    )?
                };
                bootstrap_sub_documents[index] = Some(documents);
            }
            derived.push(values);
        }

        // AUTHORITATIVE PASS: rebuild every component from the candidates,
        // re-merge at the same grove version (identical to the prover's
        // merge by the single-builder rule), and verify the whole
        // composition with succinctness on.
        let (page_path_query, sub_path_queries) =
            self.proof_path_queries(&derived, platform_version)?;
        let merged_query =
            Self::merged_path_query(&page_path_query, &sub_path_queries, platform_version)?;
        let (root_hash, proved_trios) = GroveDb::verify_query(proof, &merged_query, grove_version)?;

        let result = self.assemble_from_trios(
            &derived,
            &page_path_query,
            &sub_path_queries,
            proved_trios,
            platform_version,
        )?;

        // The PROVEN results are authoritative. The bootstrap read the
        // same proof through each component's own query, so what it
        // decoded must be exactly what the routed authoritative pass
        // assigned to that component — a difference means the routing
        // misassigned an entry — and every derivation must come out
        // identical from the proven results, or the proof was built over
        // a different page than it proves.
        if bootstrap_page != result.page_documents {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "the composite proof's page differs between the page query alone and the \
                 merged composition"
                    .to_string(),
            )));
        }
        for (index, bootstrapped) in bootstrap_sub_documents.iter().enumerate() {
            let Some(bootstrapped) = bootstrapped else {
                continue;
            };
            if bootstrapped.as_slice() != result.sub_results[index].documents() {
                return Err(Error::Proof(ProofError::CorruptedProof(format!(
                    "the composite proof's sub-query {} differs between its own query and the \
                     merged composition",
                    index
                ))));
            }
        }
        let authoritative = self.derive_all(&result.page_documents, |index| {
            result
                .sub_results
                .get(index)
                .map(|result| result.documents())
        })?;
        if authoritative != derived {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "the composite proof's page derives different sub-query values than the \
                 ones the proof covers"
                    .to_string(),
            )));
        }

        Ok((root_hash, result))
    }
}
