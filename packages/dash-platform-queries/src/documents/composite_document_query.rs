//! Composite document queries — the client half of "a page plus the
//! sub-queries derived from it", answered as ONE merged proof.
//!
//! Attach sub-queries directly to [`DocumentQuery`] with an explicit
//! page limit, then fetch the result as [`CompositeDocuments`].
//! Each [`CompositeSubQuery`] is a by-id join, an indexed lookup, a
//! grouped count, or an independent sibling, whose `IN` clause the
//! server derives from the proven page (or an earlier documents
//! sub-query) — the request never names the derived values, so the
//! responding node cannot steer them. The verifier bootstraps the page
//! from the merged proof and re-derives every sub-query with the same
//! builders, so a substituted, omitted or injected sub-result fails
//! verification. See `drive::query::composite_document_query`
//! for the shape rules and the trust model.

use crate::documents::document_query::{
    order_clause_to_proto, where_clause_to_proto, DocumentQuery,
};
use crate::error::Error;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    sub_query, SubQuery as ProtoSubQuery,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::error::query::QuerySyntaxError;
use drive::query::{
    BindingSource, DriveDocumentQuery, DriveSubQuery, OrderClause, SelectProjection,
    SubQueryBinding, SubQueryKind, WhereClause, MAX_SUB_QUERIES,
};
use drive_proof_verifier::{
    verify_composite_documents_tenderdash_proof, CompositeDocuments, FromProof,
};
use std::sync::Arc;

/// Whose proven documents a sub-query's values are read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum CompositeBindingSource {
    /// The page.
    Page,
    /// An earlier documents sub-query, by its position in
    /// [`DocumentQuery::sub_queries`].
    SubQuery(usize),
}

/// The derived clause of a sub-query: `<field> IN <values>`, where the
/// values are read off the source's proven documents.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct CompositeBinding {
    /// Whose documents supply the values.
    pub source: CompositeBindingSource,
    /// The source property read off each document: `$id`, `$ownerId`,
    /// or an identifier-typed property (dotted paths reach nested
    /// properties). Documents without it contribute nothing.
    pub source_property: String,
    /// The sub-query field receiving the `IN` clause. `$id` makes the
    /// sub-query a by-id JOIN (the source property must then declare
    /// `refersTo: permanentDocument` targeting the sub-query's type);
    /// otherwise `$ownerId` or an indexed property (a LOOKUP).
    pub field: String,
}

/// What a sub-query returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum CompositeSubQueryKind {
    /// The matching documents.
    Documents,
    /// One count per derived value, read from the countable index
    /// covering the fixed clauses plus the bound field.
    Count,
}

/// One sub-query of a composite request.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct CompositeSubQuery {
    /// The contract the sub-query targets — the page's, or any other
    /// (profiles keyed by owner, names keyed by identity).
    pub data_contract: Arc<DataContract>,
    /// The document type queried.
    pub document_type_name: String,
    /// Documents or counts.
    pub kind: CompositeSubQueryKind,
    /// The FIXED clauses — everything but the derived `IN`, which must
    /// not be named here.
    pub where_clauses: Vec<WhereClause>,
    /// Ordering (documents only). Every component of the merged proof
    /// walks in the page's direction: a bound field missing from here is
    /// appended in that direction by the node and the verifier alike, and
    /// an ordering that disagrees with the page's direction is refused
    /// (turning a limited lookup around would change the rows it returns).
    pub order_by_clauses: Vec<OrderClause>,
    /// Required for a documents lookup on a non-unique index: it caps the
    /// rows the lookup returns in total, in walk order, like an ordinary
    /// `IN` query's limit. Forbidden for a lookup already bounded by its
    /// values, a by-id join and a count.
    pub limit: Option<u32>,
    /// The derived clause, or `None` for a sibling: an independent
    /// documents query proven under the same root.
    pub binding: Option<CompositeBinding>,
}

impl CompositeSubQuery {
    fn new(
        data_contract: Arc<DataContract>,
        document_type_name: &str,
        kind: CompositeSubQueryKind,
    ) -> Result<Self, Error> {
        data_contract
            .document_type_for_name(document_type_name)
            .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))?;
        Ok(Self {
            data_contract,
            document_type_name: document_type_name.to_string(),
            kind,
            where_clauses: Vec::new(),
            order_by_clauses: Vec::new(),
            limit: None,
            binding: None,
        })
    }

    /// A documents sub-query against `document_type_name` of
    /// `data_contract`. Unbound until [`Self::bound_to`] (a sibling
    /// otherwise).
    pub fn documents<C: Into<Arc<DataContract>>>(
        data_contract: C,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        Self::new(
            data_contract.into(),
            document_type_name,
            CompositeSubQueryKind::Documents,
        )
    }

    /// A count sub-query against `document_type_name` of
    /// `data_contract`. Must be bound.
    pub fn count<C: Into<Arc<DataContract>>>(
        data_contract: C,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        Self::new(
            data_contract.into(),
            document_type_name,
            CompositeSubQueryKind::Count,
        )
    }

    /// Bind `field` to the `source_property` values of `source`'s
    /// proven documents.
    pub fn bound_to(
        mut self,
        source: CompositeBindingSource,
        source_property: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        self.binding = Some(CompositeBinding {
            source,
            source_property: source_property.into(),
            field: field.into(),
        });
        self
    }

    /// Bind `field` to the `source_property` values of the page's
    /// proven documents.
    pub fn bound_to_page(
        self,
        source_property: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        self.bound_to(CompositeBindingSource::Page, source_property, field)
    }

    /// Add a fixed `where` clause.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.where_clauses.push(clause);
        self
    }

    /// Add an `order_by` clause (documents only).
    pub fn with_order_by(mut self, clause: OrderClause) -> Self {
        self.order_by_clauses.push(clause);
        self
    }

    /// Set the total row limit of a documents lookup.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl From<&DriveSubQuery<'_>> for CompositeSubQuery {
    fn from(sub: &DriveSubQuery<'_>) -> Self {
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        Self {
            data_contract: Arc::new(sub.contract.clone()),
            document_type_name: sub.document_type.name().to_string(),
            kind: match sub.kind {
                SubQueryKind::Documents => CompositeSubQueryKind::Documents,
                SubQueryKind::Count => CompositeSubQueryKind::Count,
            },
            where_clauses: sub.where_clauses.clone(),
            order_by_clauses: sub.order_by.clone(),
            limit: sub.limit.map(u32::from),
            binding: sub.binding.as_ref().map(|binding| CompositeBinding {
                source: match binding.source {
                    BindingSource::Page => CompositeBindingSource::Page,
                    BindingSource::SubQuery(index) => CompositeBindingSource::SubQuery(index),
                },
                source_property: binding.source_property.clone(),
                field: binding.field.clone(),
            }),
        }
    }
}

impl DocumentQuery {
    /// Append a sub-query derived from this page or an earlier sub-query.
    /// Fetch the result as [`CompositeDocuments`].
    pub fn with_sub_query(mut self, sub_query: CompositeSubQuery) -> Self {
        self.sub_queries.push(sub_query);
        self
    }

    /// Replace the sub-queries. An empty list makes this an ordinary query.
    pub fn with_sub_queries(mut self, sub_queries: Vec<CompositeSubQuery>) -> Self {
        self.sub_queries = sub_queries;
        self
    }

    /// Check the composite-only shape before encoding or building Drive queries.
    pub(super) fn check_composite_shape(&self) -> Result<(), Error> {
        if self.sub_queries.is_empty() || self.sub_queries.len() > MAX_SUB_QUERIES {
            return Err(Error::Config(format!(
                "a composite document query requires between 1 and {MAX_SUB_QUERIES} sub-queries"
            )));
        }
        check_page_shape(self)?;
        for (index, sub) in self.sub_queries.iter().enumerate() {
            if let Some(limit) = sub.limit {
                if limit == 0 || limit > u32::from(DEFAULT_QUERY_LIMIT) {
                    return Err(Error::Drive(drive::error::Error::Query(
                        QuerySyntaxError::InvalidLimit(format!(
                            "sub-query {index}: limit must be in [1, {DEFAULT_QUERY_LIMIT}], got {limit}"
                        )),
                    )));
                }
            }
            if let Some(CompositeBinding {
                source: CompositeBindingSource::SubQuery(source),
                ..
            }) = &sub.binding
            {
                if *source >= index
                    || self.sub_queries[*source].kind != CompositeSubQueryKind::Documents
                {
                    return Err(Error::Config(format!(
                        "sub-query {index}: a binding must name an earlier documents sub-query"
                    )));
                }
            }
        }
        Ok(())
    }
}

/// The page-side shape rules shared by the wire encoder and the drive
/// conversion: an explicit limit, a documents projection, and nothing
/// the composite surface cannot express.
fn check_page_shape(page: &DocumentQuery) -> Result<(), Error> {
    if page.limit == 0 || page.limit > u32::from(DEFAULT_QUERY_LIMIT) {
        return Err(Error::Config(format!(
            "a composite document query requires an explicit page limit between 1 and {DEFAULT_QUERY_LIMIT}: it \
             bounds every derived sub-query clause, so there is no server-default sentinel"
        )));
    }
    if page.select != SelectProjection::documents() {
        return Err(Error::Config(
            "a composite page supports the DOCUMENTS projection only".to_string(),
        ));
    }
    if !page.time_range_clauses.is_empty()
        || page.start.is_some()
        || page.offset.is_some()
        || !page.group_by.is_empty()
        || !page.having.is_empty()
    {
        return Err(Error::Config(
            "a composite page supports where/order_by/limit only: no time-range \
             selections, cursors, offsets, group_by, or having (paginate with a range \
             clause on the page's ordering property)"
                .to_string(),
        ));
    }
    Ok(())
}

/// Encode sub-queries after [`DocumentQuery::check_composite_shape`] has
/// bounded their count, binding indices, and limits.
pub(super) fn sub_queries_to_proto(
    sub_queries: Vec<CompositeSubQuery>,
) -> Result<Vec<ProtoSubQuery>, Error> {
    sub_queries
        .into_iter()
        .map(|sub_query| {
            let CompositeSubQuery {
                data_contract,
                document_type_name,
                kind,
                where_clauses,
                order_by_clauses,
                limit,
                binding,
            } = sub_query;
            let kind = match kind {
                CompositeSubQueryKind::Documents => sub_query::Kind::Documents,
                CompositeSubQueryKind::Count => sub_query::Kind::Count,
            };
            Ok(ProtoSubQuery {
                // Always explicit: the server treats an empty id as
                // "the page's contract", but naming it costs 32
                // bytes and removes a shape the verifier would
                // otherwise have to mirror.
                data_contract_id: data_contract.id().to_vec(),
                document_type: document_type_name,
                where_clauses: where_clauses
                    .into_iter()
                    .map(where_clause_to_proto)
                    .collect::<Result<Vec<_>, _>>()?,
                order_by: order_by_clauses
                    .into_iter()
                    .map(order_clause_to_proto)
                    .collect(),
                limit,
                kind: kind as i32,
                bind: binding.map(|binding| sub_query::Binding {
                    source: match binding.source {
                        CompositeBindingSource::Page => 0,
                        CompositeBindingSource::SubQuery(index) => index as u32 + 1,
                    },
                    source_property: binding.source_property,
                    field: binding.field,
                }),
            })
        })
        .collect::<Result<Vec<_>, Error>>()
}

/// Borrow sub-queries after [`DocumentQuery::check_composite_shape`] has
/// validated limits before narrowing them to Drive's `u16`.
pub(super) fn drive_sub_queries<'a>(
    request: &'a DocumentQuery,
) -> Result<Vec<DriveSubQuery<'a>>, Error> {
    request
        .sub_queries
        .iter()
        .map(|sub_query| {
            let contract: &'a DataContract = &sub_query.data_contract;
            let document_type = contract
                .document_type_for_name(&sub_query.document_type_name)
                .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))?;
            Ok(DriveSubQuery {
                contract,
                document_type,
                kind: match sub_query.kind {
                    CompositeSubQueryKind::Documents => SubQueryKind::Documents,
                    CompositeSubQueryKind::Count => SubQueryKind::Count,
                },
                where_clauses: sub_query.where_clauses.clone(),
                order_by: sub_query.order_by_clauses.clone(),
                limit: sub_query.limit.map(|limit| limit as u16),
                binding: sub_query.binding.as_ref().map(|binding| SubQueryBinding {
                    source: match binding.source {
                        CompositeBindingSource::Page => BindingSource::Page,
                        CompositeBindingSource::SubQuery(index) => BindingSource::SubQuery(index),
                    },
                    source_property: binding.source_property.clone(),
                    field: binding.field.clone(),
                }),
            })
        })
        .collect::<Result<Vec<_>, Error>>()
}

impl FromProof<DocumentQuery> for CompositeDocuments {
    type Request = DocumentQuery;
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
        request
            .check_composite_shape()
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: e.to_string(),
            })?;
        let response: Self::Response = response.into();

        let query: DriveDocumentQuery = (&request).try_into().map_err(|e: Error| {
            drive_proof_verifier::Error::RequestError {
                error: e.to_string(),
            }
        })?;

        // The standard envelope carries the single MERGED proof, and
        // the proof alone is enough: the verifier bootstraps the page
        // from it via a subset pass and re-derives the rest.
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

        let (_root_hash, composite) = verify_composite_documents_tenderdash_proof(
            &query,
            proof,
            mtd,
            platform_version,
            provider,
        )?;

        // An empty page is a valid, proven "nothing here" — surface it
        // as Some(empty) rather than None so callers can tell it apart
        // from a missing object.
        Ok((Some(composite), mtd.clone(), proof.clone()))
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the composite client surface: the V1
    //! request-wire encoding, the page-shape rejections, and the
    //! rich→drive conversion + shared shape validation against the
    //! yappr-feed fixture. Proof verification is exercised end to end
    //! in rs-drive's `composite_query_e2e_tests` and rs-drive-abci's
    //! composite dispatch and trust-boundary tests, where a populated
    //! Drive exists.

    use super::*;
    use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
    use dapi_grpc::platform::v0::GetDocumentsRequest;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::TryFromPlatformVersioned;
    use drive::query::WhereOperator;

    const FEED_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/yappr-feed/yappr-feed-contract.json";
    const DASHPAY_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json";

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn contract(path: &str) -> Arc<DataContract> {
        Arc::new(
            json_document_to_contract(path, false, platform_version())
                .expect("expected to parse the fixture contract"),
        )
    }

    /// The feed card composition: `dash` posts, their like counts, the
    /// posts they quote, and their authors' dashpay profiles.
    fn feed_page(limit: u32) -> DocumentQuery {
        let feed = contract(FEED_CONTRACT_PATH);
        let dashpay = contract(DASHPAY_CONTRACT_PATH);
        let page = DocumentQuery::new(feed.clone(), "post")
            .expect("post doctype exists")
            .with_where(WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            })
            .with_limit(limit);
        page.with_sub_query(
            CompositeSubQuery::count(feed.clone(), "like")
                .expect("like doctype exists")
                .bound_to_page("$id", "postId"),
        )
        .with_sub_query(
            CompositeSubQuery::documents(feed, "post")
                .expect("post doctype exists")
                .bound_to_page("quotedPostId", "$id"),
        )
        .with_sub_query(
            CompositeSubQuery::documents(dashpay, "profile")
                .expect("profile doctype exists")
                .bound_to_page("$ownerId", "$ownerId"),
        )
    }

    #[test]
    fn encodes_the_v1_wire_shape() {
        let query = feed_page(10);
        let dashpay_id = query.sub_queries[2].data_contract.id().to_vec();
        let request = GetDocumentsRequest::try_from_platform_versioned(query, platform_version())
            .expect("encodes");
        let Some(RequestVersion::V1(v1)) = request.version else {
            panic!("expected a V1 request");
        };
        assert_eq!(v1.document_type, "post");
        assert_eq!(v1.limit, Some(10));
        assert!(v1.prove, "composite fetch always proves");
        assert!(v1.chained.is_none(), "composite and chained are exclusive");
        assert_eq!(v1.where_clauses.len(), 1);
        assert_eq!(v1.sub_queries.len(), 3);

        let counts = &v1.sub_queries[0];
        assert_eq!(counts.document_type, "like");
        assert_eq!(counts.kind, sub_query::Kind::Count as i32);
        assert_eq!(counts.limit, None);
        let bind = counts.bind.as_ref().expect("bound");
        assert_eq!(bind.source, 0, "the page is source 0");
        assert_eq!(bind.source_property, "$id");
        assert_eq!(bind.field, "postId");

        let quoted = &v1.sub_queries[1];
        assert_eq!(quoted.kind, sub_query::Kind::Documents as i32);
        assert_eq!(quoted.bind.as_ref().expect("bound").field, "$id");

        let profiles = &v1.sub_queries[2];
        assert_eq!(profiles.data_contract_id, dashpay_id);
        assert_eq!(profiles.document_type, "profile");
    }

    #[test]
    fn numbers_sub_query_sources_from_one() {
        let feed = contract(FEED_CONTRACT_PATH);
        let query = feed_page(10).with_sub_query(
            CompositeSubQuery::count(feed, "like")
                .expect("like doctype exists")
                .bound_to(CompositeBindingSource::SubQuery(1), "$id", "postId"),
        );
        let request = GetDocumentsRequest::try_from_platform_versioned(query, platform_version())
            .expect("encodes");
        let Some(RequestVersion::V1(v1)) = request.version else {
            panic!("expected a V1 request");
        };
        assert_eq!(
            v1.sub_queries[3].bind.as_ref().expect("bound").source,
            2,
            "sub-query 1 is wire source 2"
        );
    }

    #[test]
    fn should_preserve_the_full_composition_through_drive_conversion() {
        let feed = contract(FEED_CONTRACT_PATH);
        let query = feed_page(10).with_sub_query(
            CompositeSubQuery::documents(feed, "repost")
                .expect("repost doctype exists")
                .bound_to(CompositeBindingSource::SubQuery(1), "$id", "postId")
                .with_where(WhereClause {
                    field: "hashtag".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                })
                .with_order_by(OrderClause {
                    field: "postId".to_string(),
                    ascending: true,
                })
                .with_limit(7),
        );
        let drive_query: DriveDocumentQuery = (&query).try_into().expect("converts");
        for restored in [
            DocumentQuery::try_from(&drive_query),
            DocumentQuery::try_from(drive_query.clone()),
            DocumentQuery::new_with_drive_query(&drive_query),
        ] {
            assert_eq!(restored.expect("preserves the composition"), query);
        }
    }

    #[test]
    fn should_reject_compositions_on_v0_without_affecting_ordinary_queries() {
        let mut v0 = platform_version().clone();
        v0.drive_abci.query.document_query.default_current_version = 0;
        let query = feed_page(10);
        let refused = GetDocumentsRequest::try_from_platform_versioned(query.clone(), &v0);
        assert!(matches!(refused, Err(Error::Config(message)) if message.contains("V1")));

        let ordinary = query.with_sub_queries(vec![]);
        let request = GetDocumentsRequest::try_from_platform_versioned(ordinary.clone(), &v0)
            .expect("ordinary queries still encode as V0");
        assert!(matches!(request.version, Some(RequestVersion::V0(_))));
        let request =
            GetDocumentsRequest::try_from_platform_versioned(ordinary, platform_version())
                .expect("ordinary queries still encode as V1");
        let Some(RequestVersion::V1(v1)) = request.version else {
            panic!("expected V1");
        };
        assert!(v1.sub_queries.is_empty());
    }

    #[test]
    fn should_enforce_sub_query_limits_before_encoding_or_conversion() {
        let query = feed_page(10);
        let sub = query.sub_queries[0].clone();
        let maximum = query
            .clone()
            .with_sub_queries(vec![sub.clone(); MAX_SUB_QUERIES]);
        GetDocumentsRequest::try_from_platform_versioned(maximum.clone(), platform_version())
            .expect("the maximum sub-query count encodes");
        DriveDocumentQuery::try_from(&maximum).expect("the maximum sub-query count converts");

        let excessive = query.with_sub_queries(vec![sub; MAX_SUB_QUERIES + 1]);
        assert!(GetDocumentsRequest::try_from_platform_versioned(
            excessive.clone(),
            platform_version()
        )
        .is_err());
        assert!(DriveDocumentQuery::try_from(&excessive).is_err());

        for limit in [0, 101, u32::MAX] {
            let mut query = feed_page(10);
            query.sub_queries[2].limit = Some(limit);
            assert!(GetDocumentsRequest::try_from_platform_versioned(
                query.clone(),
                platform_version()
            )
            .is_err());
            assert!(DriveDocumentQuery::try_from(&query).is_err());
        }
    }

    #[test]
    fn should_reject_invalid_binding_sources_before_encoding_or_conversion() {
        // Source 0 is a count, source 2 is the sub-query itself, and a
        // maximal index must not wrap when mapped to the wire's u32.
        for source in [0, 2, 3, usize::MAX] {
            let mut query = feed_page(10);
            query.sub_queries[2].binding.as_mut().expect("bound").source =
                CompositeBindingSource::SubQuery(source);
            assert!(GetDocumentsRequest::try_from_platform_versioned(
                query.clone(),
                platform_version()
            )
            .is_err());
            assert!(DriveDocumentQuery::try_from(&query).is_err());
        }
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn should_preserve_mock_compositions_and_read_older_ordinary_queries() {
        let query = feed_page(10);
        let encoded = serde_json::to_value(&query).expect("serializes");
        let restored: DocumentQuery =
            serde_json::from_value(encoded.clone()).expect("deserializes");
        assert_eq!(restored, query);

        let mut legacy = encoded;
        legacy
            .as_object_mut()
            .expect("query object")
            .remove("sub_queries");
        let restored: DocumentQuery = serde_json::from_value(legacy).expect("reads older vectors");
        assert_eq!(restored, query.with_sub_queries(vec![]));
    }

    #[test]
    fn should_reject_invalid_page_limits() {
        for limit in [0, 101, u32::MAX] {
            let query = feed_page(limit);
            let refused =
                GetDocumentsRequest::try_from_platform_versioned(query.clone(), platform_version());
            assert!(
                matches!(refused, Err(Error::Config(_))),
                "an invalid page limit must be refused, got {refused:?}"
            );
            assert!(DriveDocumentQuery::try_from(&query).is_err());
        }
    }

    #[test]
    fn refuses_unsupported_page_features() {
        let mut query = feed_page(10);
        query.offset = Some(4);
        let refused = GetDocumentsRequest::try_from_platform_versioned(query, platform_version());
        assert!(
            matches!(refused, Err(Error::Config(_))),
            "a page offset must be refused, got {refused:?}"
        );
    }

    #[test]
    fn converts_to_a_valid_drive_query() {
        let query = feed_page(10);
        let drive_query: DriveDocumentQuery =
            (&query).try_into().expect("converts to a drive query");
        drive_query
            .validate_composite(platform_version())
            .expect("the feed card shape validates");
        assert_eq!(drive_query.limit, Some(10));
        assert_eq!(drive_query.sub_queries.len(), 3);
        assert_eq!(drive_query.sub_queries[0].kind, SubQueryKind::Count);
        assert_eq!(
            drive_query.sub_queries[2]
                .binding
                .as_ref()
                .expect("bound")
                .source,
            BindingSource::Page
        );
    }

    #[test]
    fn conversion_refuses_an_out_of_range_sub_query_limit() {
        let feed = contract(FEED_CONTRACT_PATH);
        let query = feed_page(10).with_sub_query(
            CompositeSubQuery::documents(feed, "repost")
                .expect("repost doctype exists")
                .bound_to_page("$id", "postId")
                .with_limit(101),
        );
        let refused: Result<DriveDocumentQuery, _> = (&query).try_into();
        assert!(
            matches!(refused, Err(Error::Drive(_))),
            "a sub-query limit above the server maximum must be refused, got {refused:?}"
        );
    }

    #[test]
    fn conversion_surfaces_shape_errors() {
        let feed = contract(FEED_CONTRACT_PATH);
        let query = feed_page(10).with_sub_query(
            CompositeSubQuery::documents(feed, "post")
                .expect("post doctype exists")
                .bound_to_page("hashtag", "$id"),
        );
        let drive_query: DriveDocumentQuery =
            (&query).try_into().expect("conversion itself succeeds");
        assert!(
            drive_query.validate_composite(platform_version()).is_err(),
            "a by-id join off a non-refersTo property must fail validation"
        );
    }
}
