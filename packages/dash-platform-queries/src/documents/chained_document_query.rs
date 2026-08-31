//! Chained document queries — the client half of the provable semi-join:
//! `SELECT * FROM <outer> WHERE $id IN (SELECT <join_property> FROM
//! <inner> WHERE …)`.
//!
//! The inner half is an ordinary [`DocumentQuery`] against an indexOnly
//! document type; the request carries no outer clauses at all — the
//! server derives the outer by-ids query from the inner results, and the
//! verifier re-derives it from the PROVEN inner results, so the join can
//! never be steered by the responding node. See
//! `drive::query::drive_chained_document_query` for the trust model.

use crate::documents::document_query::DocumentQuery;
use crate::error::Error;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::ChainedJoin;
use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_documents_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use dpp::ProtocolError;
use drive::query::drive_chained_document_query::DriveChainedDocumentQuery;
use drive::query::DriveDocumentQuery;
use drive_proof_verifier::{
    verify_chained_documents_tenderdash_proof, ChainedDocuments, FromProof,
};

/// A chained document query: the inner [`DocumentQuery`] plus the join
/// edge. The outer half has no clauses by design — it is derived.
///
/// The inner query MUST carry an explicit non-zero limit (it bounds the
/// derived outer query; there is no server-default sentinel on this
/// surface) and must resolve, server-side, to an indexOnly index
/// carrying `join_property`.
#[derive(Debug, Clone, PartialEq, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct ChainedDocumentQuery {
    /// The inner query (the subselect).
    pub inner: DocumentQuery,
    /// The inner property whose proven values become the outer `$id`s.
    /// Must carry a same-contract `refersTo: permanentDocument`
    /// declaration targeting `outer_document_type_name`.
    pub join_property: String,
    /// The outer (joined) document type — the `refersTo` target.
    pub outer_document_type_name: String,
}

impl ChainedDocumentQuery {
    /// A chained query joining `inner`'s `join_property` values onto
    /// documents of `outer_document_type_name`.
    pub fn new(
        inner: DocumentQuery,
        join_property: impl Into<String>,
        outer_document_type_name: impl Into<String>,
    ) -> Self {
        Self {
            inner,
            join_property: join_property.into(),
            outer_document_type_name: outer_document_type_name.into(),
        }
    }
}

impl TryFromPlatformVersioned<ChainedDocumentQuery> for GetDocumentsRequest {
    type Error = Error;

    fn try_from_platform_versioned(
        value: ChainedDocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let ChainedDocumentQuery {
            inner,
            join_property,
            outer_document_type_name,
        } = value;

        if inner.limit == 0 {
            return Err(Error::Config(
                "a chained document query requires an explicit non-zero inner limit: it \
                 bounds the derived outer query, so there is no server-default sentinel"
                    .to_string(),
            ));
        }
        if !inner.time_range_clauses.is_empty()
            || inner.start.is_some()
            || inner.offset.is_some()
            || !inner.group_by.is_empty()
            || !inner.having.is_empty()
        {
            return Err(Error::Config(
                "a chained inner query supports where/order_by/limit only: no time-range \
                 selections, cursors, offsets, group_by, or having (paginate with a range \
                 clause on the join property)"
                    .to_string(),
            ));
        }

        // The chained surface rides the typed V1 wire: encode the
        // inner query through the standard versioned encoder, then
        // attach the join spec. A network still on the V0 (CBOR) wire
        // cannot express the field, so refuse rather than silently
        // sending a plain documents query.
        let mut request =
            GetDocumentsRequest::try_from_platform_versioned(inner, platform_version)?;
        match request.version.as_mut() {
            Some(RequestVersion::V1(v1)) => {
                v1.chained = Some(ChainedJoin {
                    join_property,
                    outer_document_type: outer_document_type_name,
                });
            }
            _ => {
                return Err(Error::Config(
                    "chained document queries require the V1 documents wire (Platform \
                     v3.1+); this network's protocol version encodes V0"
                        .to_string(),
                ));
            }
        }
        Ok(request)
    }
}

impl<'a> TryFrom<&'a ChainedDocumentQuery> for DriveChainedDocumentQuery<'a> {
    type Error = Error;

    fn try_from(request: &'a ChainedDocumentQuery) -> Result<Self, Self::Error> {
        let inner: DriveDocumentQuery<'a> = (&request.inner).try_into()?;
        let outer_document_type = request
            .inner
            .data_contract
            .document_type_for_name(&request.outer_document_type_name)
            .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))?;
        Ok(DriveChainedDocumentQuery {
            inner,
            join_property: request.join_property.clone(),
            outer_document_type,
        })
    }
}

impl FromProof<ChainedDocumentQuery> for ChainedDocuments {
    type Request = ChainedDocumentQuery;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let query: DriveChainedDocumentQuery = (&request).try_into().map_err(|e: Error| {
            drive_proof_verifier::Error::RequestError {
                error: e.to_string(),
            }
        })?;

        // The standard envelope carries the single MERGED proof; the
        // untrusted join-value hint rides beside the result oneof.
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;
        let hint: Vec<dpp::prelude::Identifier> = match &response.version {
            Some(ResponseVersion::V1(v1)) => v1
                .proven_join_values
                .iter()
                .map(|bytes| {
                    dpp::prelude::Identifier::from_bytes(bytes).map_err(|_| {
                        drive_proof_verifier::Error::ResponseDecodeError {
                            error: "proven_join_values entries must be 32-byte identifiers"
                                .to_string(),
                        }
                    })
                })
                .collect::<Result<_, _>>()?,
            Some(ResponseVersion::V0(_)) => {
                return Err(drive_proof_verifier::Error::ResponseDecodeError {
                    error: "chained results are a V1-only response shape; got a V0 \
                            getDocuments response"
                        .to_string(),
                })
            }
            None => return Err(drive_proof_verifier::Error::EmptyVersion),
        };

        let (_root_hash, chained) = verify_chained_documents_tenderdash_proof(
            &query,
            proof,
            &hint,
            mtd,
            platform_version,
            provider,
        )?;

        // An empty inner page is a valid, proven "you have nothing
        // here" — surface it as Some(empty) rather than None so callers
        // can tell it apart from a missing object.
        Ok((Some(chained), mtd.clone(), proof.clone()))
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the chained client surface: the V1
    //! request-wire encoding (typed clauses, no CBOR), the
    //! unsupported-feature rejections, and the rich→drive conversion +
    //! shared shape validation against the yappr-likes fixture. Proof
    //! verification is exercised end-to-end in rs-drive's
    //! `chained_query_e2e_tests` and rs-drive-abci's v1 chained
    //! dispatch tests, where a populated Drive exists.

    use super::*;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract;
    use drive::query::{WhereClause, WhereOperator};
    use std::sync::Arc;

    const YAPPR_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json";
    const OWNER: [u8; 32] = [0x11; 32];

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn yappr_contract() -> Arc<DataContract> {
        Arc::new(
            json_document_to_contract(YAPPR_CONTRACT_PATH, false, platform_version())
                .expect("expected to parse the yappr-likes contract"),
        )
    }

    fn posts_i_liked(limit: u32) -> ChainedDocumentQuery {
        let inner = DocumentQuery::new(yappr_contract(), "like")
            .expect("like doctype exists")
            .with_where(WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER),
            })
            .with_limit(limit);
        ChainedDocumentQuery::new(inner, "postId", "post")
    }

    #[test]
    fn encodes_the_v1_wire_shape() {
        let request =
            GetDocumentsRequest::try_from_platform_versioned(posts_i_liked(10), platform_version())
                .expect("encodes");
        let Some(RequestVersion::V1(v1)) = request.version else {
            panic!("expected a V1 request");
        };
        assert_eq!(v1.document_type, "like");
        assert_eq!(v1.limit, Some(10));
        assert!(v1.prove, "chained fetch always proves");
        assert!(v1.order_by.is_empty());
        // Typed clauses on the wire — no CBOR anywhere on this surface.
        assert_eq!(v1.where_clauses.len(), 1);
        assert_eq!(v1.where_clauses[0].field, "$ownerId");
        let chained = v1.chained.expect("the join spec rides the request");
        assert_eq!(chained.join_property, "postId");
        assert_eq!(chained.outer_document_type, "post");
    }

    #[test]
    fn requires_an_inner_limit() {
        let refused =
            GetDocumentsRequest::try_from_platform_versioned(posts_i_liked(0), platform_version());
        assert!(
            matches!(refused, Err(Error::Config(_))),
            "a zero inner limit must be refused, got {refused:?}"
        );
    }

    #[test]
    fn refuses_unsupported_inner_features() {
        let mut query = posts_i_liked(10);
        query.inner.group_by = vec!["hashtag".to_string()];
        let refused = GetDocumentsRequest::try_from_platform_versioned(query, platform_version());
        assert!(
            matches!(refused, Err(Error::Config(_))),
            "an inner group_by must be refused, got {refused:?}"
        );
    }

    #[test]
    fn converts_to_a_valid_drive_query() {
        let query = posts_i_liked(10);
        let drive_query: DriveChainedDocumentQuery =
            (&query).try_into().expect("converts to a drive query");
        drive_query
            .validate(platform_version())
            .expect("the byLiker shape validates");
        assert_eq!(drive_query.join_property, "postId");
        assert_eq!(drive_query.inner.limit, Some(10));
    }

    #[test]
    fn conversion_surfaces_shape_errors() {
        let query = ChainedDocumentQuery::new(
            DocumentQuery::new(yappr_contract(), "like")
                .expect("like doctype exists")
                .with_limit(10),
            "hashtag",
            "post",
        );
        let drive_query: DriveChainedDocumentQuery =
            (&query).try_into().expect("conversion itself succeeds");
        let refused = drive_query.validate(platform_version());
        assert!(
            refused.is_err(),
            "a non-refersTo join property must fail validation"
        );
    }
}
