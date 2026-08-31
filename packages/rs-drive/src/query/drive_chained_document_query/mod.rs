//! Chained document queries: a provable semi-join.
//!
//! `SELECT * FROM post WHERE $id IN (SELECT postId FROM like WHERE
//! $ownerId = <me>)` — the INNER query runs against an indexOnly document
//! type and projects a `refersTo: permanentDocument` property (the JOIN
//! property); its proven values are reinjected as the OUTER query's
//! primary keys. Both halves are proven against the SAME state root —
//! grovedb proves against committed state only, so the server brackets
//! the pair with root-hash reads and retries if a block commit
//! interleaved (see [`execute_with_proofs_internal`]); the verifier
//! accepts only two proofs whose root hashes are equal (the surrounding
//! tenderdash composition then binds that root to the quorum-signed app
//! hash; see `rs-drive-proof-verifier`).
//!
//! [`execute_with_proofs_internal`]:
//!     DriveChainedDocumentQuery::execute_with_proofs_internal
//!
//! Soundness never rests on the server's join: the verifier re-derives
//! the outer query from the INNER proof's results ([`Self::join_values`]
//! → [`Self::derive_outer_query`], the same functions the server
//! executes), so a server cannot substitute, omit, or inject outer
//! documents. Because the join property's `refersTo` targets a
//! `permanentDocument` type (non-deletable, enforced at write time),
//! every proven join value MUST resolve to a document — a missing outer
//! document is an invalid proof, not an absence.
//!
//! Guardrails (v1): the inner query must resolve to an indexOnly index
//! that carries the join property (as terminal or prefix property, so
//! every synthesized projection provably carries its value); the join
//! edge must be a same-contract `refersTo: permanentDocument` whose
//! target is the outer type; the inner limit is required (it is what
//! bounds the outer fan-out); the outer half takes no clauses, no
//! limit and no cursor — it is purely the derived by-ids fetch, and
//! pagination lives on the inner query alone.

use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentTypeRef,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;

/// A chained document query: an inner indexOnly query whose proven join
/// values become the outer query's primary keys.
///
/// Construction contract: `outer_document_type` MUST be a document type
/// of `inner.contract` — build it via
/// [`DataContract::document_type_for_name`] on the same contract the
/// inner query was built from. [`Self::validate`] enforces everything
/// derivable from the types themselves.
///
/// [`DataContract::document_type_for_name`]:
///     dpp::data_contract::accessors::v0::DataContractV0Getters::document_type_for_name
#[derive(Debug, Clone)]
pub struct DriveChainedDocumentQuery<'a> {
    /// The inner query. Must target an indexOnly document type and
    /// resolve to an index carrying [`Self::join_property`].
    pub inner: DriveDocumentQuery<'a>,
    /// The inner property whose values feed the outer query's `$id`s.
    /// Must carry a same-contract `refersTo: permanentDocument`
    /// declaration targeting [`Self::outer_document_type`].
    pub join_property: String,
    /// The outer document type — the `refersTo` target.
    pub outer_document_type: DocumentTypeRef<'a>,
}

/// The materialized result of a chained query, in inner-proof order.
#[derive(Debug, Default)]
pub struct ChainedDocumentsResult {
    /// The inner projections (synthesized indexOnly documents), exactly
    /// as the inner query alone would return them — the caller reads its
    /// pagination cursor (the last join value) from here.
    pub inner_documents: Vec<Document>,
    /// The referenced outer documents, ordered by FIRST APPEARANCE of
    /// their id in `inner_documents` (deduplicated).
    pub outer_documents: Vec<Document>,
}

/// The two grovedb proofs of a chained query. Soundness requires both
/// to verify to the SAME root hash; the server guarantees it by
/// bracketing the pair with root-hash reads (grovedb proves against
/// committed state only, so a transaction cannot pin the pair).
#[derive(Debug)]
pub struct ChainedProofBundle {
    /// Proof of the inner query (the standard proof the inner
    /// [`DriveDocumentQuery`] alone would produce).
    pub inner_proof: Vec<u8>,
    /// Proof of the derived outer by-ids query. `None` if and only if
    /// the inner page is empty (there is nothing to derive).
    pub outer_proof: Option<Vec<u8>>,
}

impl<'a> DriveChainedDocumentQuery<'a> {
    /// Validates the chained shape. Called by the server before
    /// executing and by the verifier before verifying, so an invalid
    /// spec fails identically on both sides.
    pub fn validate(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        let unsupported = |message: String| Error::Query(QuerySyntaxError::Unsupported(message));

        if !self.inner.document_type.index_only() {
            return Err(unsupported(
                "chained document queries require an indexOnly inner document type: only \
                 indexOnly projections prove their values positionally"
                    .to_string(),
            ));
        }
        if self.outer_document_type.index_only() {
            return Err(unsupported(
                "the outer document type of a chained query cannot be indexOnly: outer \
                 documents are fetched by id from primary storage, which indexOnly types \
                 do not have"
                    .to_string(),
            ));
        }
        if self.inner.limit.is_none() {
            return Err(unsupported(
                "chained document queries require an explicit limit on the inner query: \
                 the inner page size is what bounds the derived outer query"
                    .to_string(),
            ));
        }
        if self.inner.offset.is_some() {
            return Err(unsupported(
                "chained document queries do not support an inner offset; paginate with a \
                 range clause on the join property"
                    .to_string(),
            ));
        }

        // The join property must be a same-contract permanentDocument
        // reference targeting the outer type. `refersTo` writes are
        // existence-validated and permanentDocument targets can never be
        // deleted, so every proven join value MUST resolve — which is
        // what lets the verifier treat a missing outer document as an
        // invalid proof instead of needing absence proofs.
        let Some(join_document_property) = self
            .inner
            .document_type
            .flattened_properties()
            .get(self.join_property.as_str())
        else {
            return Err(unsupported(format!(
                "chained query join property \"{}\" does not name a property of inner \
                 document type \"{}\"",
                self.join_property,
                self.inner.document_type.name(),
            )));
        };
        match &join_document_property.property_type {
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id,
                    document_type_name,
                    ..
                },
            ) => {
                if let Some(referenced_contract_id) = contract_id {
                    if *referenced_contract_id != self.inner.contract.id() {
                        return Err(unsupported(
                            "chained document queries support same-contract joins only: \
                             the join property's refersTo names another contract"
                                .to_string(),
                        ));
                    }
                }
                if document_type_name != self.outer_document_type.name() {
                    return Err(unsupported(format!(
                        "chained query outer document type \"{}\" does not match the join \
                         property's refersTo target \"{}\"",
                        self.outer_document_type.name(),
                        document_type_name,
                    )));
                }
            }
            _ => {
                return Err(unsupported(format!(
                    "chained query join property \"{}\" must carry a `refersTo: \
                     permanentDocument` declaration: only a permanent-document reference \
                     guarantees every proven join value resolves to an outer document",
                    self.join_property,
                )));
            }
        }

        // The resolved index must carry the join property, so every
        // synthesized inner projection provably carries its value.
        let index = self.inner.index_only_query_index(platform_version)?;
        let index_carries_join_property = index.terminal.as_deref()
            == Some(self.join_property.as_str())
            || index
                .properties
                .iter()
                .any(|property| property.name == self.join_property);
        if !index_carries_join_property {
            return Err(unsupported(format!(
                "the inner query resolves to index \"{}\", which does not carry the join \
                 property \"{}\"; constrain the query so an index carrying it serves it",
                index.name, self.join_property,
            )));
        }

        Ok(())
    }

    /// Extracts the join values from the inner documents in their proof
    /// order, deduplicated to first appearance. ONE extraction both the
    /// server and the verifier run — the single-builder rule that keeps
    /// the derived outer query identical on both sides.
    pub fn join_values(&self, inner_documents: &[Document]) -> Result<Vec<Identifier>, Error> {
        let mut seen: std::collections::BTreeSet<Identifier> = std::collections::BTreeSet::new();
        let mut join_values = Vec::with_capacity(inner_documents.len());
        for document in inner_documents {
            let value = document
                .properties()
                .get(self.join_property.as_str())
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "an inner projection is missing the join property: validate() \
                     guarantees the resolved index carries it",
                )))?;
            let identifier = value.to_identifier().map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "a chained join property must decode as an identifier: the parser \
                     only admits identifier-typed refersTo properties",
                ))
            })?;
            if seen.insert(identifier) {
                join_values.push(identifier);
            }
        }
        Ok(join_values)
    }

    /// The derived outer query: a pure by-ids fetch of the join values
    /// from the outer type's primary storage. No clauses, no limit, no
    /// cursor — completeness is set-equality against `join_values`,
    /// checked by the verifier.
    pub fn derive_outer_query(&self, join_values: &[Identifier]) -> DriveDocumentQuery<'a> {
        // Canonical value order: byte-ascending. Grove sorts query keys
        // internally either way; sorting here keeps the built query —
        // and therefore the proof — byte-identical between the server
        // and a verifier that extracted the ids in any order.
        let mut ids: Vec<Identifier> = join_values.to_vec();
        ids.sort();
        DriveDocumentQuery {
            contract: self.inner.contract,
            document_type: self.outer_document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: Some(WhereClause {
                    field: dpp::document::property_names::ID.to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(
                        ids.into_iter()
                            .map(|id| Value::Identifier(id.to_buffer()))
                            .collect(),
                    ),
                }),
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: Default::default(),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: Vec::new(),
        }
    }

    /// Reorders the outer documents (returned in key order by the by-ids
    /// query) into first-appearance join order, and enforces EXACT set
    /// equality between the proven outer ids and the derived join
    /// values — both directions. Shared by the server (where a mismatch
    /// is corrupted state: permanentDocument references cannot dangle)
    /// and the verifier (where it is an invalid proof).
    pub fn assemble_outer_documents(
        &self,
        join_values: &[Identifier],
        outer_documents: Vec<Document>,
    ) -> Result<Vec<Document>, Error> {
        use std::collections::BTreeMap;
        let mut by_id: BTreeMap<Identifier, Document> = BTreeMap::new();
        for document in outer_documents {
            let id = document.id();
            if by_id.insert(id, document).is_some() {
                return Err(Error::Proof(
                    crate::error::proof::ProofError::CorruptedProof(format!(
                        "chained outer results carry document {} twice",
                        id
                    )),
                ));
            }
        }
        let mut ordered = Vec::with_capacity(join_values.len());
        for join_value in join_values {
            let document = by_id.remove(join_value).ok_or_else(|| {
                Error::Proof(crate::error::proof::ProofError::CorruptedProof(format!(
                    "chained outer results are missing referenced document {}: a \
                     permanentDocument reference cannot dangle, so the outer half does \
                     not prove the derived query",
                    join_value
                )))
            })?;
            ordered.push(document);
        }
        if let Some((extra_id, _)) = by_id.into_iter().next() {
            return Err(Error::Proof(
                crate::error::proof::ProofError::CorruptedProof(format!(
                    "chained outer results carry document {} that no proven join value \
                     references",
                    extra_id
                )),
            ));
        }
        Ok(ordered)
    }
}

#[cfg(feature = "server")]
impl DriveChainedDocumentQuery<'_> {
    /// Executes the chained query without proofs.
    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &crate::drive::Drive,
        transaction: grovedb::TransactionArg,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ChainedDocumentsResult, Error> {
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;

        self.validate(platform_version)?;

        let (inner_documents, _skipped) =
            self.inner.execute_index_only_documents_no_proof_internal(
                drive,
                transaction,
                drive_operations,
                platform_version,
            )?;
        let join_values = self.join_values(&inner_documents)?;
        if join_values.is_empty() {
            return Ok(ChainedDocumentsResult {
                inner_documents,
                outer_documents: Vec::new(),
            });
        }

        let outer_query = self.derive_outer_query(&join_values);
        let (serialized_outer, _outer_skipped) = outer_query
            .execute_raw_results_no_proof_internal(
                drive,
                transaction,
                drive_operations,
                platform_version,
            )?;
        let outer_documents = serialized_outer
            .into_iter()
            .map(|serialized| {
                Document::from_bytes(
                    serialized.as_slice(),
                    self.outer_document_type,
                    platform_version,
                )
                .map_err(|e| Error::Protocol(Box::new(e)))
            })
            .collect::<Result<Vec<Document>, Error>>()?;
        let outer_documents = self.assemble_outer_documents(&join_values, outer_documents)?;

        Ok(ChainedDocumentsResult {
            inner_documents,
            outer_documents,
        })
    }

    /// Executes the chained query AND generates both proofs.
    ///
    /// Same-root contract: grovedb generates proofs against COMMITTED
    /// state only (`prove_query` rejects transactions), so the pair
    /// cannot be pinned to a snapshot. Instead the whole sequence —
    /// materialize, inner proof, outer proof — is BRACKETED by root-hash
    /// reads: if the root moved, a block commit interleaved and the
    /// halves may describe different states, so the attempt is discarded
    /// and retried. On a quiet root the bracket proves both proofs
    /// commit to that root — exactly what the verifier's root-equality
    /// check demands.
    pub(crate) fn execute_with_proofs_internal(
        &self,
        drive: &crate::drive::Drive,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(ChainedProofBundle, ChainedDocumentsResult), Error> {
        // Block commits are seconds apart while an attempt is
        // milliseconds, so a bracket collision is rare and two in a row
        // vanishingly so; three attempts is generosity, not need.
        const MAX_ATTEMPTS: usize = 3;
        for _ in 0..MAX_ATTEMPTS {
            let root_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()?;

            // Materializes both halves (validating on the way in) — the
            // join values drive the outer proof's query, and the
            // documents ride back for callers that want proof + results
            // in one pass.
            let result =
                self.execute_no_proof_internal(drive, None, drive_operations, platform_version)?;

            let inner_proof = self.inner.clone().execute_with_proof_internal(
                drive,
                None,
                drive_operations,
                platform_version,
            )?;

            let join_values = self.join_values(&result.inner_documents)?;
            let outer_proof = if join_values.is_empty() {
                None
            } else {
                Some(
                    self.derive_outer_query(&join_values)
                        .execute_with_proof_internal(
                            drive,
                            None,
                            drive_operations,
                            platform_version,
                        )?,
                )
            };

            let root_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()?;
            if root_before != root_after {
                continue;
            }

            return Ok((
                ChainedProofBundle {
                    inner_proof,
                    outer_proof,
                },
                result,
            ));
        }
        Err(Error::Drive(DriveError::NotSupported(
            "chained proof generation raced a block commit on every attempt; \
             transient — retry the request",
        )))
    }
}
