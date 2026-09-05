//! Composite document queries — the client half of "a page plus the
//! sub-queries derived from it", answered as ONE merged proof.
//!
//! The page is an ordinary [`DocumentQuery`] with an explicit limit.
//! Each [`CompositeSubQuery`] is a by-id join, an indexed lookup, a
//! grouped count, or an independent sibling, whose `IN` clause the
//! server derives from the proven page (or an earlier documents
//! sub-query) — the request never names the derived values, so the
//! responding node cannot steer them. The verifier bootstraps the page
//! from the merged proof and re-derives every sub-query with the same
//! builders, so a substituted, omitted or injected sub-result fails
//! verification. See `drive::query::drive_composite_document_query`
//! for the shape rules and the trust model.

use crate::documents::document_query::{
    order_clause_to_proto, where_clause_to_proto, DocumentQuery,
};
use crate::error::Error;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    sub_query, SubQuery as ProtoSubQuery,
};
use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use dpp::ProtocolError;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::error::query::QuerySyntaxError;
use drive::query::drive_composite_document_query::{
    BindingSource, DriveCompositeDocumentQuery, DriveSubQuery, SubQueryBinding, SubQueryKind,
};
use drive::query::{DriveDocumentQuery, OrderClause, SelectProjection, WhereClause};
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
    /// [`CompositeDocumentQuery::sub_queries`].
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

    /// Set the per-value limit of a documents lookup.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// A composite document query: the page plus its sub-queries, in
/// binding order (a sub-query may only bind the page or an earlier
/// documents sub-query).
///
/// The page MUST carry an explicit non-zero limit (it bounds every
/// derived clause; there is no server-default sentinel on this
/// surface) and supports where/order_by only: no cursor, offset,
/// projection, grouping or time-range selection — paginate with a
/// range clause on the page's ordering property.
#[derive(Debug, Clone, PartialEq, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct CompositeDocumentQuery {
    /// The page.
    pub page: DocumentQuery,
    /// The sub-queries, in request (and binding) order.
    pub sub_queries: Vec<CompositeSubQuery>,
}

impl CompositeDocumentQuery {
    /// A composite query around `page`, with no sub-queries yet.
    pub fn new(page: DocumentQuery) -> Self {
        Self {
            page,
            sub_queries: Vec::new(),
        }
    }

    /// Append a sub-query; its position is what later bindings name
    /// through [`CompositeBindingSource::SubQuery`].
    pub fn with_sub_query(mut self, sub_query: CompositeSubQuery) -> Self {
        self.sub_queries.push(sub_query);
        self
    }
}

/// The page-side shape rules shared by the wire encoder and the drive
/// conversion: an explicit limit, a documents projection, and nothing
/// the composite surface cannot express.
fn check_page_shape(page: &DocumentQuery) -> Result<(), Error> {
    if page.limit == 0 {
        return Err(Error::Config(
            "a composite document query requires an explicit non-zero page limit: it \
             bounds every derived sub-query clause, so there is no server-default sentinel"
                .to_string(),
        ));
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

impl TryFromPlatformVersioned<CompositeDocumentQuery> for GetDocumentsRequest {
    type Error = Error;

    fn try_from_platform_versioned(
        value: CompositeDocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let CompositeDocumentQuery { page, sub_queries } = value;
        check_page_shape(&page)?;

        let proto_sub_queries = sub_queries
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
            .collect::<Result<Vec<_>, Error>>()?;

        // The composite surface rides the typed V1 wire: encode the
        // page through the standard versioned encoder, then attach the
        // sub-queries. A network still on the V0 (CBOR) wire cannot
        // express the field, so refuse rather than silently sending a
        // plain documents query.
        let mut request = GetDocumentsRequest::try_from_platform_versioned(page, platform_version)?;
        match request.version.as_mut() {
            Some(RequestVersion::V1(v1)) => {
                v1.sub_queries = proto_sub_queries;
            }
            _ => {
                return Err(Error::Config(
                    "composite document queries require the V1 documents wire (Platform \
                     v3.1+); this network's protocol version encodes V0"
                        .to_string(),
                ));
            }
        }
        Ok(request)
    }
}

impl<'a> TryFrom<&'a CompositeDocumentQuery> for DriveCompositeDocumentQuery<'a> {
    type Error = Error;

    fn try_from(request: &'a CompositeDocumentQuery) -> Result<Self, Self::Error> {
        check_page_shape(&request.page)?;
        let page: DriveDocumentQuery<'a> = (&request.page).try_into()?;

        let sub_queries = request
            .sub_queries
            .iter()
            .enumerate()
            .map(|(index, sub_query)| {
                let contract: &'a DataContract = &sub_query.data_contract;
                let document_type = contract
                    .document_type_for_name(&sub_query.document_type_name)
                    .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))?;
                // Mirror the server's limit contract: `[1,
                // max_query_limit]`, with anything else refused rather
                // than clamped, so a proof can only ever verify against
                // a query an honest server would have run.
                let limit = match sub_query.limit {
                    None => None,
                    Some(limit) if limit >= 1 && limit <= u32::from(DEFAULT_QUERY_LIMIT) => {
                        Some(limit as u16)
                    }
                    Some(limit) => {
                        return Err(Error::Drive(drive::error::Error::Query(
                            QuerySyntaxError::InvalidLimit(format!(
                                "sub-query {}: limit must be in [1, {}], got {}",
                                index, DEFAULT_QUERY_LIMIT, limit
                            )),
                        )));
                    }
                };
                Ok(DriveSubQuery {
                    contract,
                    document_type,
                    kind: match sub_query.kind {
                        CompositeSubQueryKind::Documents => SubQueryKind::Documents,
                        CompositeSubQueryKind::Count => SubQueryKind::Count,
                    },
                    where_clauses: sub_query.where_clauses.clone(),
                    order_by: sub_query.order_by_clauses.clone(),
                    limit,
                    binding: sub_query.binding.as_ref().map(|binding| SubQueryBinding {
                        source: match binding.source {
                            CompositeBindingSource::Page => BindingSource::Page,
                            CompositeBindingSource::SubQuery(index) => {
                                BindingSource::SubQuery(index)
                            }
                        },
                        source_property: binding.source_property.clone(),
                        field: binding.field.clone(),
                    }),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(DriveCompositeDocumentQuery { page, sub_queries })
    }
}

impl FromProof<CompositeDocumentQuery> for CompositeDocuments {
    type Request = CompositeDocumentQuery;
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

        let query: DriveCompositeDocumentQuery = (&request).try_into().map_err(|e: Error| {
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
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract;
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
    fn feed_page(limit: u32) -> CompositeDocumentQuery {
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
        CompositeDocumentQuery::new(page)
            .with_sub_query(
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
    fn requires_a_page_limit() {
        let refused =
            GetDocumentsRequest::try_from_platform_versioned(feed_page(0), platform_version());
        assert!(
            matches!(refused, Err(Error::Config(_))),
            "a zero page limit must be refused, got {refused:?}"
        );
    }

    #[test]
    fn refuses_unsupported_page_features() {
        let mut query = feed_page(10);
        query.page.offset = Some(4);
        let refused = GetDocumentsRequest::try_from_platform_versioned(query, platform_version());
        assert!(
            matches!(refused, Err(Error::Config(_))),
            "a page offset must be refused, got {refused:?}"
        );
    }

    #[test]
    fn converts_to_a_valid_drive_query() {
        let query = feed_page(10);
        let drive_query: DriveCompositeDocumentQuery =
            (&query).try_into().expect("converts to a drive query");
        drive_query
            .validate(platform_version())
            .expect("the feed card shape validates");
        assert_eq!(drive_query.page.limit, Some(10));
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
        let refused: Result<DriveCompositeDocumentQuery, _> = (&query).try_into();
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
        let drive_query: DriveCompositeDocumentQuery =
            (&query).try_into().expect("conversion itself succeeds");
        assert!(
            drive_query.validate(platform_version()).is_err(),
            "a by-id join off a non-refersTo property must fail validation"
        );
    }
}
