//! Verified **composite document** results: a page plus the
//! sub-queries derived from it, answered as ONE merged grovedb proof.
//!
//! The server proves the limited page and every sub-query (by-id
//! joins, indexed lookups, grouped counts, siblings) as one merged
//! path query. The verifier
//! ([`DriveDocumentQuery::verify_composite_documents_proof`])
//! bootstraps the page from the proof with a subset pass, re-derives
//! every sub-query's `IN` clause from the PROVEN page (or the proven
//! earlier sub-query it binds), rebuilds the same merged query, verifies
//! it in one authoritative pass, and routes the proved entries back to
//! their components — refusing unclaimed entries, dangling joins and
//! derivation divergence. This module's [`FromProof`] impl composes
//! that with the tenderdash signature binding of the single root.
//!
//! There is deliberately **no unproven decoder with verification
//! semantics** here: an unproven composite response is free to
//! fabricate any sub-result, which is precisely what the surface exists
//! to prevent. [`CompositeDocuments`] can still be built from a trusted
//! node's unproven wire by the SDK if it chooses, but the canonical
//! path proves.

use crate::error::MapGroveDbError;
use crate::verify::{document_query_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::document_type::{
    DecodedValueBudget, DocumentTypeRef, DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET,
};
use dpp::data_contract::errors::DataContractError;
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::contract::moderation::types::ContractDocumentRemovalEntry;
use drive::query::{DriveDocumentQuery, SubQueryResult};
use drive::verify::RootHash;
use std::collections::BTreeMap;

/// The verified result of a composite document query.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompositeDocuments {
    /// The page, exactly as the page query alone would return it.
    pub page_documents: Vec<Document>,
    /// One result per sub-query, in request order: a by-id join's
    /// documents in first-appearance order of their ids among the
    /// source documents, a lookup's or sibling's in query order, or one
    /// count per derived value that has a count tree (a value without
    /// an entry counts zero).
    pub sub_results: Vec<SubQueryResult>,
    /// One list per sub-query, in request order: for a by-id join off a
    /// `deletableDocument` property, the derived ids that have NO
    /// document, in first-appearance order, each one a PROVEN absence;
    /// empty for every other sub-query.
    pub sub_result_missing_ids: Vec<Vec<dpp::identifier::Identifier>>,
    /// One list per sub-query, in request order: for a by-id join off a
    /// `moderatedDocument` property, the derived ids whose document the
    /// contract's moderators removed, each with its PROVEN removal record,
    /// in first-appearance order; empty for every other sub-query. There a
    /// derived id with neither a document nor a record is an invalid proof.
    pub sub_result_removals: Vec<Vec<ContractDocumentRemovalEntry>>,
}

impl CompositeDocuments {
    /// Decodes the kept field values of every removal record across every
    /// sub-query, enforcing [`DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET`] across
    /// the page documents, every sub-query's documents, and every sub-query's
    /// removal records returned together in this response.
    ///
    /// `sub_query_document_type(index)` supplies the document type of sub-query
    /// `index` whenever `self.sub_result_removals[index]` is non-empty.
    pub fn decode_sub_result_removal_kept_values<'a>(
        &self,
        sub_query_document_type: impl FnMut(usize) -> Result<DocumentTypeRef<'a>, DataContractError>,
    ) -> Result<Vec<Vec<BTreeMap<String, Value>>>, ProtocolError> {
        let mut budget = DecodedValueBudget::new(DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET);
        self.decode_sub_result_removal_kept_values_with_budget(sub_query_document_type, &mut budget)
    }

    /// Like [`Self::decode_sub_result_removal_kept_values`], charging every
    /// retained document and decoded kept field across the entire composite
    /// response against a shared `budget`.
    pub fn decode_sub_result_removal_kept_values_with_budget<'a>(
        &self,
        mut sub_query_document_type: impl FnMut(usize) -> Result<DocumentTypeRef<'a>, DataContractError>,
        budget: &mut DecodedValueBudget,
    ) -> Result<Vec<Vec<BTreeMap<String, Value>>>, ProtocolError> {
        budget
            .charge_documents(&self.page_documents)
            .map_err(ProtocolError::DataContractError)?;
        for sub_result in &self.sub_results {
            budget
                .charge_documents(sub_result.documents())
                .map_err(ProtocolError::DataContractError)?;
        }
        self.sub_result_removals
            .iter()
            .enumerate()
            .map(|(index, removals)| {
                if removals.is_empty() {
                    return Ok(Vec::new());
                }
                let document_type =
                    sub_query_document_type(index).map_err(ProtocolError::DataContractError)?;
                removals
                    .iter()
                    .map(|entry| entry.removal.kept_values_with_budget(document_type, budget))
                    .collect()
            })
            .collect()
    }
}

/// Verify a composite query's single merged proof and bind its root
/// hash to the quorum signature.
///
/// The merk-level composition (bootstrap subset pass on the page,
/// re-derivation of every sub-query, authoritative full verification,
/// routing with the completeness checks) lives in rs-drive's
/// [`DriveDocumentQuery::verify_composite_documents_proof`];
/// this wrapper adds the [`verify_tenderdash_proof`] binding — the root
/// hash the proof commits to is only an attested fact once it is tied
/// to the quorum-signed app hash, and this function exists so the
/// composition can never be skipped by accident.
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_composite_documents_proof(
    query: &DriveDocumentQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, CompositeDocuments), Error> {
    let (root_hash, result) = query
        .verify_composite_documents_proof(
            document_query_proof_bytes(proof, mtd, platform_version)?,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((
        root_hash,
        CompositeDocuments {
            page_documents: result.page_documents,
            sub_results: result.sub_results,
            sub_result_missing_ids: result.sub_result_missing_ids,
            sub_result_removals: result.sub_result_removals,
        },
    ))
}

impl<'dq, Q> FromProof<Q> for CompositeDocuments
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let query: DriveDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        // The standard envelope carries the single MERGED proof, and
        // the proof alone is enough: the verifier bootstraps the page
        // from it via a subset pass and re-derives the rest.
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (_root_hash, composite) =
            verify_composite_documents_proof(&query, proof, mtd, platform_version, provider)?;

        // An empty page is a valid, proven "nothing here" — surface it
        // as Some(empty) rather than None so callers can tell it apart
        // from a missing object.
        Ok((Some(composite), mtd.clone(), proof.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::moderation::{
        encode_kept_fields, ContractDocumentRemoval, ContractModerationConfig, ContractModerators,
    };
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::data_contract::errors::DataContractError;
    use dpp::document::DocumentV0;
    use dpp::identifier::Identifier;
    use dpp::platform_value::platform_value;

    #[test]
    fn should_share_decoded_value_budget_across_page_subqueries_and_multiple_removal_groups() {
        let pv = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(pv)
            .expect("default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let post_type = DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "post",
            platform_value!({
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "array",
                        "maxItems": 32,
                        "items": { "type": "string", "maxLength": 16 },
                        "position": 0,
                    },
                },
                "additionalProperties": false,
                "moderatorAbilities": {
                    "delete": true,
                    "deleteKeepsFields": ["tags"],
                },
            }),
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            pv,
        )
        .expect("post type parses");

        let make_doc = |seed: u8, n: usize| {
            Document::V0(DocumentV0 {
                id: Identifier::new([seed; 32]),
                owner_id: Identifier::new([2; 32]),
                properties: BTreeMap::from([(
                    "tags".to_string(),
                    Value::Array((0..n).map(|i| Value::Text(format!("t{i}"))).collect()),
                )]),
                ..Default::default()
            })
        };

        // Page doc: 1 array + 4 tags = 5 values.
        // Sub-query 0 live doc: 1 array + 4 tags = 5 values, plus 1 removal keeping 1 + 9 = 10 values.
        // Sub-query 1 removal: keeping 1 + 9 = 10 values.
        // Total across the entire composite response = 30 values.
        let page_doc = make_doc(10, 4);
        let sub0_doc = make_doc(11, 4);
        let kept_fields =
            encode_kept_fields(&make_doc(12, 9), post_type.as_ref()).expect("encodes");
        let removal_entry = |seed: u8| ContractDocumentRemovalEntry {
            document_id: Identifier::new([seed; 32]),
            removal: ContractDocumentRemoval {
                kept_fields: kept_fields.clone(),
                ..Default::default()
            },
        };

        let composite = CompositeDocuments {
            page_documents: vec![page_doc],
            sub_results: vec![
                SubQueryResult::Documents(vec![sub0_doc]),
                SubQueryResult::Documents(vec![]),
            ],
            sub_result_missing_ids: vec![vec![], vec![]],
            sub_result_removals: vec![vec![removal_entry(20)], vec![removal_entry(21)]],
        };

        let mut exact = DecodedValueBudget::new(30);
        let decoded = composite
            .decode_sub_result_removal_kept_values_with_budget(
                |_| Ok(post_type.as_ref()),
                &mut exact,
            )
            .expect("combined page + subqueries + multiple removal groups fit exact budget");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].len(), 1);
        assert_eq!(decoded[1].len(), 1);
        assert_eq!(exact.remaining(), 0);

        let mut tight = DecodedValueBudget::new(29);
        let error = composite
            .decode_sub_result_removal_kept_values_with_budget(
                |_| Ok(post_type.as_ref()),
                &mut tight,
            )
            .expect_err("second subquery removal exceeding shared response budget must fail");
        assert!(
            matches!(
                error,
                ProtocolError::DataContractError(DataContractError::DecodedValueBudgetExceeded(_))
            ),
            "unexpected error: {error}"
        );
    }
}
