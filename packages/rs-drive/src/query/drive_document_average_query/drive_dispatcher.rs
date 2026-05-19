//! Average-query dispatcher entry point.
//!
//! Implementation strategy: **compose** count + sum into the
//! `(count, sum)` pair the client divides. Both executors are real
//! and live in `drive_document_count_query` /
//! `drive_document_sum_query` respectively; the average dispatcher
//! issues both requests under the same `transaction` and zips their
//! responses together by `(in_key, key)` for grouped shapes.
//!
//! ## Why compose instead of using a single PCPS traversal?
//!
//! grovedb's `AggregateCountAndSumOnRange` primitive can in principle
//! return both metrics from one root-hash-committed traversal, which
//! would be cheaper on the wire (one proof instead of two) and
//! atomic. The PCPS executor that calls that primitive is a planned
//! follow-up — see `drive_document_sum_query/grovedb_pr_670.rs`. For
//! now this composition path delivers correct AVG semantics today by
//! reusing the proven SUM / COUNT executors:
//!
//! - **No-prove paths**: count + sum are read within the same
//!   `transaction` snapshot, so they see identical state (no block-
//!   boundary race, no off-by-one).
//! - **Prove path**: not supported here — the on-wire bytes would be
//!   two concatenated proofs whose verification semantics aren't
//!   defined. Returns a typed `NotYetImplemented` so callers requesting
//!   `prove=true` AVG can detect the gap and route to the future PCPS
//!   path once it lands.
//!
//! When the PCPS executor lands, this dispatcher's no-prove paths can
//! switch over without breaking the wire surface — the
//! `DocumentAverageRequest` / `DocumentAverageResponse` shapes stay
//! identical.

use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_average_query::{
    AverageEntry, AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use crate::query::drive_document_count_query::{
    CountMode, DocumentCountRequest, DocumentCountResponse,
};
use crate::query::drive_document_sum_query::{DocumentSumRequest, DocumentSumResponse, SumMode};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

#[cfg(feature = "server")]
impl Drive {
    /// Server-side entry point for the average surface. Composes the
    /// count + sum executors and zips their outputs into the
    /// `(count, sum)` pair the client divides.
    ///
    /// See the module docstring for the rationale on composition vs.
    /// a single PCPS traversal.
    pub fn execute_document_average_request(
        &self,
        request: DocumentAverageRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        if request.prove {
            // PCPS-based proof returning (count, sum) atomically is the
            // planned future path. Composition would produce two
            // concatenated proofs whose verification contract isn't
            // defined; reject explicitly rather than fabricate bytes
            // that no verifier knows how to consume.
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "execute_document_average_request with prove=true: proven averages \
                 require grovedb's `AggregateCountAndSumOnRange` proof primitive (PR \
                 670). The no-prove path composes count + sum executors directly and \
                 works today — switch to prove=false to get the (count, sum) pair the \
                 client divides."
                    .to_string(),
            )));
        }

        // Map `AverageMode` → matching `CountMode` / `SumMode`. The
        // three enums are structurally identical (same four variants);
        // each pair just lives in its own namespace.
        let (count_mode, sum_mode) = match request.mode {
            AverageMode::Aggregate => (CountMode::Aggregate, SumMode::Aggregate),
            AverageMode::GroupByIn => (CountMode::GroupByIn, SumMode::GroupByIn),
            AverageMode::GroupByRange => (CountMode::GroupByRange, SumMode::GroupByRange),
            AverageMode::GroupByCompound => (CountMode::GroupByCompound, SumMode::GroupByCompound),
        };

        // Build parallel sub-requests. Both consume the same
        // `where_clauses` + `order_clauses` + `limit` + (false) `prove`
        // — the average's shape contract is "two reads of the same
        // grovedb snapshot, zipped after."
        let count_request = DocumentCountRequest {
            contract: request.contract,
            document_type: request.document_type,
            where_clauses: request.where_clauses.clone(),
            order_clauses: request.order_clauses.clone(),
            mode: count_mode,
            limit: request.limit,
            prove: false,
            drive_config: request.drive_config,
        };
        let sum_request = DocumentSumRequest {
            contract: request.contract,
            document_type: request.document_type,
            sum_property: request.sum_property,
            where_clauses: request.where_clauses,
            order_clauses: request.order_clauses,
            mode: sum_mode,
            limit: request.limit,
            prove: false,
            drive_config: request.drive_config,
        };

        let count_response =
            self.execute_document_count_request(count_request, transaction, platform_version)?;
        let sum_response =
            self.execute_document_sum_request(sum_request, transaction, platform_version)?;

        // Combine. The two executors emit either Aggregate or Entries
        // (Proof is unreachable here since `prove=false` above). The
        // mode-pair is symmetric so they must agree on which shape
        // they emit — mismatches indicate a routing bug, surface as
        // CorruptedCodeExecution.
        match (count_response, sum_response) {
            (DocumentCountResponse::Aggregate(count), DocumentSumResponse::Aggregate(sum)) => {
                Ok(DocumentAverageResponse::Aggregate { count, sum })
            }
            (
                DocumentCountResponse::Entries(count_entries),
                DocumentSumResponse::Entries(sum_entries),
            ) => Ok(DocumentAverageResponse::Entries(zip_entries(
                count_entries,
                sum_entries,
            ))),
            // Mismatched shapes — count executor and sum executor
            // disagreed on whether the result fits in a single row.
            // Should be impossible because they share the same mode
            // and `validate_and_canonicalize_where_clauses` runs the
            // same checks on both.
            _ => Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedCodeExecution(
                    "average composition: count and sum executors emitted disagreeing \
                     response shapes — both should agree on Aggregate vs Entries given \
                     identical mode + where + group_by",
                ),
            )),
        }
    }
}

/// Zip per-`(in_key, key)` count entries and sum entries into average
/// entries. Both inputs are emitted by the same executor family in the
/// same `(in_key, key)` order, so a single pass works.
///
/// Defensive: if the two streams diverge on keys (executor bug), keys
/// present only in one side get `None` for the other axis on the
/// emitted `AverageEntry` so the wire layer can decide how to surface
/// the inconsistency (clients see `Option<u64> count` and
/// `Option<i64> sum`).
/// `(in_key, key)` pair used to zip count and sum entries together.
/// `in_key` is `Some` for compound (`In + range`) executor paths, `None`
/// otherwise; `key` is the terminator value.
#[cfg(feature = "server")]
type EntryKey = (Option<Vec<u8>>, Vec<u8>);

#[cfg(feature = "server")]
fn zip_entries(
    count_entries: Vec<crate::query::SplitCountEntry>,
    sum_entries: Vec<crate::query::SumEntry>,
) -> Vec<AverageEntry> {
    // Stream-merge by `(in_key, key)`. Both executors emit entries in
    // ascending grovedb key order, so a sort/merge isn't needed in the
    // happy path — but we keep the merge logic robust against future
    // executor changes that might reorder.
    let mut sum_by_key: BTreeMap<EntryKey, Option<i64>> = sum_entries
        .into_iter()
        .map(|e| ((e.in_key, e.key), e.sum))
        .collect();

    let mut out = Vec::with_capacity(count_entries.len() + sum_by_key.len());
    for ce in count_entries {
        let key_pair = (ce.in_key, ce.key);
        let sum = sum_by_key.remove(&key_pair);
        out.push(AverageEntry {
            in_key: key_pair.0,
            key: key_pair.1,
            count: ce.count,
            sum: sum.unwrap_or(None),
        });
    }
    // Any sum-only keys (sum had entries the count side didn't —
    // indicates an executor bug, but emit them with `count: None` so
    // the wire layer can decide what to do).
    for ((in_key, key), sum) in sum_by_key {
        out.push(AverageEntry {
            in_key,
            key,
            count: None,
            sum,
        });
    }
    out
}
