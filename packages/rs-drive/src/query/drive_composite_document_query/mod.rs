//! Composite document queries: one page query plus sub-queries derived
//! from its proven results, answered as ONE merged grovedb proof.
//!
//! A feed is a page of posts and then, for that page, the things a card
//! renders: the referenced (quoted) posts, the per-post engagement
//! counts, the authors' profiles, the viewer's own likes. Each of those
//! is a query whose INPUT is the page — its ids, its owners, a
//! property's values — and asking for them one round trip at a time
//! turns a single feed into a burst of dependent calls. A composite
//! query carries the page and its sub-queries in one request and proves
//! them together: the server materializes the page, derives every
//! sub-query's `IN` clause from it (or from an earlier sub-query's
//! documents), and `prove_query_many` merges all the component path
//! queries into one proof over one state root.
//!
//! Soundness never rests on the server's derivation. The verifier
//! bootstraps the page (a subset pass against the merged proof), derives
//! every sub-query itself with the SAME builders the server ran, merges
//! the same way, and verifies the whole composition in one authoritative
//! pass; then it recomputes the derived values from the proven page and
//! refuses any divergence from the bootstrap, any result outside a
//! derived value set, and (for by-id joins on `refersTo:
//! permanentDocument` properties, which cannot dangle) any missing
//! referenced document. A node that ignores the sub-queries serves a
//! page-only proof, which cannot satisfy the merged query whenever a
//! sub-query derived anything — the composition fails closed.
//!
//! Three sub-query shapes, one binding rule:
//!
//! - **Documents by id** (`bind.field == "$id"`): the classic join. The
//!   source property must declare `refersTo: permanentDocument` targeting
//!   the sub-query's type, so every derived id MUST resolve — the result
//!   is the referenced documents in first-appearance order, set-equal to
//!   the derived ids.
//! - **Documents by an indexed property** (`bind.field` is `$ownerId` or
//!   an indexed property): a lookup, `WHERE <fixed clauses> AND <field>
//!   IN <derived values>`, with an explicit limit unless the values
//!   already bound it (a unique index, or an indexOnly terminal with
//!   every prefix fixed, yields at most one row per value). Absence is
//!   inherent in the range proof (a value with no document simply
//!   yields none), so profiles keyed by owner or reposts keyed by post
//!   work without absence proofs, and the target may live in another
//!   contract.
//! - **Count** by an indexed property: the grouped point-lookup count
//!   `COUNT(*) WHERE <fixed clauses> AND <field> IN <derived values>
//!   GROUP BY <field>` on a `countable` index — one entry per value that
//!   has a count tree (zero-count trees are not materialized).
//!
//! A sub-query without a binding is a **sibling**: an independent
//! documents query proven under the same root (counts must be bound —
//! the aggregate and range count shapes have their own proof
//! primitives and stay on the regular count surface).
//!
//! Derived values are identifiers only (v1): the page's `$id`, its
//! `$ownerId`, or an identifier-typed property. The page limit is
//! required and capped at [`MAX_BOUND_VALUES`] (an `IN` clause admits at
//! most that many values); the page takes no cursor and no offset —
//! paginate with a range clause, exactly as chained queries do.

use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::index_only_synthesis::synthesize_index_only_document;
use crate::query::{
    DriveDocumentCountQuery, DriveDocumentQuery, InternalClauses, OrderClause, SplitCountEntry,
    WhereClause, WhereOperator,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentTypeRef,
};
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery};
use std::collections::{BTreeMap, BTreeSet};

/// The most sub-queries one composite request carries. Every sub-query
/// is another branch of one merged proof; ten covers a feed card's
/// whole enrichment (quotes, four counts, reposts, profiles, names,
/// the viewer's marks) with room to spare.
pub const MAX_SUB_QUERIES: usize = 10;

/// The most values one binding can derive: a derived `IN` clause admits
/// at most this many (`WhereClause::in_values`), so the page limit and
/// every sub-query limit that feeds a later binding are capped here.
pub const MAX_BOUND_VALUES: usize = 100;

/// Where a sub-query's derived values come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingSource {
    /// The page's proven documents.
    Page,
    /// An earlier documents sub-query's proven documents (its index in
    /// [`DriveCompositeDocumentQuery::sub_queries`]).
    SubQuery(usize),
}

/// The derived clause of a sub-query: `<field> IN <values>`, where the
/// values are read off the source's proven documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubQueryBinding {
    /// Whose documents supply the values.
    pub source: BindingSource,
    /// The source property read off each document: `$id`, `$ownerId`,
    /// or an identifier-typed property (dotted paths reach nested
    /// properties). Documents without the property contribute nothing.
    pub source_property: String,
    /// The sub-query field that receives the `IN` clause: `$id` for a
    /// by-id join, otherwise `$ownerId` or an indexed property.
    pub field: String,
}

/// What a sub-query returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubQueryKind {
    /// The matching documents.
    Documents,
    /// One count per derived value, from the countable index covering
    /// the fixed clauses plus the bound field.
    Count,
}

/// One sub-query of a composite request.
#[derive(Debug, Clone)]
pub struct DriveSubQuery<'a> {
    /// The contract the sub-query targets — the page's, or another one.
    pub contract: &'a DataContract,
    /// The document type queried.
    pub document_type: DocumentTypeRef<'a>,
    /// Documents or counts.
    pub kind: SubQueryKind,
    /// The fixed clauses (everything but the derived `IN`), typed.
    pub where_clauses: Vec<WhereClause>,
    /// Ordering; documents only.
    pub order_by: Vec<OrderClause>,
    /// Required for a documents lookup on a non-unique index (it bounds
    /// the walk under each value); forbidden for a value-bounded lookup,
    /// a by-id join (completeness is set-based) and a count.
    pub limit: Option<u16>,
    /// The derived clause, or `None` for a sibling.
    pub binding: Option<SubQueryBinding>,
}

/// A composite document query.
///
/// Construction contract: every sub-query's `document_type` MUST be a
/// document type of its own `contract`. [`Self::validate`] enforces
/// everything derivable from the shapes themselves.
#[derive(Debug, Clone)]
pub struct DriveCompositeDocumentQuery<'a> {
    /// The page: an ordinary document query with an explicit limit.
    pub page: DriveDocumentQuery<'a>,
    /// The sub-queries, in binding order (a sub-query may only bind an
    /// earlier one).
    pub sub_queries: Vec<DriveSubQuery<'a>>,
}

/// One sub-query's materialized result.
#[derive(Debug, Clone, PartialEq)]
pub enum SubQueryResult {
    /// Documents: for a by-id join, in first-appearance order of their
    /// ids among the source documents; otherwise in query order.
    Documents(Vec<Document>),
    /// Counts keyed by the bound value's index-key bytes (a 32-byte
    /// identifier), one entry per value with a materialized count.
    Counts(Vec<SplitCountEntry>),
}

impl SubQueryResult {
    /// The documents of a documents result, or an empty slice.
    pub fn documents(&self) -> &[Document] {
        match self {
            Self::Documents(documents) => documents,
            Self::Counts(_) => &[],
        }
    }

    /// The entries of a count result, or an empty slice.
    pub fn counts(&self) -> &[SplitCountEntry] {
        match self {
            Self::Counts(entries) => entries,
            Self::Documents(_) => &[],
        }
    }
}

/// The materialized result of a composite query.
#[derive(Debug, Default)]
pub struct CompositeDocumentsResult {
    /// The page, exactly as the page query alone would return it.
    pub page_documents: Vec<Document>,
    /// One result per sub-query, in request order.
    pub sub_results: Vec<SubQueryResult>,
}

/// The values one binding derived, deduplicated to first appearance.
type DerivedValues = Vec<Identifier>;

/// A `(path, key, element)` triple as grovedb's verifier reports it —
/// the element absent for a queried key that is not there.
pub(crate) type ProvedTrio = (Vec<Vec<u8>>, Vec<u8>, Option<Element>);

/// A proved triple whose element is present.
pub(crate) type PresentTrio = (Vec<Vec<u8>>, Vec<u8>, Element);

/// What the routing step decoded out of one path-query group's trios.
enum DecodedItems {
    Documents(Vec<Document>),
    Counts(Vec<SplitCountEntry>),
}

/// A component of the merged proof: the page or one sub-query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Page,
    Sub(usize),
}

fn unsupported(message: String) -> Error {
    Error::Query(QuerySyntaxError::Unsupported(message))
}

fn corrupted_proof(message: String) -> Error {
    Error::Proof(ProofError::CorruptedProof(message))
}

/// The bound identifier a document carries for `field`, or `None` when
/// the property is absent.
fn document_bound_value(document: &Document, field: &str) -> Result<Option<Identifier>, Error> {
    use dpp::document::property_names::{ID, OWNER_ID};
    if field == ID {
        return Ok(Some(document.id()));
    }
    if field == OWNER_ID {
        return Ok(Some(document.owner_id()));
    }
    let Some(value) = document
        .properties()
        .get_optional_at_path(field)
        .ok()
        .flatten()
    else {
        return Ok(None);
    };
    value.to_identifier().map(Some).map_err(|_| {
        Error::Drive(DriveError::CorruptedCodeExecution(
            "a bound composite property must decode as an identifier: validate() only \
             admits identifier-typed properties",
        ))
    })
}

/// Canonical value order for a derived `IN` clause: byte-ascending, so
/// the built query — and therefore the proof — is byte-identical between
/// the server and a verifier that extracted the ids in any order.
fn sorted_values(values: &[Identifier]) -> Vec<Identifier> {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted
}

impl<'a> DriveSubQuery<'a> {
    fn bound_field(&self) -> Option<&str> {
        self.binding.as_ref().map(|binding| binding.field.as_str())
    }

    fn is_by_id_join(&self) -> bool {
        self.bound_field() == Some(dpp::document::property_names::ID)
    }
}

impl<'a> DriveCompositeDocumentQuery<'a> {
    /// Validates the composite shape. Called by the server before
    /// executing and by the verifier before verifying, so an invalid
    /// request fails identically on both sides.
    pub fn validate(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        if self.sub_queries.is_empty() {
            return Err(unsupported(
                "a composite query needs at least one sub-query; a page alone is a plain \
                 documents query"
                    .to_string(),
            ));
        }
        if self.sub_queries.len() > MAX_SUB_QUERIES {
            return Err(unsupported(format!(
                "a composite query carries at most {} sub-queries, got {}",
                MAX_SUB_QUERIES,
                self.sub_queries.len(),
            )));
        }
        match self.page.limit {
            None => {
                return Err(unsupported(
                    "composite queries require an explicit limit on the page: the page size \
                     bounds every derived sub-query"
                        .to_string(),
                ));
            }
            Some(limit) if limit as usize > MAX_BOUND_VALUES => {
                return Err(unsupported(format!(
                    "a composite page limit of {} exceeds {}: a derived `IN` clause admits at \
                     most that many values",
                    limit, MAX_BOUND_VALUES,
                )));
            }
            Some(_) => {}
        }
        if self.page.offset.is_some() {
            return Err(unsupported(
                "composite queries do not support a page offset; paginate with a range clause"
                    .to_string(),
            ));
        }
        if self.page.start_at.is_some() {
            return Err(unsupported(
                "composite queries do not support a page cursor (startAt/startAfter); \
                 paginate with a range clause on the page's ordering property"
                    .to_string(),
            ));
        }
        // A by-ids page is proven without its limit (see
        // `page_path_query`), so the limit must not be what bounds it.
        if self.page_is_by_ids() {
            let ids = self.page_ids()?.len();
            if (self.page.limit.unwrap_or(0) as usize) < ids {
                return Err(unsupported(format!(
                    "a by-ids composite page addresses {} ids but its limit is {}: the ids \
                     bound the page, so the limit must cover them",
                    ids,
                    self.page.limit.unwrap_or(0),
                )));
            }
        }
        // The page must lower to a path query at all — an unindexed
        // shape fails here, before any sub-query is inspected.
        self.page_path_query(platform_version)?;

        for (index, sub_query) in self.sub_queries.iter().enumerate() {
            self.validate_sub_query(index, sub_query, platform_version)?;
        }
        self.validate_component_paths(platform_version)
    }

    fn validate_sub_query(
        &self,
        index: usize,
        sub_query: &DriveSubQuery<'a>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let label = |message: &str| unsupported(format!("sub-query {}: {}", index, message));

        let Some(binding) = &sub_query.binding else {
            // A sibling: an independent documents query.
            if sub_query.kind == SubQueryKind::Count {
                return Err(label(
                    "a count sub-query must be bound (`COUNT ... WHERE <field> IN <derived \
                     values> GROUP BY <field>`); unbound counts stay on the regular count \
                     surface",
                ));
            }
            match sub_query.limit {
                None => {
                    return Err(label(
                        "a sibling documents sub-query requires an explicit limit",
                    ));
                }
                Some(limit) if limit as usize > MAX_BOUND_VALUES => {
                    return Err(label(&format!(
                        "limit {} exceeds {}",
                        limit, MAX_BOUND_VALUES
                    )));
                }
                Some(_) => {}
            }
            // Must lower to a path query.
            self.sub_query_document_query(sub_query, &[], platform_version)?
                .construct_path_query(None, platform_version)?;
            return Ok(());
        };

        // The source must precede this sub-query and produce documents.
        let (source_contract, source_type, source_is_index_only_query) = match binding.source {
            BindingSource::Page => (
                self.page.contract,
                self.page.document_type,
                self.page.document_type.index_only(),
            ),
            BindingSource::SubQuery(source_index) => {
                if source_index >= index {
                    return Err(label("a binding may only reference an earlier sub-query"));
                }
                let source = &self.sub_queries[source_index];
                if source.kind != SubQueryKind::Documents {
                    return Err(label("a binding must reference a documents sub-query"));
                }
                (
                    source.contract,
                    source.document_type,
                    source.document_type.index_only(),
                )
            }
        };

        // The source property: a system identifier or an identifier-typed
        // property of the source type.
        let source_property_type: Option<&DocumentPropertyType> = {
            use dpp::document::property_names::{ID, OWNER_ID};
            if binding.source_property == ID || binding.source_property == OWNER_ID {
                None
            } else {
                let Some(property) = source_type
                    .flattened_properties()
                    .get(binding.source_property.as_str())
                else {
                    return Err(label(&format!(
                        "source property \"{}\" does not name a property of \"{}\"",
                        binding.source_property,
                        source_type.name(),
                    )));
                };
                if !matches!(
                    property.property_type,
                    DocumentPropertyType::Identifier
                        | DocumentPropertyType::IdentifierWithReference(_)
                ) {
                    return Err(label(&format!(
                        "source property \"{}\" is not identifier-typed; composite bindings \
                         derive identifiers only",
                        binding.source_property,
                    )));
                }
                Some(&property.property_type)
            }
        };

        // An indexOnly source proves only what its resolved index
        // carries, so the property must sit on that index.
        if source_is_index_only_query {
            let carries = |index: &dpp::data_contract::document_type::Index| {
                index.terminal.as_deref() == Some(binding.source_property.as_str())
                    || index
                        .properties
                        .iter()
                        .any(|property| property.name == binding.source_property)
            };
            let (carried, index_name) = match binding.source {
                BindingSource::Page => {
                    let index = self.page.index_only_query_index(platform_version)?;
                    (carries(index), index.name.clone())
                }
                BindingSource::SubQuery(source_index) => {
                    let source = &self.sub_queries[source_index];
                    let shape = self.sub_query_document_query(
                        source,
                        &[Identifier::default()],
                        platform_version,
                    )?;
                    let index = shape.index_only_query_index(platform_version)?;
                    (carries(index), index.name.clone())
                }
            };
            if !carried {
                return Err(label(&format!(
                    "the indexOnly source resolves to index \"{}\", which does not carry the \
                     source property \"{}\"",
                    index_name, binding.source_property,
                )));
            }
        }

        if sub_query
            .where_clauses
            .iter()
            .any(|clause| clause.field == binding.field)
        {
            return Err(label(&format!(
                "the fixed clauses may not name the bound field \"{}\"; its `IN` clause is \
                 derived",
                binding.field,
            )));
        }

        match sub_query.kind {
            SubQueryKind::Documents if sub_query.is_by_id_join() => {
                if sub_query.document_type.index_only() {
                    return Err(label(
                        "a by-id join cannot target an indexOnly type: there is no \
                         primary-key tree to fetch from",
                    ));
                }
                if sub_query.limit.is_some() {
                    return Err(label(
                        "a by-id join takes no limit: every derived id must resolve, so \
                         completeness is set equality, not a page",
                    ));
                }
                if !sub_query.order_by.is_empty() {
                    return Err(label(
                        "a by-id join takes no ordering: results follow the derived ids' \
                         first appearance",
                    ));
                }
                // Only a permanentDocument reference guarantees every
                // derived id resolves, which is what lets a missing
                // document be an invalid proof instead of an absence.
                match source_property_type {
                    Some(DocumentPropertyType::IdentifierWithReference(
                        DocumentPropertyReferenceTarget::PermanentDocument {
                            contract_id,
                            document_type_name,
                            ..
                        },
                    )) => {
                        let referenced_contract =
                            contract_id.unwrap_or_else(|| source_contract.id());
                        if referenced_contract != sub_query.contract.id()
                            || document_type_name != sub_query.document_type.name()
                        {
                            return Err(label(&format!(
                                "the source property's refersTo targets \"{}\", not this \
                                 sub-query's type \"{}\"",
                                document_type_name,
                                sub_query.document_type.name(),
                            )));
                        }
                    }
                    _ => {
                        return Err(label(&format!(
                            "a by-id join needs a source property declaring `refersTo: \
                             permanentDocument` (\"{}\" does not): only a permanent-document \
                             reference guarantees every derived id resolves",
                            binding.source_property,
                        )));
                    }
                }
            }
            SubQueryKind::Documents => {
                // Must lower to a path query with a representative value.
                let shape = self.sub_query_document_query(
                    sub_query,
                    &[Identifier::default()],
                    platform_version,
                )?;
                shape.construct_path_query(None, platform_version)?;
                if sub_query.document_type.index_only() {
                    // The lookup's own field must be provable positionally:
                    // the resolved index has to carry it.
                    let index = shape.index_only_query_index(platform_version)?;
                    let carried = index.terminal.as_deref() == Some(binding.field.as_str())
                        || index
                            .properties
                            .iter()
                            .any(|property| property.name == binding.field);
                    if !carried {
                        return Err(label(&format!(
                            "the indexOnly lookup resolves to index \"{}\", which does not \
                             carry the bound field \"{}\"",
                            index.name, binding.field,
                        )));
                    }
                }
                // A lookup whose rows are bounded by its values (at most
                // one per derived value) carries no limit: the values
                // are the bound, and a limit it does not need is exactly
                // what would keep it from merging with another lookup on
                // the same index. Anything else needs one, to bound the
                // walk under each value.
                let value_bounded =
                    self.lookup_is_value_bounded(sub_query, &shape, platform_version)?;
                match (value_bounded, sub_query.limit) {
                    (true, Some(_)) => {
                        return Err(label(
                            "a value-bounded lookup (a unique index, or an indexOnly terminal \
                             with every prefix fixed, yields at most one row per derived \
                             value) takes no limit",
                        ));
                    }
                    (false, None) => {
                        return Err(label(
                            "a documents lookup on a non-unique index requires an explicit \
                             limit: it bounds the walk under each derived value",
                        ));
                    }
                    (false, Some(limit)) if limit as usize > MAX_BOUND_VALUES => {
                        return Err(label(&format!(
                            "limit {} exceeds {}",
                            limit, MAX_BOUND_VALUES
                        )));
                    }
                    _ => {}
                }
            }
            SubQueryKind::Count => {
                if sub_query.limit.is_some() {
                    return Err(label("a count sub-query takes no limit"));
                }
                if !sub_query.order_by.is_empty() {
                    return Err(label("a count sub-query takes no ordering"));
                }
                if sub_query.is_by_id_join() {
                    return Err(label(
                        "a count sub-query counts by an indexed property, not by `$id`",
                    ));
                }
                // Must resolve a countable index with a representative value.
                self.sub_query_count_query(sub_query, &[Identifier::default()], platform_version)?
                    .point_lookup_count_path_query(platform_version)?;
            }
        }
        Ok(())
    }

    /// Whether a bound documents lookup yields at most one row per
    /// derived value: on an indexOnly type, when the resolved index's
    /// terminal is the bound field and every prefix property is fixed
    /// by an equality (entries are unique per full index path); on a
    /// stored type, when a `unique` index's properties are exactly the
    /// fixed equality fields plus the bound field.
    fn lookup_is_value_bounded(
        &self,
        sub_query: &DriveSubQuery<'a>,
        shape: &DriveDocumentQuery<'a>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let Some(binding) = &sub_query.binding else {
            return Ok(false);
        };
        let fixed_equalities: BTreeSet<&str> = sub_query
            .where_clauses
            .iter()
            .filter(|clause| clause.operator == WhereOperator::Equal)
            .map(|clause| clause.field.as_str())
            .collect();
        if sub_query.document_type.index_only() {
            let index = shape.index_only_query_index(platform_version)?;
            let terminal_is_bound = index.terminal.as_deref() == Some(binding.field.as_str());
            let prefix_fixed = index
                .properties
                .iter()
                .all(|property| fixed_equalities.contains(property.name.as_str()));
            return Ok(terminal_is_bound && prefix_fixed);
        }
        let mut wanted: BTreeSet<&str> = fixed_equalities.clone();
        wanted.insert(binding.field.as_str());
        Ok(sub_query.document_type.indexes().values().any(|index| {
            index.unique
                && index.properties.len() == wanted.len()
                && index
                    .properties
                    .iter()
                    .all(|property| wanted.contains(property.name.as_str()))
        }))
    }

    /// Whether the page is a primary-key fetch (`$id IN` / `$id ==`).
    fn page_is_by_ids(&self) -> bool {
        self.page.internal_clauses.primary_key_in_clause.is_some()
            || self
                .page
                .internal_clauses
                .primary_key_equal_clause
                .is_some()
    }

    /// The page's path query as the proof covers it. A by-ids page is
    /// built WITHOUT its limit: its ids already bound it, and grovedb
    /// cannot lift a limit off a query that lands at the merged root
    /// (which a by-ids page shares with a join on the same type). Every
    /// other page keeps its limit, lifted into its branch on merge.
    pub fn page_path_query(&self, platform_version: &PlatformVersion) -> Result<PathQuery, Error> {
        if self.page_is_by_ids() {
            let mut unlimited = self.page.clone();
            unlimited.limit = None;
            return unlimited.construct_path_query(None, platform_version);
        }
        self.page.construct_path_query(None, platform_version)
    }

    /// Every component lands its proved entries under its path query's
    /// base path; entries are routed back to components by the longest
    /// matching base path, and within one base path by what they are
    /// (count trees go to count sub-queries, items to documents ones)
    /// and by bound-value membership. That routing needs two things the
    /// shapes must guarantee up front: no limited component may land at
    /// the merged root (grovedb has no branch to lift its limit into),
    /// and documents components sharing a base path must be tellable
    /// apart by their derived values — so a sibling, which has none,
    /// stays alone, and a page only shares the primary tree with joins
    /// when it is itself a by-ids fetch.
    fn validate_component_paths(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        let representative = [Identifier::default()];
        let mut components: Vec<(Vec<Vec<u8>>, Component, bool)> = Vec::new();
        let page = self.page_path_query(platform_version)?;
        components.push((page.path, Component::Page, page.query.limit.is_some()));
        for (index, sub_query) in self.sub_queries.iter().enumerate() {
            let path_query =
                self.sub_query_path_query(sub_query, &representative, platform_version)?;
            components.push((
                path_query.path,
                Component::Sub(index),
                path_query.query.limit.is_some(),
            ));
        }

        let merged_root: Vec<Vec<u8>> =
            components
                .iter()
                .skip(1)
                .fold(components[0].0.clone(), |common, (path, _, _)| {
                    common
                        .iter()
                        .zip(path)
                        .take_while(|(a, b)| a == b)
                        .map(|(a, _)| a.clone())
                        .collect()
                });
        for (path, component, limited) in &components {
            if *limited && *path == merged_root {
                return Err(unsupported(format!(
                    "{} carries a limit and lands at the merged root of the composite proof, \
                     where grovedb has no branch to lift the limit into; give it a clause that \
                     narrows its path, or split it into a separate request",
                    match component {
                        Component::Page => "the page".to_string(),
                        Component::Sub(index) => format!("sub-query {}", index),
                    }
                )));
            }
        }

        let mut groups: BTreeMap<&Vec<Vec<u8>>, Vec<Component>> = BTreeMap::new();
        for (path, component, _) in &components {
            groups.entry(path).or_default().push(*component);
        }
        for members in groups.values() {
            let documents_members: Vec<Component> = members
                .iter()
                .copied()
                .filter(|component| match component {
                    Component::Page => true,
                    Component::Sub(index) => {
                        self.sub_queries[*index].kind == SubQueryKind::Documents
                    }
                })
                .collect();
            let has_count_member = members.iter().any(|component| {
                matches!(component, Component::Sub(index) if self.sub_queries[*index].kind == SubQueryKind::Count)
            });
            // A count reads an index's value trees themselves; a documents
            // component on the same index descends past them to the rows.
            // One tree node cannot serve both selections in one proof.
            if has_count_member && !documents_members.is_empty() {
                return Err(unsupported(
                    "a count sub-query shares its index path with a documents component: \
                     the count reads the index's value trees themselves while the documents \
                     query descends past them, and one proof cannot serve both; count on \
                     another index, or split them into separate requests"
                        .to_string(),
                ));
            }
            if documents_members.len() < 2 {
                continue;
            }
            let has_sibling = documents_members.iter().any(|component| {
                matches!(component, Component::Sub(index) if self.sub_queries[*index].binding.is_none())
            });
            let has_page = documents_members.contains(&Component::Page);
            let all_subs_are_joins = documents_members.iter().all(|component| match component {
                Component::Page => true,
                Component::Sub(index) => self.sub_queries[*index].is_by_id_join(),
            });
            if has_sibling || (has_page && !(self.page_is_by_ids() && all_subs_are_joins)) {
                return Err(unsupported(
                    "two documents components of the composite query address the same index \
                     path and cannot be told apart by their derived values (a sibling, or a \
                     page that is not a by-ids fetch, shares a path with another component); \
                     split them into separate requests"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Extracts a binding's values from its source documents in their
    /// order, deduplicated to first appearance. ONE extraction both the
    /// server and the verifier run — the single-builder rule that keeps
    /// every derived sub-query identical on both sides.
    pub fn derive_values(
        &self,
        binding: &SubQueryBinding,
        source_documents: &[Document],
    ) -> Result<DerivedValues, Error> {
        let mut seen: BTreeSet<Identifier> = BTreeSet::new();
        let mut values = Vec::new();
        for document in source_documents {
            if let Some(value) = document_bound_value(document, &binding.source_property)? {
                if seen.insert(value) {
                    values.push(value);
                }
            }
        }
        if values.len() > MAX_BOUND_VALUES {
            return Err(unsupported(format!(
                "{} derived values exceed the {} a derived `IN` clause admits",
                values.len(),
                MAX_BOUND_VALUES,
            )));
        }
        Ok(values)
    }

    /// The concrete documents query of a sub-query for `values`: the
    /// fixed clauses plus the derived `IN`, or a pure by-ids fetch for
    /// a join. A sibling ignores `values`.
    pub fn sub_query_document_query(
        &self,
        sub_query: &DriveSubQuery<'a>,
        values: &[Identifier],
        platform_version: &PlatformVersion,
    ) -> Result<DriveDocumentQuery<'a>, Error> {
        let ids = sorted_values(values);
        let in_value = || {
            Value::Array(
                ids.iter()
                    .map(|id| Value::Identifier(id.to_buffer()))
                    .collect(),
            )
        };

        if sub_query.is_by_id_join() {
            return Ok(DriveDocumentQuery {
                contract: sub_query.contract,
                document_type: sub_query.document_type,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: Some(WhereClause {
                        field: dpp::document::property_names::ID.to_string(),
                        operator: WhereOperator::In,
                        value: in_value(),
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
            });
        }

        let mut clauses = sub_query.where_clauses.clone();
        let mut order_by: indexmap::IndexMap<String, OrderClause> = sub_query
            .order_by
            .iter()
            .map(|clause| (clause.field.clone(), clause.clone()))
            .collect();
        if let Some(binding) = &sub_query.binding {
            clauses.push(WhereClause {
                field: binding.field.clone(),
                operator: WhereOperator::In,
                value: in_value(),
            });
            // An `IN` on a secondary index orders by the bound field;
            // supply the ordering when the caller did not, so the
            // request stays minimal and both sides build the same query.
            if !order_by.contains_key(&binding.field) {
                order_by.insert(
                    binding.field.clone(),
                    OrderClause {
                        field: binding.field.clone(),
                        ascending: true,
                    },
                );
            }
        }
        Ok(DriveDocumentQuery {
            contract: sub_query.contract,
            document_type: sub_query.document_type,
            internal_clauses: InternalClauses::extract_from_clauses(clauses, platform_version)?,
            offset: None,
            limit: sub_query.limit,
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: Vec::new(),
        })
    }

    /// The concrete count query of a bound count sub-query for `values`.
    /// Borrows the covering index through `sub_query`, so the count query
    /// lives as long as that reference.
    pub fn sub_query_count_query<'b>(
        &'b self,
        sub_query: &'b DriveSubQuery<'a>,
        values: &[Identifier],
        platform_version: &PlatformVersion,
    ) -> Result<DriveDocumentCountQuery<'b>, Error> {
        let Some(binding) = &sub_query.binding else {
            return Err(unsupported("a count sub-query must be bound".to_string()));
        };
        let mut where_clauses = sub_query.where_clauses.clone();
        where_clauses.push(WhereClause {
            field: binding.field.clone(),
            operator: WhereOperator::In,
            value: Value::Array(
                sorted_values(values)
                    .into_iter()
                    .map(|id| Value::Identifier(id.to_buffer()))
                    .collect(),
            ),
        });
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            sub_query.document_type.indexes(),
            &where_clauses,
            &[],
        )
        .ok_or_else(|| {
            unsupported(format!(
                "count sub-query on \"{}\" needs a `countable: true` index covering its fixed \
                 clauses and the bound field \"{}\"",
                sub_query.document_type.name(),
                binding.field,
            ))
        })?;
        let _ = platform_version;
        Ok(DriveDocumentCountQuery {
            document_type: sub_query.document_type,
            contract_id: sub_query.contract.id().to_buffer(),
            document_type_name: sub_query.document_type.name().to_string(),
            index,
            where_clauses,
        })
    }

    /// The path query of one sub-query for `values`.
    pub fn sub_query_path_query(
        &self,
        sub_query: &DriveSubQuery<'a>,
        values: &[Identifier],
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match sub_query.kind {
            SubQueryKind::Documents => self
                .sub_query_document_query(sub_query, values, platform_version)?
                .construct_path_query(None, platform_version),
            SubQueryKind::Count => self
                .sub_query_count_query(sub_query, values, platform_version)?
                .point_lookup_count_path_query(platform_version),
        }
    }

    /// The component path queries the merged proof covers, in component
    /// order: the page, then one entry per sub-query — `None` for a
    /// bound sub-query whose binding derived nothing (it has no branch).
    /// Every sub-query walks in the page's direction: grovedb's merge
    /// requires all inputs to agree, and a key-addressed sub-query means
    /// the same set either way. ONE builder both the prover
    /// (`prove_query_many`) and the verifier (`PathQuery::merge`) call,
    /// so the merged query is byte-identical on both sides.
    pub fn proof_path_queries(
        &self,
        derived: &[DerivedValues],
        platform_version: &PlatformVersion,
    ) -> Result<(PathQuery, Vec<Option<PathQuery>>), Error> {
        if derived.len() != self.sub_queries.len() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "one derived value list per sub-query",
            )));
        }
        let page = self.page_path_query(platform_version)?;
        let direction = page.query.query.left_to_right;
        let mut sub_path_queries = Vec::with_capacity(self.sub_queries.len());
        for (sub_query, values) in self.sub_queries.iter().zip(derived) {
            if sub_query.binding.is_some() && values.is_empty() {
                sub_path_queries.push(None);
                continue;
            }
            let mut path_query = self.sub_query_path_query(sub_query, values, platform_version)?;
            path_query.query.query.left_to_right = direction;
            sub_path_queries.push(Some(path_query));
        }
        Ok((page, sub_path_queries))
    }

    /// Merges the component path queries into the one query the proof
    /// covers.
    pub fn merged_path_query(
        page: &PathQuery,
        sub_path_queries: &[Option<PathQuery>],
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let mut components: Vec<&PathQuery> = vec![page];
        components.extend(sub_path_queries.iter().flatten());
        if components.len() == 1 {
            return Ok(page.clone());
        }
        PathQuery::merge(components, &platform_version.drive.grove_version).map_err(Error::from)
    }

    /// Decodes the proved entries of a documents component: stored
    /// documents from item elements, indexOnly projections synthesized
    /// from their proved positions.
    pub(crate) fn decode_document_trios(
        query: &DriveDocumentQuery<'a>,
        trios: Vec<PresentTrio>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        if query.document_type.index_only() {
            let index = query.index_only_query_index(platform_version)?;
            return trios
                .into_iter()
                .map(|(path, key, _)| {
                    synthesize_index_only_document(
                        query.contract.id(),
                        query.document_type,
                        index,
                        &path,
                        &key,
                    )
                })
                .collect();
        }
        trios
            .into_iter()
            .map(|(_, _, element)| {
                let serialized = element.into_item_bytes().map_err(Error::from)?;
                Document::from_bytes(serialized.as_slice(), query.document_type, platform_version)
                    .map_err(|e| Error::Protocol(Box::new(e)))
            })
            .collect()
    }

    /// Decodes the proved entries of a count component: one entry per
    /// count tree, keyed by the `IN` value — which sits one segment
    /// past the base path when the walk descended through trailing
    /// equalities, and IS the key otherwise (the same layout
    /// `verify_point_lookup_count_proof` reads).
    fn decode_count_trios(base_path_len: usize, trios: Vec<PresentTrio>) -> Vec<SplitCountEntry> {
        trios
            .into_iter()
            .map(|(path, key, element)| {
                let key = if path.len() > base_path_len {
                    path[base_path_len].clone()
                } else {
                    key
                };
                SplitCountEntry {
                    in_key: None,
                    key,
                    count: Some(element.count_value_or_default()),
                }
            })
            .collect()
    }

    /// Assembles one sub-query's result from the decoded items routed to
    /// its group, keeping only the items its derived values admit and,
    /// for a by-id join, enforcing exact set equality in first-appearance
    /// order. Shared by the server (where a violation is corrupted
    /// state) and the verifier (where it is an invalid proof).
    fn assemble_sub_result(
        &self,
        sub_query: &DriveSubQuery<'a>,
        values: &[Identifier],
        items: &DecodedItems,
    ) -> Result<SubQueryResult, Error> {
        let admitted: BTreeSet<Identifier> = values.iter().copied().collect();
        match (sub_query.kind, items) {
            (SubQueryKind::Documents, DecodedItems::Documents(documents)) => {
                let Some(binding) = &sub_query.binding else {
                    return Ok(SubQueryResult::Documents(documents.clone()));
                };
                if sub_query.is_by_id_join() {
                    let mut by_id: BTreeMap<Identifier, Document> = BTreeMap::new();
                    for document in documents {
                        let id = document.id();
                        if !admitted.contains(&id) {
                            // Another join on the same type owns it.
                            continue;
                        }
                        if by_id.insert(id, document.clone()).is_some() {
                            return Err(corrupted_proof(format!(
                                "composite join results carry document {} twice",
                                id
                            )));
                        }
                    }
                    let mut ordered = Vec::with_capacity(values.len());
                    for value in values {
                        let document = by_id.remove(value).ok_or_else(|| {
                            corrupted_proof(format!(
                                "composite join results are missing referenced document {}: \
                                 a permanentDocument reference cannot dangle, so the proof \
                                 does not cover the derived query",
                                value
                            ))
                        })?;
                        ordered.push(document);
                    }
                    return Ok(SubQueryResult::Documents(ordered));
                }
                let mut mine = Vec::new();
                for document in documents {
                    match document_bound_value(document, &binding.field)? {
                        Some(value) if admitted.contains(&value) => mine.push(document.clone()),
                        _ => {}
                    }
                }
                Ok(SubQueryResult::Documents(mine))
            }
            (SubQueryKind::Count, DecodedItems::Counts(entries)) => {
                let mut mine = Vec::new();
                for entry in entries {
                    let Ok(value) = Identifier::from_bytes(&entry.key) else {
                        return Err(corrupted_proof(
                            "a composite count entry is keyed by something other than an \
                             identifier"
                                .to_string(),
                        ));
                    };
                    if admitted.contains(&value) {
                        mine.push(entry.clone());
                    }
                }
                Ok(SubQueryResult::Counts(mine))
            }
            _ => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a component group decoded into the wrong kind of items",
            ))),
        }
    }

    /// Routes the proved trios of the merged query back to the page and
    /// the sub-queries, decodes each group, and assembles every
    /// component's result. Every trio must land in a component, and
    /// every decoded item must be claimed by one — an entry the
    /// derivation never asked for means the responding node steered the
    /// composition.
    pub(crate) fn assemble_from_trios(
        &self,
        derived: &[DerivedValues],
        page_path_query: &PathQuery,
        sub_path_queries: &[Option<PathQuery>],
        trios: Vec<ProvedTrio>,
        platform_version: &PlatformVersion,
    ) -> Result<CompositeDocumentsResult, Error> {
        // Group components by base path.
        let mut groups: Vec<(Vec<Vec<u8>>, Vec<Component>)> = Vec::new();
        let mut register = |path: &Vec<Vec<u8>>, component: Component| {
            if let Some((_, members)) = groups.iter_mut().find(|(p, _)| p == path) {
                members.push(component);
            } else {
                groups.push((path.clone(), vec![component]));
            }
        };
        register(&page_path_query.path, Component::Page);
        for (index, path_query) in sub_path_queries.iter().enumerate() {
            if let Some(path_query) = path_query {
                register(&path_query.path, Component::Sub(index));
            }
        }

        // Distribute trios to the group with the longest matching base path.
        let mut trios_by_group: Vec<Vec<PresentTrio>> = vec![Vec::new(); groups.len()];
        for (path, key, element) in trios {
            let Some(element) = element else {
                continue;
            };
            let best = groups
                .iter()
                .enumerate()
                .filter(|(_, (base, _))| path.starts_with(base))
                .max_by_key(|(_, (base, _))| base.len())
                .map(|(index, _)| index)
                .ok_or_else(|| {
                    corrupted_proof(
                        "the composite proof proved an entry outside every component's \
                         subtree"
                            .to_string(),
                    )
                })?;
            trios_by_group[best].push((path, key, element));
        }

        // Decode each group once — items are documents, everything else
        // is a count tree — then let every member claim its share.
        let mut page_documents: Option<Vec<Document>> = None;
        let mut sub_results: Vec<Option<SubQueryResult>> = vec![None; self.sub_queries.len()];
        for ((base_path, members), group_trios) in groups.iter().zip(trios_by_group) {
            let (document_trios, count_trios): (Vec<_>, Vec<_>) = group_trios
                .into_iter()
                .partition(|(_, _, element)| matches!(element, Element::Item(..)));
            let documents_members: Vec<Component> = members
                .iter()
                .copied()
                .filter(|component| match component {
                    Component::Page => true,
                    Component::Sub(index) => {
                        self.sub_queries[*index].kind == SubQueryKind::Documents
                    }
                })
                .collect();
            let count_members: Vec<usize> = members
                .iter()
                .filter_map(|component| match component {
                    Component::Sub(index)
                        if self.sub_queries[*index].kind == SubQueryKind::Count =>
                    {
                        Some(*index)
                    }
                    _ => None,
                })
                .collect();
            if !document_trios.is_empty() && documents_members.is_empty() {
                return Err(corrupted_proof(
                    "the composite proof proved documents under a path only counts were \
                     asked of"
                        .to_string(),
                ));
            }
            if !count_trios.is_empty() && count_members.is_empty() {
                return Err(corrupted_proof(
                    "the composite proof proved count trees under a path only documents \
                     were asked of"
                        .to_string(),
                ));
            }

            if !documents_members.is_empty() {
                // Every documents member of a group addresses the same
                // type, so any member's query decodes the group.
                let documents = match documents_members[0] {
                    Component::Page => {
                        Self::decode_document_trios(&self.page, document_trios, platform_version)?
                    }
                    Component::Sub(index) => {
                        let query = self.sub_query_document_query(
                            &self.sub_queries[index],
                            &derived[index],
                            platform_version,
                        )?;
                        Self::decode_document_trios(&query, document_trios, platform_version)?
                    }
                };
                let decoded = DecodedItems::Documents(documents.clone());
                let mut claimed: BTreeSet<usize> = BTreeSet::new();
                for member in &documents_members {
                    match member {
                        Component::Page => {
                            let page_ids: Option<BTreeSet<Identifier>> =
                                if documents_members.len() > 1 {
                                    Some(self.page_ids()?)
                                } else {
                                    None
                                };
                            let mut mine = Vec::new();
                            for (position, document) in documents.iter().enumerate() {
                                let is_mine = page_ids
                                    .as_ref()
                                    .is_none_or(|ids| ids.contains(&document.id()));
                                if is_mine {
                                    claimed.insert(position);
                                    mine.push(document.clone());
                                }
                            }
                            page_documents = Some(mine);
                        }
                        Component::Sub(index) => {
                            let sub_query = &self.sub_queries[*index];
                            let result =
                                self.assemble_sub_result(sub_query, &derived[*index], &decoded)?;
                            let mine_ids: BTreeSet<Identifier> = result
                                .documents()
                                .iter()
                                .map(|document| document.id())
                                .collect();
                            for (position, document) in documents.iter().enumerate() {
                                if mine_ids.contains(&document.id()) {
                                    claimed.insert(position);
                                }
                            }
                            sub_results[*index] = Some(result);
                        }
                    }
                }
                if claimed.len() != documents.len() {
                    return Err(corrupted_proof(
                        "the composite proof carries a document that no component's \
                         derivation asked for"
                            .to_string(),
                    ));
                }
            }

            if !count_members.is_empty() {
                let entries = Self::decode_count_trios(base_path.len(), count_trios);
                let decoded = DecodedItems::Counts(entries.clone());
                let mut claimed: BTreeSet<usize> = BTreeSet::new();
                for index in count_members {
                    let sub_query = &self.sub_queries[index];
                    let result = self.assemble_sub_result(sub_query, &derived[index], &decoded)?;
                    let mine_keys: BTreeSet<&Vec<u8>> =
                        result.counts().iter().map(|entry| &entry.key).collect();
                    for (position, entry) in entries.iter().enumerate() {
                        if mine_keys.contains(&entry.key) {
                            claimed.insert(position);
                        }
                    }
                    sub_results[index] = Some(result);
                }
                if claimed.len() != entries.len() {
                    return Err(corrupted_proof(
                        "the composite proof carries a count that no component's derivation \
                         asked for"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(CompositeDocumentsResult {
            page_documents: page_documents.unwrap_or_default(),
            sub_results: sub_results
                .into_iter()
                .zip(&self.sub_queries)
                .map(|(result, sub_query)| {
                    result.unwrap_or_else(|| match sub_query.kind {
                        SubQueryKind::Documents => SubQueryResult::Documents(Vec::new()),
                        SubQueryKind::Count => SubQueryResult::Counts(Vec::new()),
                    })
                })
                .collect(),
        })
    }

    /// The ids a by-ids page addresses (its `$id IN` / `$id ==` clause),
    /// used to tell the page's documents from a join's when they share
    /// the primary tree.
    fn page_ids(&self) -> Result<BTreeSet<Identifier>, Error> {
        let mut ids = BTreeSet::new();
        if let Some(clause) = &self.page.internal_clauses.primary_key_equal_clause {
            ids.insert(clause.value.to_identifier().map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "a primary-key equality clause holds an identifier",
                ))
            })?);
        }
        if let Some(clause) = &self.page.internal_clauses.primary_key_in_clause {
            for value in clause
                .in_values()
                .into_data()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "a primary-key in clause holds an array",
                    ))
                })?
                .iter()
            {
                ids.insert(value.to_identifier().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "a primary-key in clause holds identifiers",
                    ))
                })?);
            }
        }
        Ok(ids)
    }

    /// Derives every sub-query's values from the (materialized or
    /// proven) page and earlier sub-query documents, in request order.
    pub fn derive_all(
        &self,
        page_documents: &[Document],
        sub_documents: &dyn Fn(usize) -> Option<Vec<Document>>,
    ) -> Result<Vec<DerivedValues>, Error> {
        let mut derived: Vec<DerivedValues> = Vec::with_capacity(self.sub_queries.len());
        for sub_query in &self.sub_queries {
            let Some(binding) = &sub_query.binding else {
                derived.push(Vec::new());
                continue;
            };
            let values = match binding.source {
                BindingSource::Page => self.derive_values(binding, page_documents)?,
                BindingSource::SubQuery(source_index) => {
                    let documents = sub_documents(source_index).ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "a binding's source sub-query was not materialized before it",
                        ))
                    })?;
                    self.derive_values(binding, &documents)?
                }
            };
            derived.push(values);
        }
        Ok(derived)
    }

    /// Whether a sub-query's documents feed a later binding.
    pub(crate) fn is_binding_source(&self, index: usize) -> bool {
        self.sub_queries.iter().any(|sub_query| {
            matches!(
                sub_query.binding,
                Some(SubQueryBinding {
                    source: BindingSource::SubQuery(source),
                    ..
                }) if source == index
            )
        })
    }
}

#[cfg(feature = "server")]
impl<'a> DriveCompositeDocumentQuery<'a> {
    /// Materializes the page without a proof.
    fn materialize_page(
        &self,
        drive: &crate::drive::Drive,
        transaction: grovedb::TransactionArg,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        Self::materialize_documents(
            &self.page,
            drive,
            transaction,
            drive_operations,
            platform_version,
        )
    }

    /// Materializes a documents query without a proof: indexOnly
    /// projections are synthesized, stored documents deserialized.
    fn materialize_documents(
        query: &DriveDocumentQuery<'a>,
        drive: &crate::drive::Drive,
        transaction: grovedb::TransactionArg,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        if query.document_type.index_only() {
            let (documents, _skipped) = query.execute_index_only_documents_no_proof_internal(
                drive,
                transaction,
                drive_operations,
                platform_version,
            )?;
            return Ok(documents);
        }
        let (serialized, _skipped) = query.execute_raw_results_no_proof_internal(
            drive,
            transaction,
            drive_operations,
            platform_version,
        )?;
        serialized
            .into_iter()
            .map(|bytes| {
                Document::from_bytes(bytes.as_slice(), query.document_type, platform_version)
                    .map_err(|e| Error::Protocol(Box::new(e)))
            })
            .collect()
    }

    /// Materializes one sub-query's result without a proof.
    fn materialize_sub_result(
        &self,
        sub_query: &DriveSubQuery<'a>,
        values: &[Identifier],
        drive: &crate::drive::Drive,
        transaction: grovedb::TransactionArg,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<SubQueryResult, Error> {
        use grovedb::query_result_type::{QueryResultElement, QueryResultType};

        if sub_query.binding.is_some() && values.is_empty() {
            return Ok(match sub_query.kind {
                SubQueryKind::Documents => SubQueryResult::Documents(Vec::new()),
                SubQueryKind::Count => SubQueryResult::Counts(Vec::new()),
            });
        }
        match sub_query.kind {
            SubQueryKind::Documents => {
                let query = self.sub_query_document_query(sub_query, values, platform_version)?;
                let documents = Self::materialize_documents(
                    &query,
                    drive,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;
                self.assemble_sub_result(sub_query, values, &DecodedItems::Documents(documents))
            }
            SubQueryKind::Count => {
                let path_query = self
                    .sub_query_count_query(sub_query, values, platform_version)?
                    .point_lookup_count_path_query(platform_version)?;
                let base_path_len = path_query.path.len();
                let (results, _skipped) = match drive.grove_get_path_query(
                    &path_query,
                    transaction,
                    QueryResultType::QueryPathKeyElementTrioResultType,
                    drive_operations,
                    &platform_version.drive,
                ) {
                    // No count tree yet under this index: every count is zero.
                    Err(Error::GroveDB(e))
                        if matches!(
                            e.as_ref(),
                            grovedb::Error::PathKeyNotFound(_)
                                | grovedb::Error::PathNotFound(_)
                                | grovedb::Error::PathParentLayerNotFound(_)
                        ) =>
                    {
                        return Ok(SubQueryResult::Counts(Vec::new()));
                    }
                    other => other?,
                };
                let trios = results
                    .elements
                    .into_iter()
                    .filter_map(|element| match element {
                        QueryResultElement::PathKeyElementTrioResultItem(trio) => Some(trio),
                        _ => None,
                    })
                    .collect();
                let entries = Self::decode_count_trios(base_path_len, trios);
                self.assemble_sub_result(sub_query, values, &DecodedItems::Counts(entries))
            }
        }
    }

    /// Executes the composite query without proofs.
    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &crate::drive::Drive,
        transaction: grovedb::TransactionArg,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<CompositeDocumentsResult, Error> {
        self.validate(platform_version)?;

        let page_documents =
            self.materialize_page(drive, transaction, drive_operations, platform_version)?;
        let mut sub_results: Vec<SubQueryResult> = Vec::with_capacity(self.sub_queries.len());
        for sub_query in &self.sub_queries {
            let values = match &sub_query.binding {
                None => Vec::new(),
                Some(binding) => match binding.source {
                    BindingSource::Page => self.derive_values(binding, &page_documents)?,
                    BindingSource::SubQuery(source) => {
                        self.derive_values(binding, sub_results[source].documents())?
                    }
                },
            };
            sub_results.push(self.materialize_sub_result(
                sub_query,
                &values,
                drive,
                transaction,
                drive_operations,
                platform_version,
            )?);
        }
        Ok(CompositeDocumentsResult {
            page_documents,
            sub_results,
        })
    }

    /// Executes the composite query AND generates its single merged
    /// proof.
    ///
    /// The page (and every sub-query that feeds a later binding) is
    /// materialized so the sub-queries can be derived; then
    /// [`Self::proof_path_queries`] builds the component path queries
    /// and `prove_query_many` merges them — one proof, one root by
    /// construction. Grovedb proves committed state only, so the
    /// materialize/prove sequence is bracketed by root-hash reads and
    /// retried if a block commit interleaved (otherwise the proof's page
    /// branch could disagree with the sub-queries derived from a stale
    /// materialization and every verifier would reject it).
    ///
    /// Returns the proof and the materialized page (the caller's
    /// pagination cursor derives from it); the sub-query results are
    /// covered by the proof and not materialized twice.
    pub(crate) fn execute_with_proof_internal(
        &self,
        drive: &crate::drive::Drive,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, Vec<Document>), Error> {
        self.validate(platform_version)?;

        const MAX_ATTEMPTS: usize = 3;
        for _ in 0..MAX_ATTEMPTS {
            let root_before = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()?;

            let page_documents =
                self.materialize_page(drive, None, drive_operations, platform_version)?;
            // Sub-queries that feed later bindings are materialized in
            // order; everything else is only derived.
            let mut derived: Vec<DerivedValues> = Vec::with_capacity(self.sub_queries.len());
            let mut materialized: Vec<Option<Vec<Document>>> = vec![None; self.sub_queries.len()];
            for (index, sub_query) in self.sub_queries.iter().enumerate() {
                let values = match &sub_query.binding {
                    None => Vec::new(),
                    Some(binding) => match binding.source {
                        BindingSource::Page => self.derive_values(binding, &page_documents)?,
                        BindingSource::SubQuery(source) => {
                            let documents = materialized[source].as_deref().ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedCodeExecution(
                                    "a binding's source sub-query was not materialized",
                                ))
                            })?;
                            self.derive_values(binding, documents)?
                        }
                    },
                };
                if self.is_binding_source(index) {
                    let result = self.materialize_sub_result(
                        sub_query,
                        &values,
                        drive,
                        None,
                        drive_operations,
                        platform_version,
                    )?;
                    materialized[index] = Some(result.documents().to_vec());
                }
                derived.push(values);
            }

            let (page_path_query, sub_path_queries) =
                self.proof_path_queries(&derived, platform_version)?;
            let mut components: Vec<&PathQuery> = vec![&page_path_query];
            components.extend(sub_path_queries.iter().flatten());
            let proof = drive
                .grove
                .prove_query_many(components, None, &platform_version.drive.grove_version)
                .unwrap()?;

            let root_after = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()?;
            if root_before != root_after {
                continue;
            }
            return Ok((proof, page_documents));
        }
        Err(Error::Drive(DriveError::NotSupported(
            "composite proof generation raced a block commit on every attempt; transient — \
             retry the request",
        )))
    }
}
