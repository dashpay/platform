//! Verified **chained document** (provable semi-join) results.
//!
//! A chained query is `SELECT * FROM <outer> WHERE $id IN (SELECT
//! <join_property> FROM <inner> WHERE …)` answered as ONE merged
//! grovedb proof: the limited inner indexOnly page and the outer
//! by-ids fetch derived from its values, merged by the server (the
//! inner limit a per-instance cap on its branch). The
//! verifier ([`DriveDocumentQuery::verify_chained_documents_proof`])
//! reconstructs the merged query from the response's UNTRUSTED
//! join-value hint, verifies in one pass, and requires the proven
//! outer documents to match the PROVEN inner join values. For a
//! `refersTo: permanentDocument` join property the match is exact: a
//! missing referenced document is an invalid proof, since such a target
//! cannot dangle. For a `refersTo: deletableDocument` join property a
//! join value whose document is proven absent (deleted after the inner
//! document was written) has no outer document; the proof must still
//! show every derived `$id` present or absent, so an existing document
//! cannot be passed off as deleted. This module's
//! [`FromProof`] impl composes that with the tenderdash signature
//! binding of the single root.
//!
//! There is deliberately **no unproven decoder with verification
//! semantics** here: an unproven chained response is free to fabricate
//! the join entirely (substitute, omit, inject), which is precisely
//! what the surface exists to prevent. [`ChainedDocuments`] can still
//! be built from a trusted node's unproven wire by the SDK if it
//! chooses, but the canonical path proves.

use crate::verify::{document_query_proof_bytes, verify_tenderdash_proof, MapProvedBudgetError};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::document_type::{DecodedValueBudget, DocumentTypeRef};
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::contract::moderation::types::ContractDocumentRemovalEntry;
use drive::query::DriveDocumentQuery;
use drive::verify::RootHash;
use std::collections::BTreeMap;

/// The verified result of a chained document query, both halves in
/// inner-proof order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChainedDocuments {
    /// The inner projections (synthesized indexOnly documents), exactly
    /// as the inner query alone would return them. The last one's join
    /// property carries the pagination cursor.
    pub inner_documents: Vec<Document>,
    /// The joined outer documents, ordered by first appearance of their
    /// id among the inner projections (deduplicated). Under a
    /// `deletableDocument` join property a deleted target has no entry
    /// here, so this can be shorter than the distinct join values; match
    /// the halves by id, not by position.
    pub outer_documents: Vec<Document>,
    /// The join values that have NO outer document, in first-appearance
    /// order: each one a PROVEN absence (the merged proof covers every
    /// derived `$id`). Always empty under a `permanentDocument` join
    /// property, where a missing document is an invalid proof, and under a
    /// `moderatedDocument` one, which reports its missing documents in
    /// [`Self::removed_outer_documents`].
    pub missing_outer_ids: Vec<dpp::identifier::Identifier>,
    /// The join values whose outer document the contract's moderators
    /// removed, each with its PROVEN removal record, in first-appearance
    /// order. Only a `moderatedDocument` join property reports any; there a
    /// join value with neither a document nor a record is an invalid proof.
    pub removed_outer_documents: Vec<ContractDocumentRemovalEntry>,
}

impl ChainedDocuments {
    /// Decodes the kept field values of every removed outer document under
    /// `outer_document_type`, enforcing a decoded-value budget proportional to
    /// each removal record's encoded `kept_fields` length.
    pub fn decode_removed_outer_kept_values(
        &self,
        outer_document_type: DocumentTypeRef,
    ) -> Result<Vec<BTreeMap<String, Value>>, ProtocolError> {
        self.removed_outer_documents
            .iter()
            .map(|entry| {
                entry
                    .removal
                    .kept_values_with_default_budget(outer_document_type)
            })
            .collect()
    }

    /// Like [`Self::decode_removed_outer_kept_values`], charging every retained
    /// document and decoded kept field against a shared `budget`.
    pub fn decode_removed_outer_kept_values_with_budget(
        &self,
        outer_document_type: DocumentTypeRef,
        budget: &mut DecodedValueBudget,
    ) -> Result<Vec<BTreeMap<String, Value>>, ProtocolError> {
        budget
            .charge_documents(&self.inner_documents)
            .map_err(ProtocolError::DataContractError)?;
        budget
            .charge_documents(&self.outer_documents)
            .map_err(ProtocolError::DataContractError)?;
        self.removed_outer_documents
            .iter()
            .map(|entry| {
                entry
                    .removal
                    .kept_values_with_budget(outer_document_type, budget)
            })
            .collect()
    }
}

/// Verify a chained query's single merged proof and bind its root hash
/// to the quorum signature.
///
/// The merk-level composition (bootstrap subset pass on the inner
/// query, merged-query re-derivation, authoritative full verification,
/// exact set equality against the PROVEN join values) lives in rs-drive's
/// [`DriveDocumentQuery::verify_chained_documents_proof`]; this
/// wrapper adds the [`verify_tenderdash_proof`] binding — the root hash
/// the proof commits to is only an attested fact once it is tied to the
/// quorum-signed app hash, and this function exists so the composition
/// can never be skipped by accident.
///
/// The query is the chained shape: the inner [`DriveDocumentQuery`]
/// carrying its single by-id join in `sub_queries` (see
/// `DriveDocumentQuery::with_by_id_join`).
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_chained_documents_proof(
    query: &DriveDocumentQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, ChainedDocuments), Error> {
    let (root_hash, result) = query
        .verify_chained_documents_proof(
            document_query_proof_bytes(proof, mtd, platform_version)?,
            platform_version,
        )
        .map_proved_budget_error(proof, mtd, provider)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((
        root_hash,
        ChainedDocuments {
            inner_documents: result.inner_documents,
            outer_documents: result.outer_documents,
            missing_outer_ids: result.missing_outer_ids,
            removed_outer_documents: result.removed_outer_documents,
        },
    ))
}

impl<'dq, Q> FromProof<Q> for ChainedDocuments
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
        // the proof alone is enough: the verifier bootstraps the join
        // values from it via a subset pass.
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (_root_hash, chained) =
            verify_chained_documents_proof(&query, proof, mtd, platform_version, provider)?;

        // An empty inner page is a valid, proven "you have nothing
        // here" — surface it as Some(empty) rather than None so callers
        // can tell it apart from a missing object.
        Ok((Some(chained), mtd.clone(), proof.clone()))
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
    fn should_share_decoded_value_budget_between_live_outer_documents_and_removed_kept_fields() {
        let pv = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(pv)
            .expect("default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let outer_type = DocumentType::try_from_schema(
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

        // Live outer doc holds 1 array + 10 strings = 11 decoded values.
        let live_outer = make_doc(3, 10);
        // Removed outer doc keeps 1 array + 10 strings = 11 decoded values.
        let removed_doc = make_doc(4, 10);
        let kept_fields = encode_kept_fields(&removed_doc, outer_type.as_ref()).expect("encodes");

        let chained = ChainedDocuments {
            inner_documents: vec![],
            outer_documents: vec![live_outer],
            missing_outer_ids: vec![],
            removed_outer_documents: vec![ContractDocumentRemovalEntry {
                document_id: Identifier::new([4; 32]),
                removal: ContractDocumentRemoval {
                    kept_fields,
                    ..Default::default()
                },
            }],
        };

        // Combined total is 22 decoded values: exact budget succeeds, 1 short fails.
        let mut exact = DecodedValueBudget::new(22);
        let decoded = chained
            .decode_removed_outer_kept_values_with_budget(outer_type.as_ref(), &mut exact)
            .expect("combined live + removed fits exact budget");
        assert_eq!(decoded.len(), 1);
        assert_eq!(exact.remaining(), 0);

        let mut tight = DecodedValueBudget::new(21);
        let error = chained
            .decode_removed_outer_kept_values_with_budget(outer_type.as_ref(), &mut tight)
            .expect_err("live + removed exceeding shared budget must fail");
        assert!(
            matches!(
                error,
                ProtocolError::DataContractError(DataContractError::DecodedValueBudgetExceeded(_))
            ),
            "unexpected error: {error}"
        );
    }
}
