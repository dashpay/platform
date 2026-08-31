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
use dapi_grpc::platform::v0::get_chained_documents_request::GetChainedDocumentsRequestV0;
use dapi_grpc::platform::v0::get_chained_documents_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetChainedDocumentsRequest, GetChainedDocumentsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Value;
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

impl TryFromPlatformVersioned<ChainedDocumentQuery> for GetChainedDocumentsRequest {
    type Error = Error;

    fn try_from_platform_versioned(
        value: ChainedDocumentQuery,
        _platform_version: &PlatformVersion,
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

        // The chained wire carries the inner clauses in the same CBOR
        // encoding as `GetDocumentsRequestV0.where` / `.order_by`.
        let where_bytes = if inner.where_clauses.is_empty() {
            Vec::new()
        } else {
            let where_value =
                Value::Array(inner.where_clauses.into_iter().map(Value::from).collect());
            where_value.to_cbor_buffer().map_err(|e| {
                Error::Protocol(ProtocolError::EncodingError(format!(
                    "failed to CBOR-encode chained inner where clauses: {e}"
                )))
            })?
        };
        let order_by_bytes = if inner.order_by_clauses.is_empty() {
            Vec::new()
        } else {
            let order_value = Value::Array(
                inner
                    .order_by_clauses
                    .into_iter()
                    .map(Value::from)
                    .collect(),
            );
            order_value.to_cbor_buffer().map_err(|e| {
                Error::Protocol(ProtocolError::EncodingError(format!(
                    "failed to CBOR-encode chained inner order_by clauses: {e}"
                )))
            })?
        };

        Ok(GetChainedDocumentsRequest {
            version: Some(
                dapi_grpc::platform::v0::get_chained_documents_request::Version::V0(
                    GetChainedDocumentsRequestV0 {
                        data_contract_id: inner.data_contract.id().to_vec(),
                        inner_document_type: inner.document_type_name,
                        inner_where: where_bytes,
                        inner_order_by: order_by_bytes,
                        inner_limit: inner.limit,
                        join_property,
                        outer_document_type: outer_document_type_name,
                        // Chained fetch always proves — the whole point
                        // of the surface is the verifiable composition.
                        prove: true,
                    },
                ),
            ),
        })
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
    type Response = GetChainedDocumentsResponse;

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

        // The standard envelope carries the INNER proof (and the
        // signature fields); the outer grovedb proof rides beside the
        // result oneof.
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;
        let outer_grovedb_proof = match &response.version {
            Some(ResponseVersion::V0(v0)) => v0.outer_grovedb_proof.as_slice(),
            None => return Err(drive_proof_verifier::Error::EmptyVersion),
        };

        let (_root_hash, chained) = verify_chained_documents_tenderdash_proof(
            &query,
            proof,
            outer_grovedb_proof,
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
    //! Offline tests for the chained client surface: the request→wire
    //! encoding, the unsupported-feature rejections, and the
    //! rich→drive conversion + shared shape validation against the
    //! yappr-likes fixture. Proof verification is exercised end-to-end
    //! in rs-drive's `chained_query_e2e_tests` and rs-drive-abci's
    //! `chained_document_query` handler tests, where a populated Drive
    //! exists.

    use super::*;
    use dapi_grpc::platform::v0::get_chained_documents_request::Version as RequestVersion;
    use dpp::data_contract::DataContract;
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
    fn encodes_the_v0_wire_shape() {
        let request = GetChainedDocumentsRequest::try_from_platform_versioned(
            posts_i_liked(10),
            platform_version(),
        )
        .expect("encodes");
        let Some(RequestVersion::V0(v0)) = request.version else {
            panic!("expected a V0 request");
        };
        assert_eq!(v0.inner_document_type, "like");
        assert_eq!(v0.join_property, "postId");
        assert_eq!(v0.outer_document_type, "post");
        assert_eq!(v0.inner_limit, 10);
        assert!(v0.prove, "chained fetch always proves");
        assert!(v0.inner_order_by.is_empty());
        // The where bytes are the same CBOR the server decodes for
        // GetDocumentsRequestV0.where: an array of clause arrays —
        // byte-identical to encoding the clause list directly.
        let expected = Value::Array(vec![Value::from(WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(OWNER),
        })])
        .to_cbor_buffer()
        .expect("encode expected clauses");
        assert_eq!(v0.inner_where, expected);
    }

    #[test]
    fn requires_an_inner_limit() {
        let refused = GetChainedDocumentsRequest::try_from_platform_versioned(
            posts_i_liked(0),
            platform_version(),
        );
        assert!(
            matches!(refused, Err(Error::Config(_))),
            "a zero inner limit must be refused, got {refused:?}"
        );
    }

    #[test]
    fn refuses_unsupported_inner_features() {
        let mut query = posts_i_liked(10);
        query.inner.group_by = vec!["hashtag".to_string()];
        let refused =
            GetChainedDocumentsRequest::try_from_platform_versioned(query, platform_version());
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
