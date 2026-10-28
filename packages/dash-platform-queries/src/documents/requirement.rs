//! The protocol version from which every node serves what a document request uses.
//!
//! A protocol upgrade adds document features in two ways: a request field or operator that
//! nodes of an older release do not know, or a route the server opens behind a version table
//! slot. Below the version that adds it, a node drops the field and proves another question,
//! refuses the request, or serves it as another one. A caller compares
//! [`DocumentQuery::required_protocol_version`] with the protocol version the network is
//! proven to run before it sends the request.
//!
//! Each feature reads the version table slot that decides it, never a literal: the slot the
//! server switches on, or, for a field nodes of an older release drop, the feature slot that
//! is `None` before the protocol version that adds the field.

use crate::documents::chained_document_query::ChainedDocumentQuery;
use crate::documents::document_query::DocumentQuery;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::version::{PlatformVersion, ProtocolVersion, PLATFORM_VERSIONS};
use drive::query::drive_document_ranked_query::mode_detection::ranked_order_key;
use drive::query::{SelectFunction, WhereOperator};

/// The protocol version from which every node serves a document request, and the feature of
/// the request that needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredProtocolVersion {
    /// The first protocol version at which every node serves the request.
    pub protocol_version: ProtocolVersion,
    /// The feature that needs it, named for the caller, for example "time-range selections".
    pub feature: &'static str,
}

/// A document request feature that nodes below some protocol version do not serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Feature {
    SubQueries,
    Chained,
    ArrayElements,
    TimeRanges,
    IntegerRanges,
    Ranked,
    HavingRange,
    SeveralIns,
    CursorOnSummedDocuments,
}

impl Feature {
    #[cfg(test)]
    const ALL: [Self; 9] = [
        Self::SubQueries,
        Self::Chained,
        Self::ArrayElements,
        Self::TimeRanges,
        Self::IntegerRanges,
        Self::Ranked,
        Self::HavingRange,
        Self::SeveralIns,
        Self::CursorOnSummedDocuments,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SubQueries => "composite sub-queries",
            Self::Chained => "chained queries",
            Self::ArrayElements => "contains and containsAny clauses",
            Self::TimeRanges => "time-range selections",
            Self::IntegerRanges => "integer-range selections",
            Self::Ranked => "aggregates ordered by the selected aggregate",
            Self::HavingRange => "having clauses",
            Self::SeveralIns => "several in clauses",
            Self::CursorOnSummedDocuments => "start cursors on a type whose documents are summed",
        }
    }

    /// Whether every node that signs `platform_version` serves the feature.
    fn served_at(self, platform_version: &PlatformVersion) -> bool {
        let query = &platform_version.drive_abci.query;
        let drive_query = &platform_version.drive.methods.document.query;
        match self {
            // Nodes of a release without the field drop it.
            Self::SubQueries => query.document_sub_queries.is_some(),
            Self::Chained => query.chained_document_query.is_some(),
            // No index keys the elements of a typed array, or a time or integer window, before
            // contract grammar generation 3, and nodes refuse the operators until then.
            Self::ArrayElements | Self::TimeRanges | Self::IntegerRanges => {
                platform_version
                    .dpp
                    .contract_versions
                    .document_type_versions
                    .schema
                    .document_type_schema
                    >= 3
            }
            // The server's own routing: below these helper versions it serves a ranked
            // request as a plain grouped aggregate or refuses it, and refuses every having.
            Self::Ranked => {
                query
                    .document_query_helpers
                    .compute_aggregate_mode_and_check_limit
                    >= 1
            }
            Self::HavingRange => {
                query
                    .document_query_helpers
                    .compute_aggregate_mode_and_check_limit
                    >= 2
            }
            // Grouping v0 refuses more than one `In` clause.
            Self::SeveralIns => drive_query.where_clause_grouping >= 1,
            // Cursor reading v0 refuses the sum items such documents are stored as, with an
            // error clients retry on every other node.
            Self::CursorOnSummedDocuments => drive_query.start_at_document >= 1,
        }
    }

    /// The first protocol version from which every node serves the feature, or
    /// [`ProtocolVersion::MAX`] when no protocol version this build knows serves it.
    fn first_protocol_version(self) -> ProtocolVersion {
        PLATFORM_VERSIONS
            .iter()
            .find(|platform_version| self.served_at(platform_version))
            .map_or(ProtocolVersion::MAX, |platform_version| {
                platform_version.protocol_version
            })
    }
}

/// The most demanding of `features`, or `None` when there are none.
fn requirement(features: impl IntoIterator<Item = Feature>) -> Option<RequiredProtocolVersion> {
    features
        .into_iter()
        .map(|feature| RequiredProtocolVersion {
            protocol_version: feature.first_protocol_version(),
            feature: feature.name(),
        })
        .max_by_key(|requirement| requirement.protocol_version)
}

impl DocumentQuery {
    /// The protocol version from which every node serves the features this query uses, or
    /// `None` when every node serves it.
    ///
    /// It does not depend on the protocol version the query is encoded with: it says which
    /// networks can answer the query, not how to encode it.
    pub fn required_protocol_version(&self) -> Option<RequiredProtocolVersion> {
        requirement(self.features())
    }

    fn features(&self) -> Vec<Feature> {
        let mut features = Vec::new();
        if !self.sub_queries.is_empty() {
            features.push(Feature::SubQueries);
        }
        if self
            .where_clauses
            .iter()
            .chain(
                self.sub_queries
                    .iter()
                    .flat_map(|sub_query| sub_query.where_clauses.iter()),
            )
            .any(|clause| clause.operator.reads_array_elements())
        {
            features.push(Feature::ArrayElements);
        }
        if !self.time_range_clauses.is_empty() {
            features.push(Feature::TimeRanges);
        }
        if !self.integer_range_clauses.is_empty() {
            features.push(Feature::IntegerRanges);
        }
        if self.select.function == SelectFunction::Documents {
            let non_primary_ins = self
                .where_clauses
                .iter()
                .filter(|clause| clause.operator == WhereOperator::In && !clause.is_identifier())
                .count();
            if non_primary_ins > 1 {
                features.push(Feature::SeveralIns);
            }
            if self.start.is_some() && self.documents_are_summed() {
                features.push(Feature::CursorOnSummedDocuments);
            }
        } else if !self.group_by.is_empty() {
            if let [only] = self.order_by_clauses.as_slice() {
                if only.field == ranked_order_key(&self.select) {
                    features.push(Feature::Ranked);
                }
            }
            if !self.having.is_empty() {
                features.push(Feature::HavingRange);
            }
        }
        features
    }

    /// Whether the queried document type sums its documents, which stores them as sum items.
    fn documents_are_summed(&self) -> bool {
        self.data_contract
            .document_type_for_name(&self.document_type_name)
            .is_ok_and(|document_type| document_type.documents_summable().is_some())
    }
}

impl ChainedDocumentQuery {
    /// The protocol version from which every node serves the join and the features its inner
    /// query uses.
    pub fn required_protocol_version(&self) -> Option<RequiredProtocolVersion> {
        requirement(std::iter::once(Feature::Chained).chain(self.inner.features()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::composite_document_query::CompositeSubQuery;
    use crate::documents::document_query::RankingDirection;
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
    use dpp::data_contract::{DataContract, DataContractFactory};
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::query::{
        HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
        OrderClause, SelectProjection, TimeRangeSelector, WhereClause,
    };
    use std::sync::Arc;

    fn contract() -> Arc<DataContract> {
        Arc::new(
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned(),
        )
    }

    fn query() -> DocumentQuery {
        DocumentQuery::new(contract(), "niceDocument").expect("the fixture has this document type")
    }

    /// A `tip` summed by `amount`.
    fn tips() -> DocumentQuery {
        let schemas = platform_value!({"tip": {
            "type": "object",
            "documentsSummable": "amount",
            "properties": {
                "recipient": {"type": "string", "minLength": 1, "maxLength": 63, "position": 0},
                "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1}
            },
            "required": ["recipient", "amount"],
            "indices": [
                {"name": "byRecipient", "properties": [{"recipient": "asc"}], "summable": "amount"}
            ],
            "additionalProperties": false
        }});
        let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([8u8; 32]), 0, schemas, None, None)
            .expect("the tip contract is well-formed")
            .data_contract_owned();
        DocumentQuery::new(Arc::new(contract), "tip").expect("the contract has this type")
    }

    fn clause(field: &str, operator: WhereOperator, value: Value) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator,
            value,
        }
    }

    fn having() -> Vec<HavingClause> {
        vec![HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Count,
                field: String::new(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(Value::U64(1)),
        }]
    }

    fn sub_query() -> CompositeSubQuery {
        CompositeSubQuery::documents(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_limit(10)
    }

    /// Whether a protocol version's tables serve a feature.
    type Served = fn(&PlatformVersion) -> bool;

    /// The first protocol version at which `served` holds, read from the tables.
    fn first_where(served: impl Fn(&PlatformVersion) -> bool) -> ProtocolVersion {
        PLATFORM_VERSIONS
            .iter()
            .find(|platform_version| served(platform_version))
            .expect("some known version serves it")
            .protocol_version
    }

    fn required(query: &DocumentQuery) -> Option<(ProtocolVersion, &'static str)> {
        query
            .required_protocol_version()
            .map(|requirement| (requirement.protocol_version, requirement.feature))
    }

    /// Queries built only from what every node serves need nothing.
    #[test]
    fn should_require_nothing_of_queries_every_node_serves() {
        let ins = |field: &str| clause(field, WhereOperator::In, platform_value!(["a", "b"]));
        for query in [
            query(),
            query().with_where(ins("$ownerId")),
            query().with_where(ins("$id")).with_where(ins("$ownerId")),
            query().with_select(SelectProjection::count_star()),
            query()
                .with_select(SelectProjection::count_star())
                .with_where(ins("restaurantId"))
                .with_group_by("restaurantId"),
            // Ordered by the grouped property, not by the aggregate: a grouped count.
            query()
                .with_select(SelectProjection::count_star())
                .with_group_by("restaurantId")
                .with_order_by(OrderClause {
                    field: "restaurantId".to_string(),
                    ascending: true,
                }),
            tips(),
            tips().with_select(SelectProjection::sum("amount")),
        ] {
            assert_eq!(required(&query), None, "{query:?}");
        }
    }

    /// Each feature is detected and needs the first protocol version whose table serves it,
    /// whichever slot that is.
    #[test]
    fn should_require_the_first_version_whose_table_serves_each_feature() {
        let ranked = query()
            .with_select(SelectProjection::count_star())
            .with_group_by("restaurantId")
            .order_by_selected_aggregate(RankingDirection::Descending)
            .with_limit(5);
        let start = |mut query: DocumentQuery| {
            query.start = Some(Start::StartAfter(vec![1; 32]));
            query
        };
        let cases: Vec<(&str, DocumentQuery, Served)> = vec![
            (
                "composite sub-queries",
                query().with_limit(10).with_sub_query(sub_query()),
                |v| v.drive_abci.query.document_sub_queries.is_some(),
            ),
            (
                "contains and containsAny clauses",
                query().with_where(clause(
                    "tags",
                    WhereOperator::Contains,
                    Value::Text("dash".into()),
                )),
                |v| {
                    v.dpp
                        .contract_versions
                        .document_type_versions
                        .schema
                        .document_type_schema
                        >= 3
                },
            ),
            (
                "contains and containsAny clauses",
                query()
                    .with_limit(10)
                    .with_sub_query(sub_query().with_where(clause(
                        "tags",
                        WhereOperator::ContainsAny,
                        platform_value!(["dash"]),
                    ))),
                |v| {
                    v.dpp
                        .contract_versions
                        .document_type_versions
                        .schema
                        .document_type_schema
                        >= 3
                },
            ),
            (
                "time-range selections",
                query().with_time_range("$createdAt", TimeRangeSelector::Newest),
                |v| {
                    v.dpp
                        .contract_versions
                        .document_type_versions
                        .schema
                        .document_type_schema
                        >= 3
                },
            ),
            (
                "integer-range selections",
                query().with_integer_range("price", Value::U64(600)),
                |v| {
                    v.dpp
                        .contract_versions
                        .document_type_versions
                        .schema
                        .document_type_schema
                        >= 3
                },
            ),
            (
                "aggregates ordered by the selected aggregate",
                ranked,
                |v| {
                    v.drive_abci
                        .query
                        .document_query_helpers
                        .compute_aggregate_mode_and_check_limit
                        >= 1
                },
            ),
            (
                "having clauses",
                query()
                    .with_select(SelectProjection::count_star())
                    .with_group_by("hashtag")
                    .with_having(having()),
                |v| {
                    v.drive_abci
                        .query
                        .document_query_helpers
                        .compute_aggregate_mode_and_check_limit
                        >= 2
                },
            ),
            (
                "several in clauses",
                query()
                    .with_where(clause(
                        "restaurantId",
                        WhereOperator::In,
                        platform_value!(["a"]),
                    ))
                    .with_where(clause("grade", WhereOperator::In, platform_value!([1, 2]))),
                |v| v.drive.methods.document.query.where_clause_grouping >= 1,
            ),
            (
                "start cursors on a type whose documents are summed",
                start(tips()),
                |v| v.drive.methods.document.query.start_at_document >= 1,
            ),
        ];
        for (feature, query, served) in cases {
            assert_eq!(
                required(&query),
                Some((first_where(served), feature)),
                "{query:?}"
            );
        }
        // A cursor on a type whose documents are not summed is read by every node.
        assert_eq!(required(&start(query())), None);
    }

    /// The join needs its own version, and the query needs the most demanding of its
    /// features, in the page and in the chained inner query alike.
    #[test]
    fn should_require_the_most_demanding_feature_of_a_query() {
        let chained = ChainedDocumentQuery::new(query().with_limit(10), "ownerId", "niceDocument");
        assert_eq!(
            chained
                .required_protocol_version()
                .map(|requirement| (requirement.protocol_version, requirement.feature)),
            Some((
                first_where(|v| v.drive_abci.query.chained_document_query.is_some()),
                "chained queries"
            ))
        );
        let several = query()
            .with_limit(10)
            .with_sub_query(sub_query())
            .with_time_range("$createdAt", TimeRangeSelector::Newest);
        let most_demanding = [Feature::SubQueries, Feature::TimeRanges]
            .into_iter()
            .map(Feature::first_protocol_version)
            .max();
        assert_eq!(
            several
                .required_protocol_version()
                .map(|requirement| requirement.protocol_version),
            most_demanding
        );
    }

    /// Every feature is in [`Feature::ALL`], so the checks over it cover them all: a feature
    /// added without it fails to compile here.
    #[test]
    fn should_list_every_feature() {
        let position = |feature: Feature| match feature {
            Feature::SubQueries => 0,
            Feature::Chained => 1,
            Feature::ArrayElements => 2,
            Feature::TimeRanges => 3,
            Feature::IntegerRanges => 4,
            Feature::Ranked => 5,
            Feature::HavingRange => 6,
            Feature::SeveralIns => 7,
            Feature::CursorOnSummedDocuments => 8,
        };
        for (index, feature) in Feature::ALL.into_iter().enumerate() {
            assert_eq!(position(feature), index, "{feature:?}");
        }
    }

    /// Once a protocol version serves a feature, every later one does: the requirement is
    /// the first version, so a feature withdrawn later would be sent to nodes that refuse it.
    #[test]
    fn should_serve_each_feature_at_every_version_from_its_first() {
        for feature in Feature::ALL {
            let first = feature.first_protocol_version();
            assert!(
                first <= PlatformVersion::latest().protocol_version,
                "{feature:?} is served by no known version"
            );
            for platform_version in PLATFORM_VERSIONS {
                assert_eq!(
                    feature.served_at(platform_version),
                    platform_version.protocol_version >= first,
                    "{feature:?} at {}",
                    platform_version.protocol_version
                );
            }
        }
    }
}
