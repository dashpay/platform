use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::drive_chained_document_query::{
    ChainedDocumentsResult, DriveChainedDocumentQuery,
};
use crate::query::index_only_synthesis::synthesize_index_only_document;
use crate::verify::RootHash;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::{GroveDb, PathQuery};

impl DriveChainedDocumentQuery<'_> {
    /// v0 of the chained proof verification — see the versioned wrapper
    /// for the trust model.
    #[inline(always)]
    pub(super) fn verify_chained_documents_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ChainedDocumentsResult), Error> {
        self.validate(platform_version)?;
        let grove_version = &platform_version.drive.grove_version;

        // BOOTSTRAP PASS: run the inner query alone against the merged
        // proof (subset verification — succinctness off, so the outer
        // branch's extra coverage is tolerated) and extract the join
        // values from its proven positions. These are only CANDIDATES
        // for reconstructing the merged query; the full pass below is
        // the authority, so nothing rests on this pass's completeness
        // semantics.
        let inner_path_query = self.inner.construct_path_query(None, platform_version)?;
        let (_, bootstrap_trios) =
            GroveDb::verify_subset_query(proof, &inner_path_query, grove_version)?;
        let index = self.inner.index_only_query_index(platform_version)?;
        let bootstrap_documents = bootstrap_trios
            .into_iter()
            .filter(|(_, _, element)| element.is_some())
            .map(|(path, key, _)| {
                synthesize_index_only_document(
                    self.inner.contract.id(),
                    self.inner.document_type,
                    index,
                    &path,
                    &key,
                )
            })
            .collect::<Result<Vec<Document>, Error>>()?;
        let candidate_join_values = self.join_values(&bootstrap_documents)?;

        // AUTHORITATIVE PASS: re-derive the outer component from the
        // candidates, re-merge at the same grove version (identical to
        // the prover's merge by the single-builder rule), and verify
        // the whole composition with succinctness on — grovedb enforces
        // the inner page's lifted per-instance limit and range
        // completeness here. A proof that covers only the inner half
        // (e.g. a node that predates the chained surface serving the
        // plain inner query) fails this pass whenever the candidates
        // are non-empty: the merged query demands outer coverage the
        // proof cannot supply.
        let path_queries = self.proof_path_queries(&candidate_join_values, platform_version)?;
        let path_query_refs: Vec<&PathQuery> = path_queries.iter().collect();
        let merged_query = if path_query_refs.len() > 1 {
            PathQuery::merge(path_query_refs, grove_version)?
        } else {
            path_queries[0].clone()
        };

        let (root_hash, proved_path_key_values) =
            GroveDb::verify_query(proof, &merged_query, grove_version)?;

        // Split the proved trios between the halves by their doctype
        // path segment: `[DataContractDocuments, contract_id, 1,
        // <doctype>, …]`.
        let inner_type_name = self.inner.document_type.name().as_bytes();
        let outer_type_name = self.outer_document_type.name().as_bytes();
        let mut inner_documents: Vec<Document> = Vec::new();
        let mut outer_documents: Vec<Document> = Vec::new();
        for (path, key, element) in proved_path_key_values {
            let Some(element) = element else {
                continue;
            };
            match path.get(3).map(|segment| segment.as_slice()) {
                Some(segment) if segment == inner_type_name => {
                    inner_documents.push(synthesize_index_only_document(
                        self.inner.contract.id(),
                        self.inner.document_type,
                        index,
                        &path,
                        &key,
                    )?);
                }
                Some(segment) if segment == outer_type_name => {
                    let grovedb::Element::Item(serialized, _) = element else {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "chained proof's outer half proved a non-item element where a \
                             stored document was expected"
                                .to_string(),
                        )));
                    };
                    outer_documents.push(
                        Document::from_bytes(
                            serialized.as_slice(),
                            self.outer_document_type,
                            platform_version,
                        )
                        .map_err(|e| Error::Protocol(Box::new(e)))?,
                    );
                }
                _ => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "chained proof proved an entry outside both document types' \
                         subtrees"
                            .to_string(),
                    )));
                }
            }
        }

        // The full pass's PROVEN join values are authoritative — the
        // bootstrap candidates were only for reconstructing the query —
        // and the exact-set assembly refuses any divergence between
        // them and the proven outer documents, in either direction.
        let join_values = self.join_values(&inner_documents)?;
        let outer_documents = self.assemble_outer_documents(&join_values, outer_documents)?;

        Ok((
            root_hash,
            ChainedDocumentsResult {
                inner_documents,
                outer_documents,
            },
        ))
    }
}
