//! Document synthesis for indexOnly queries.
//!
//! An indexOnly entry is `[…prefix values, 0, <terminal value>] → Item(row
//! commitment)` — the row IS its grove position, and the 32-byte payload
//! binds the entry to its document's full tuple (see
//! `index_only_row_commitment`; synthesis itself has no use for it, since
//! the proved position already carries every recoverable field). A query
//! result therefore arrives as a
//! `(path, key, element)` trio whose path segments and member key carry
//! every recoverable field, and this module turns one trio back into a
//! `Document`. It is the single builder BOTH sides call — the server's
//! no-proof execution and the proof verifier — so prover and verifier agree
//! on the synthesized shape by construction (the same one-builder rule the
//! ranked query's `path.rs` follows).
//!
//! Field recovery:
//! * prefix properties — decoded from the value path segments via
//!   [`DocumentPropertyType::decode_value_for_tree_keys`], the inverse of
//!   the key encoding the write path used;
//! * the terminal property — decoded from the member key the same way;
//! * `$ownerId` / `$createdAt` — from whichever position (prefix or
//!   terminal) the index carries them.
//!
//! The synthesized `$id` is deterministic over the synthesized position:
//! `hash_double("index_only_synthesized_id_v1" ‖ contract_id ‖ owner_id ‖
//! frame(doctype) ‖ (frame(name) ‖ frame(key-bytes))*)` in index order —
//! `frame(x) = u32_be(len(x)) ‖ x`, with every non-owner component
//! (`$createdAt` included) participating, so distinct grove positions can
//! never share an id. Nothing on chain is ever addressed by it — a query
//! over a subset index yields a *projection*, and its id is scoped to that
//! projection's content.
//!
//! Fail-closed: any arity or property-name mismatch between the trio and
//! the index the query resolved is an error, never a partial document.

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{index_admissible_for_skip_if_absent, DriveDocumentQuery};
use crate::verify::RootHash;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::btreemap_extensions::BTreeValueMapInsertionPathHelper;
use dpp::platform_value::Value;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

/// A resolved indexOnly terminal route: the matcher's winning index, the
/// clause on its terminal, and — for mixed shapes — the *prefix pivot*: a
/// range or `in` clause sitting on one of the index's prefix properties
/// (by position), with every property above it equality-bound and every
/// property below it unconstrained.
pub(crate) struct IndexOnlyTerminalRoute<'a> {
    /// The index the entries are addressed through.
    pub index: &'a Index,
    /// The clause on the index's terminal property; `None` only for the
    /// first keyset page (order-by on the terminal, no cursor clause yet).
    pub terminal_clause: Option<&'a crate::query::WhereClause>,
    /// `(position, clause)` of a range / `in` clause on a prefix
    /// property. When present the terminal clause is always an equality.
    pub prefix_pivot: Option<(usize, &'a crate::query::WhereClause)>,
}

impl DriveDocumentQuery<'_> {
    /// The terminal-clause route for indexOnly queries.
    ///
    /// An indexOnly entry's member key IS the terminal property's encoded
    /// value, so a clause on the terminal lowers directly onto the final
    /// key level — the shape behind both "did I like X" (equality on the
    /// terminal) and keyset pagination (range on the terminal after the
    /// last seen value, with a limit). Index selection is the SAME dpp
    /// matcher the generic route uses
    /// ([`index_for_types_matching_including_terminal`]), with terminals
    /// as matchable deepest components and difference-scored best-match
    /// semantics — generic matches keep absolute precedence inside the
    /// matcher itself. What remains here is clause-shape validation on
    /// the matcher's winner: every prefix property must carry an equality
    /// clause (the path down to the `0` level must be fully determined),
    /// the terminal carries the one remaining clause, and `orderBy` names
    /// nothing outside the index (a range or `in` on the terminal
    /// requires ordering by it, mirroring the stored-document rule).
    ///
    /// Mixed shapes are served through a *prefix pivot*: one range or
    /// `in` clause may sit on a prefix property instead of the terminal
    /// (`hashtag == h AND postId > p AND $ownerId == me`), provided every
    /// property above the pivot is equality-bound, properties below it
    /// are unconstrained, the terminal clause is an equality, and the
    /// pivot is ordered by (`MissingOrderByForRange` otherwise).
    ///
    /// Returns `Ok(None)` when the matcher finds no terminal-using index
    /// (the generic route's miss error stands), `Ok(Some(..))` with the
    /// resolved route — `terminal_clause: None` only for the first keyset
    /// page, which has no cursor clause yet and scans the member keys in
    /// `orderBy` order — and a targeted error when a terminal index
    /// matched but the clause shape does not hold.
    ///
    /// [`index_for_types_matching_including_terminal`]:
    /// dpp::data_contract::document_type::methods::DocumentTypeV0Methods::index_for_types_matching_including_terminal
    pub(crate) fn index_only_terminal_clause_selection(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IndexOnlyTerminalRoute<'_>>, Error> {
        use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;

        // Shapes the terminal route can never serve opt out up front, so
        // their generic-route miss errors propagate untouched: a resolved
        // time range binds to a bucketed index (whose entries serve only
        // the aggregate surfaces — see
        // `refuse_bucketed_index_only_synthesis`), and the multi-`In`
        // machinery has its own error surface.
        if !self.resolved_time_ranges.is_empty() || self.internal_clauses.in_clauses.len() > 1 {
            return Ok(None);
        }

        // Classified once against the doctype: unless some clause or
        // order-by field actually holds the TERMINAL role, this query has
        // nothing for the terminal route and the generic miss stands —
        // the modeled form of "a clause may sit on a terminal, not only
        // on an index prefix property".
        let clause_roles = self.internal_clauses.classify_fields(self.document_type);
        let names_a_terminal = clause_roles.values().any(|roles| roles.terminal)
            || self.order_by.keys().any(|field| {
                crate::query::InternalClauses::classify_field(self.document_type, field).terminal
            });
        if !names_a_terminal {
            return Ok(None);
        }

        // The same by-role field assembly the generic matcher receives.
        let equal_fields = self
            .internal_clauses
            .equal_clauses
            .keys()
            .map(|field| field.as_str())
            .collect::<Vec<&str>>();
        let range_field = self
            .internal_clauses
            .range_clause
            .as_ref()
            .map(|range_clause| range_clause.field.as_str());
        let in_field = self
            .internal_clauses
            .in_clauses
            .first()
            .map(|in_clause| in_clause.field.as_str());
        let order_by_keys: Vec<&str> = self.order_by.keys().map(String::as_str).collect();

        // The union of every field the query binds, for the skip-index
        // admissibility gate — the by-role slices above are what the
        // matcher consumes.
        let mut bound_fields = equal_fields.clone();
        bound_fields.extend(range_field);
        bound_fields.extend(in_field);
        for order_by_key in &order_by_keys {
            if !bound_fields.contains(order_by_key) {
                bound_fields.push(order_by_key);
            }
        }

        let Some((index, _difference, terminal_used)) = self
            .document_type
            .index_for_types_matching_including_terminal(
                equal_fields.as_slice(),
                range_field,
                in_field,
                order_by_keys.as_slice(),
                // Bucketed indexes never serve the terminal route: only
                // resolved time ranges may bind to bucket keys, and those
                // opted out above — a raw query name-matching a bucketed
                // index's properties must not walk its grid-keyed levels.
                // A skipIfAbsent index additionally requires its trigger
                // bound — it is a sparse projection, and while the
                // contiguous matcher already forces position 0 to be bound
                // whenever any deeper property is used, an all-unused match
                // inside the difference budget could still slip through
                // (see [`index_admissible_for_skip_if_absent`]).
                |index| {
                    index.time_range.is_none()
                        && index_admissible_for_skip_if_absent(index, &bound_fields)
                },
                platform_version,
            )
            .map_err(|e| Error::Protocol(Box::new(e)))?
        else {
            return Ok(None);
        };
        if !terminal_used {
            // A generic cover exists after all — the generic route owns
            // it (unreachable when this runs after a generic miss, since
            // both share one matching algorithm).
            return Ok(None);
        }

        let terminal =
            index
                .terminal
                .as_deref()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a terminal-using match implies an indexOnly index",
                )))?;
        let terminal_clause = self
            .internal_clauses
            .equal_clauses
            .get(terminal)
            .or(match &self.internal_clauses.range_clause {
                Some(range_clause) if range_clause.field == terminal => Some(range_clause),
                _ => None,
            })
            .or_else(|| {
                self.internal_clauses
                    .in_clauses
                    .iter()
                    .find(|in_clause| in_clause.field == terminal)
            });

        let shape_error = |message: &str| {
            Error::Query(crate::error::query::QuerySyntaxError::Unsupported(
                message.to_string(),
            ))
        };

        // The one non-equality, non-terminal clause — the prefix pivot
        // candidate. (Field coverage is the matcher's job; PLACEMENT of
        // non-equality clauses is the shape rule here.)
        let position_of = |field: &str| -> Option<usize> {
            index
                .properties
                .iter()
                .position(|property| property.name == field)
        };
        let mut prefix_pivot: Option<(usize, &crate::query::WhereClause)> = None;
        for clause in self
            .internal_clauses
            .range_clause
            .iter()
            .chain(self.internal_clauses.in_clauses.iter())
        {
            if clause.field == terminal {
                continue;
            }
            let Some(position) = position_of(&clause.field) else {
                // Not on the terminal, not on a prefix property — the
                // matcher could not have covered it. Unreachable.
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a matched terminal route cannot carry a clause outside the index",
                )));
            };
            if prefix_pivot.is_some() {
                return Err(shape_error(
                    "an indexOnly terminal query supports at most one range or `in` \
                     clause on a prefix property (the pivot)",
                ));
            }
            prefix_pivot = Some((position, clause));
        }

        match prefix_pivot {
            None => {
                // Fully determined prefix: every prefix property must
                // carry an equality clause — the path down to the entry
                // level admits no gaps.
                if !index.properties.iter().all(|property| {
                    self.internal_clauses
                        .equal_clauses
                        .contains_key(property.name.as_str())
                }) {
                    return Err(shape_error(
                        "a clause on an indexOnly terminal property requires equality \
                         clauses on ALL of that index's properties (the path to the \
                         entries must be fully determined), with no other clauses and \
                         orderBy limited to the index",
                    ));
                }
                if let Some(terminal_clause) = terminal_clause {
                    if terminal_clause.operator.is_range() && !self.order_by.contains_key(terminal)
                    {
                        return Err(Error::Query(
                            crate::error::query::QuerySyntaxError::MissingOrderByForRange(
                                "a range or `in` clause on an indexOnly terminal property \
                                 requires an orderBy on that property",
                            ),
                        ));
                    }
                }
            }
            Some((pivot_position, pivot_clause)) => {
                // Mixed shape: everything above the pivot equality-bound,
                // everything below it unconstrained, terminal clause an
                // equality, pivot ordered by.
                let terminal_is_equality =
                    self.internal_clauses.equal_clauses.contains_key(terminal);
                if !terminal_is_equality {
                    return Err(shape_error(
                        "a range or `in` clause on an indexOnly prefix property requires \
                         an EQUALITY clause on the terminal: two simultaneous non-equality \
                         levels have no single pagination order",
                    ));
                }
                for (position, property) in index.properties.iter().enumerate() {
                    let has_equality = self
                        .internal_clauses
                        .equal_clauses
                        .contains_key(property.name.as_str());
                    if position < pivot_position && !has_equality {
                        return Err(shape_error(
                            "every prefix property ABOVE a pivot range/`in` clause must \
                             carry an equality clause",
                        ));
                    }
                    if position > pivot_position && has_equality {
                        return Err(shape_error(
                            "prefix properties BELOW a pivot range/`in` clause must be \
                             unconstrained: an equality below the pivot is not yet \
                             supported",
                        ));
                    }
                }
                if !self.order_by.contains_key(pivot_clause.field.as_str()) {
                    return Err(Error::Query(
                        crate::error::query::QuerySyntaxError::MissingOrderByForRange(
                            "a range or `in` clause on an indexOnly prefix property \
                             requires an orderBy on that property",
                        ),
                    ));
                }
            }
        }

        Ok(Some(IndexOnlyTerminalRoute {
            index,
            terminal_clause,
            prefix_pivot,
        }))
    }

    /// Build the path query for a terminal-clause indexOnly query. One
    /// builder for the server's execution, the prover and the verifier.
    ///
    /// Without a pivot: the fully determined prefix path down to the `0`
    /// entry level, with the terminal clause lowered over the member
    /// keys. With a prefix pivot: the path stops at the pivot property,
    /// the pivot clause ranges over its values, and a subquery chain
    /// walks each selected value through the unconstrained properties
    /// below it (`insert_all` per level) down to `0`, where the terminal
    /// equality selects the member key.
    pub(crate) fn index_only_terminal_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        route: &IndexOnlyTerminalRoute<'_>,
        platform_version: &PlatformVersion,
    ) -> Result<grovedb::PathQuery, Error> {
        use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;

        let IndexOnlyTerminalRoute {
            index,
            terminal_clause,
            prefix_pivot,
        } = route;

        let direction_for = |field: &str, fallback: bool| {
            self.order_by
                .get(field)
                .map(|order_clause| order_clause.ascending)
                .unwrap_or(fallback)
        };
        let terminal_query = match terminal_clause {
            Some(terminal_clause) => {
                let left_to_right = if terminal_clause.operator.is_range() {
                    direction_for(terminal_clause.field.as_str(), true)
                } else {
                    true
                };
                terminal_clause.to_path_query(
                    self.document_type,
                    &None,
                    left_to_right,
                    platform_version,
                )?
            }
            // First keyset page: no cursor clause yet — every member key
            // in the terminal's orderBy direction.
            None => {
                let terminal = index.terminal.as_deref().ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution(
                        "terminal-route selection guarantees an indexOnly index",
                    ),
                ))?;
                let mut query = grovedb::Query::new_with_direction(direction_for(terminal, true));
                query.insert_all();
                query
            }
        };

        let mut path = document_type_path;
        let (final_query, path_property_count) = match prefix_pivot {
            None => {
                // Fully determined prefix: the path descends every
                // property's value; the terminal query runs at `0`.
                (terminal_query, index.properties.len())
            }
            Some((pivot_position, pivot_clause)) => {
                // The pivot clause ranges over its property's values;
                // below it, one `insert_all` level per unconstrained
                // property, then `0` and the terminal equality. Built
                // innermost-out.
                let mut chain = terminal_query;
                let mut chain_is_terminal = true;
                for position in ((pivot_position + 1)..index.properties.len()).rev() {
                    let property = &index.properties[position];
                    let mut values_query = grovedb::Query::new_with_direction(direction_for(
                        &property.name,
                        property.ascending,
                    ));
                    values_query.insert_all();
                    if chain_is_terminal {
                        values_query.set_subquery_key(vec![0]);
                    } else {
                        values_query.set_subquery_key(
                            index.properties[position + 1].name.as_bytes().to_vec(),
                        );
                    }
                    values_query.set_subquery(chain);
                    chain = values_query;
                    chain_is_terminal = false;
                }

                let mut pivot_query = pivot_clause.to_path_query(
                    self.document_type,
                    &None,
                    direction_for(pivot_clause.field.as_str(), true),
                    platform_version,
                )?;
                if chain_is_terminal {
                    // The pivot is the last property: `0` sits directly
                    // under each of its values.
                    pivot_query.set_subquery_key(vec![0]);
                } else {
                    pivot_query.set_subquery_key(
                        index.properties[pivot_position + 1]
                            .name
                            .as_bytes()
                            .to_vec(),
                    );
                }
                pivot_query.set_subquery(chain);
                (pivot_query, *pivot_position)
            }
        };

        for property in index.properties.iter().take(path_property_count) {
            let where_clause = self
                .internal_clauses
                .equal_clauses
                .get(property.name.as_str())
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "terminal-route selection guarantees an equality per determined prefix \
                     property",
                )))?;
            path.push(property.name.as_bytes().to_vec());
            path.push(self.document_type.serialize_value_for_key(
                &property.name,
                &where_clause.value,
                platform_version,
            )?);
        }
        match prefix_pivot {
            None => path.push(vec![0]),
            Some((pivot_position, _)) => {
                path.push(index.properties[*pivot_position].name.as_bytes().to_vec())
            }
        }

        Ok(grovedb::PathQuery::new(
            path,
            grovedb::SizedQuery::new(final_query, self.limit, self.offset),
        ))
    }

    /// The index an indexOnly query resolves to — the generic matcher
    /// when it can serve the query, else the terminal-clause route. Used
    /// by synthesis (which must decode trios against the same index the
    /// path query was built from) and by the route dispatch in the path
    /// constructors.
    pub(crate) fn index_only_query_index(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<&Index, Error> {
        match self.select_best_index(platform_version)? {
            crate::query::BestIndexOutcome::Matched(index) => {
                Self::refuse_bucketed_index_only_synthesis(index)?;
                Ok(index)
            }
            crate::query::BestIndexOutcome::NoIndexMatches(no_index_error) => {
                match self.index_only_terminal_clause_selection(platform_version)? {
                    Some(route) => Ok(route.index),
                    None => Err(no_index_error),
                }
            }
        }
    }

    /// Document synthesis over a bucketed indexOnly index is not
    /// supported: the bucket level is a derived value — the synthesized
    /// `$createdAt` would carry bucket-start granularity, not the
    /// document's timestamp — and the type's non-bucketed indexes (the
    /// proof-index rule guarantees at least one exists) serve the raw
    /// entries. Bucketed indexOnly indexes exist for the aggregate
    /// surfaces (`IN_TIME_RANGE` count / range count), which never open
    /// value trees. Only a resolved `IN_TIME_RANGE` query can select a
    /// bucketed index (raw queries are inadmissible against them), so this
    /// fires exactly on "documents in this time bucket" requests.
    fn refuse_bucketed_index_only_synthesis(index: &Index) -> Result<(), Error> {
        if index.time_range.is_some() {
            return Err(Error::Query(
                crate::error::query::QuerySyntaxError::Unsupported(
                    "IN_TIME_RANGE document queries are not supported on an indexOnly type: \
                     the bucketed entries carry bucket-start time granularity, so documents \
                     cannot be synthesized from them; use the count aggregate surfaces over \
                     the bucketed index, or query the raw entries through a non-bucketed \
                     index"
                        .to_string(),
                ),
            ));
        }
        Ok(())
    }

    /// Route an indexOnly query: `Ok(Some(..))` with the terminal-route
    /// path query when the generic index matcher cannot serve the query
    /// but a terminal clause can, `Ok(None)` when the generic route owns
    /// it. Shared by both path constructors so server, prover and
    /// verifier build the same query.
    pub(crate) fn index_only_route(
        &self,
        document_type_path: &[Vec<u8>],
        platform_version: &PlatformVersion,
    ) -> Result<Option<grovedb::PathQuery>, Error> {
        match self.select_best_index(platform_version)? {
            crate::query::BestIndexOutcome::Matched(index) => {
                // Refused here as well as in `index_only_query_index` so
                // the prover and the no-proof executor fail before
                // building a path query the synthesis side would refuse.
                Self::refuse_bucketed_index_only_synthesis(index)?;
                Ok(None)
            }
            crate::query::BestIndexOutcome::NoIndexMatches(no_index_error) => {
                match self.index_only_terminal_clause_selection(platform_version)? {
                    Some(route) => self
                        .index_only_terminal_path_query(
                            document_type_path.to_vec(),
                            &route,
                            platform_version,
                        )
                        .map(Some),
                    None => Err(no_index_error),
                }
            }
        }
    }

    /// Verify a proof for an indexOnly query, synthesizing the documents
    /// from the proved `(path, key)` positions. The mirror of the server's
    /// no-proof synthesis path — both call the one builder below.
    pub(crate) fn verify_index_only_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Document>), Error> {
        if self.start_at.is_some() {
            return Err(Error::Query(
                crate::error::query::QuerySyntaxError::Unsupported(
                    "startAt/startAfter cannot address an indexOnly position (the synthesized \
                     document id is a one-way hash of it); paginate with a range clause on \
                     the terminal property ordered by the terminal, with a limit"
                        .to_string(),
                ),
            ));
        }

        let path_query = self.construct_path_query(None, platform_version)?;
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let index = self.index_only_query_index(platform_version)?;
        let documents = proved_key_values
            .into_iter()
            .filter_map(|(path, key, element)| element.map(|_| (path, key)))
            .map(|(path, key)| {
                synthesize_index_only_document(
                    self.contract.id(),
                    self.document_type,
                    index,
                    &path,
                    &key,
                )
            })
            .collect::<Result<Vec<Document>, Error>>()?;

        Ok((root_hash, documents))
    }
}

#[cfg(feature = "server")]
impl DriveDocumentQuery<'_> {
    /// Execute an indexOnly query without a proof, synthesizing the
    /// documents from the `(path, key)` positions grove returns. The
    /// server-side mirror of [`DriveDocumentQuery::verify_index_only_proof`]
    /// — both call the one builder below.
    pub(crate) fn execute_index_only_documents_no_proof_internal(
        &self,
        drive: &crate::drive::Drive,
        transaction: grovedb::TransactionArg,
        drive_operations: &mut Vec<crate::fees::op::LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Document>, u16), Error> {
        use grovedb::query_result_type::QueryResultType;

        if self.start_at.is_some() {
            return Err(Error::Query(
                crate::error::query::QuerySyntaxError::Unsupported(
                    "startAt/startAfter cannot address an indexOnly position (the synthesized \
                     document id is a one-way hash of it); paginate with a range clause on \
                     the terminal property ordered by the terminal, with a limit"
                        .to_string(),
                ),
            ));
        }

        let path_query = self.construct_path_query_operations(
            drive,
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;

        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        );
        let (elements, skipped) = match query_result {
            Err(Error::GroveDB(grove_error))
                if matches!(
                    grove_error.as_ref(),
                    grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                ) =>
            {
                return Ok((Vec::new(), 0));
            }
            other => other?,
        };

        let index = self.index_only_query_index(platform_version)?;
        let documents = elements
            .to_path_key_elements()
            .into_iter()
            .map(|(path, key, _element)| {
                synthesize_index_only_document(
                    self.contract.id(),
                    self.document_type,
                    index,
                    &path,
                    &key,
                )
            })
            .collect::<Result<Vec<Document>, Error>>()?;
        Ok((documents, skipped))
    }
}

/// The index an executed-transition proof (waitForStateTransitionResult)
/// runs against: the first `$ownerId`-bearing index that involves no
/// `$createdAt` AND is not `skipIfAbsent` — the verifier cannot know the
/// block timestamp a time-keyed entry was written with, and a skipIfAbsent
/// index has no entry at all for a trigger-absent document, so neither can
/// anchor a proof that must exist for every create/delete. The parser
/// guarantees such an index exists (`apply_index_only`'s proof-index rule
/// mirrors exactly this predicate); prover and verifier share this one
/// selector, so they can never disagree on the anchor.
pub fn index_only_proof_index<'a>(document_type: &'a DocumentTypeRef) -> Result<&'a Index, Error> {
    use dpp::document::property_names::{CREATED_AT, OWNER_ID};
    document_type
        .indexes()
        .values()
        .find(|index| {
            let carries_owner = index.terminal.as_deref() == Some(OWNER_ID)
                || index.properties.iter().any(|p| p.name == OWNER_ID);
            let carries_created_at = index.terminal.as_deref() == Some(CREATED_AT)
                || index.properties.iter().any(|p| p.name == CREATED_AT);
            carries_owner && !carries_created_at && !index.skip_if_absent
        })
        .ok_or(Error::Query(
            crate::error::query::QuerySyntaxError::Unsupported(
                "executed-transition proofs for an indexOnly type need an \
                 $ownerId-bearing, non-skipIfAbsent index that does not involve $createdAt"
                    .to_string(),
            ),
        ))
}

/// The grove path and member key of the entry a transition's values
/// produce under `index` — the from-values twin of the write path's
/// document-based derivation, for provers and verifiers that hold a
/// transition rather than a document.
pub fn index_only_entry_path_and_key_from_values(
    contract_id: Identifier,
    document_type: DocumentTypeRef,
    index: &Index,
    data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> Result<(Vec<Vec<u8>>, Vec<u8>), Error> {
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::document::property_names::OWNER_ID;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;

    let encoded_value_for = |property_name: &str| -> Result<Vec<u8>, Error> {
        if property_name == OWNER_ID {
            return Ok(owner_id.to_vec());
        }
        // Index property names are flattened dotted paths (`profile.targetId`)
        // while transition data keeps the document's nested map shape — a
        // plain top-level get would miss every nested indexed leaf the
        // contract parser explicitly admits.
        let value = data
            .get_optional_at_path(property_name)
            .ok()
            .flatten()
            .ok_or(Error::Query(
                crate::error::query::QuerySyntaxError::Unsupported(
                    "the transition's values do not cover the index's properties".to_string(),
                ),
            ))?;
        document_type
            .serialize_value_for_key(property_name, value, platform_version)
            .map_err(|e| Error::Protocol(Box::new(e)))
    };

    // Bare property names are correct here because a bucketed index can
    // never reach this builder: it is used with the proof index (which by
    // the contract-admission rule involves no $createdAt, so it cannot be
    // bucketed) and with terminal-route indexes (which exclude bucketed
    // indexes at selection). Guarded rather than assumed.
    if index.time_range.is_some() {
        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
            "index_only_entry_path_and_key_from_values cannot address a bucketed index: \
             its levels are keyed by the grid-qualified storage key, not the property name",
        )));
    }

    let mut path: Vec<Vec<u8>> = Vec::with_capacity(5 + index.properties.len() * 2);
    path.push(vec![crate::drive::RootTree::DataContractDocuments as u8]);
    path.push(contract_id.to_vec());
    path.push(vec![1]);
    path.push(document_type.name().as_bytes().to_vec());
    for property in index.properties.iter() {
        path.push(property.name.as_bytes().to_vec());
        path.push(encoded_value_for(&property.name)?);
    }
    path.push(vec![0]);

    let terminal =
        index
            .terminal
            .as_deref()
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "index_only_entry_path_and_key_from_values requires an indexOnly index",
            )))?;
    let member_key = encoded_value_for(terminal)?;

    Ok((path, member_key))
}

/// The single-entry `PathQuery` an executed indexOnly create or delete is
/// proven (and verified) against. Shared by `prove_state_transition` and
/// `verify_state_transition_was_executed_with_proof` — one builder, both
/// sides.
pub fn index_only_transition_entry_path_query(
    contract_id: Identifier,
    document_type: DocumentTypeRef,
    data: &BTreeMap<String, Value>,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> Result<grovedb::PathQuery, Error> {
    let index = index_only_proof_index(&document_type)?;
    let (path, member_key) = index_only_entry_path_and_key_from_values(
        contract_id,
        document_type,
        index,
        data,
        owner_id,
        platform_version,
    )?;
    let mut query = grovedb::Query::new();
    query.insert_key(member_key);
    Ok(grovedb::PathQuery::new_unsized(path, query))
}

/// Synthesize the document a proved indexOnly entry represents.
///
/// `path` is the grove path of the entry's parent (ending with the `0`
/// storage marker); `member_key` is the entry's key (the terminal
/// property's encoded value).
pub fn synthesize_index_only_document(
    contract_id: Identifier,
    document_type: DocumentTypeRef,
    index: &Index,
    path: &[Vec<u8>],
    member_key: &[u8],
) -> Result<Document, Error> {
    use dpp::document::property_names::{CREATED_AT, OWNER_ID};

    let corrupted =
        |message: &'static str| Error::Drive(DriveError::CorruptedCodeExecution(message));

    // The path must end [<prop1>, <val1>, …, <propK>, <valK>, [0]].
    let expected_suffix_len = index.properties.len() * 2 + 1;
    if path.len() < expected_suffix_len {
        return Err(corrupted(
            "indexOnly synthesis: proved path is shorter than the resolved index's shape",
        ));
    }
    let suffix = &path[path.len() - expected_suffix_len..];
    if suffix.last().map(|segment| segment.as_slice()) != Some(&[0u8][..]) {
        return Err(corrupted(
            "indexOnly synthesis: proved path does not end at the 0 storage marker",
        ));
    }

    let mut properties: BTreeMap<String, Value> = BTreeMap::new();
    let mut owner_id: Option<Identifier> = None;
    let mut created_at: Option<u64> = None;

    let mut assign = |property_name: &str, encoded: &[u8]| -> Result<(), Error> {
        match property_name {
            OWNER_ID => {
                owner_id = Some(
                    Identifier::from_bytes(encoded)
                        .map_err(|_| corrupted("indexOnly synthesis: $ownerId is not 32 bytes"))?,
                );
            }
            CREATED_AT => {
                created_at = Some(
                    dpp::data_contract::document_type::DocumentPropertyType::decode_date_timestamp(
                        encoded,
                    )
                    .ok_or(corrupted(
                        "indexOnly synthesis: $createdAt key bytes are not a timestamp",
                    ))?,
                );
            }
            name => {
                let property = document_type
                    .flattened_properties()
                    .get(name)
                    .ok_or(corrupted(
                        "indexOnly synthesis: index names a property the document type lacks",
                    ))?;
                let value = property
                    .property_type
                    .decode_value_for_tree_keys(encoded)
                    .map_err(|e| Error::Protocol(Box::new(e)))?;
                // A flattened name like `profile.targetId` must come back
                // as a nested `profile` map, not as a dotted top-level key
                // — field access, schema serialization and index encoding
                // all traverse the nested shape.
                properties.insert_at_path(name, value).map_err(|_| {
                    corrupted("indexOnly synthesis: could not rebuild the nested property path")
                })?;
            }
        }
        Ok(())
    };

    for (position, index_property) in index.properties.iter().enumerate() {
        let name_segment = &suffix[position * 2];
        if name_segment.as_slice() != index_property.name.as_bytes() {
            return Err(corrupted(
                "indexOnly synthesis: proved path property name does not match the \
                 resolved index — refusing to mislabel a value",
            ));
        }
        assign(&index_property.name, &suffix[position * 2 + 1])?;
    }

    let terminal = index.terminal.as_deref().ok_or(corrupted(
        "indexOnly synthesis requires an indexOnly index (terminal is always Some \
         after parse normalization)",
    ))?;
    assign(terminal, member_key)?;

    let owner_id = owner_id.ok_or(Error::Query(
        crate::error::query::QuerySyntaxError::Unsupported(
            "documents cannot be synthesized from an index that carries no $ownerId; \
             query through an owner-bearing index"
                .to_string(),
        ),
    ))?;

    // Deterministic content-scoped id (see module docs). Every
    // variable-length component is length-framed (`u32_be(len) ‖ bytes`)
    // so distinct index positions can never concatenate to the same
    // preimage, and every distinguishing component of the proved position
    // — `$createdAt` included — participates: two rows differing only in
    // their indexed creation time are different positions and must get
    // different ids (verified results downstream are keyed by id, where a
    // collision would silently drop a document).
    let frame = |preimage: &mut Vec<u8>, bytes: &[u8]| {
        preimage.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        preimage.extend_from_slice(bytes);
    };
    let mut id_preimage: Vec<u8> = Vec::with_capacity(128);
    id_preimage.extend_from_slice(b"index_only_synthesized_id_v1");
    id_preimage.extend_from_slice(contract_id.as_bytes());
    id_preimage.extend_from_slice(owner_id.as_bytes());
    frame(&mut id_preimage, document_type.name().as_bytes());
    for (position, index_property) in index.properties.iter().enumerate() {
        if index_property.name != OWNER_ID {
            frame(&mut id_preimage, index_property.name.as_bytes());
            frame(&mut id_preimage, &suffix[position * 2 + 1]);
        }
    }
    if terminal != OWNER_ID {
        frame(&mut id_preimage, terminal.as_bytes());
        frame(&mut id_preimage, member_key);
    }
    let id = Identifier::new(hash_double(id_preimage));

    Ok(DocumentV0 {
        id,
        owner_id,
        properties,
        created_at,
        ..Default::default()
    }
    .into())
}
