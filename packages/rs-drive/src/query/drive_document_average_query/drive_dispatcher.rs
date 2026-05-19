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
//!   grovedb snapshot, so they see identical state (no block-
//!   boundary race, no off-by-one). When the caller passes a
//!   `TransactionArg::None` (the drive-abci query path), the
//!   dispatcher opens a short-lived read transaction internally and
//!   reuses it across both sub-calls so the atomicity guarantee
//!   holds regardless of caller plumbing. The internal transaction
//!   is rolled back at the end (read-only, never commits).
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

        // Atomicity: both sub-reads must see the same grovedb root. If
        // the caller didn't provide a transaction we open a short-lived
        // read transaction here and reuse it across both executors so
        // a concurrent block commit can't slip between the count and
        // sum reads (the attacker-steerable race documented in the
        // module-level docstring). The local transaction is read-only
        // and dropped without commit at the end of this function.
        let local_tx;
        let effective_transaction: TransactionArg = if transaction.is_some() {
            transaction
        } else {
            local_tx = self.grove.start_transaction();
            Some(&local_tx)
        };

        let count_response = self.execute_document_count_request(
            count_request,
            effective_transaction,
            platform_version,
        )?;
        let sum_response = self.execute_document_sum_request(
            sum_request,
            effective_transaction,
            platform_version,
        )?;

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
            )?)),
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

/// Merge per-`(in_key, key)` count entries and sum entries into average
/// entries via a strict two-pointer merge keyed on `(in_key, key)`.
///
/// Both inputs are emitted by the same executor family with identical
/// `where_clauses` / `order_clauses` / `mode` against the same grovedb
/// snapshot, so they MUST emit the same set of keys in the same
/// ascending `(in_key, key)` order. Any divergence (key on one side
/// only, or different ordering) indicates an executor bug and is
/// surfaced as `CorruptedCodeExecution` rather than silently zeroed at
/// the wire layer — the previous defensive `None`-preservation pattern
/// was indistinguishable from "this key matched zero documents but the
/// sum is nonzero" once the wire mapping flattened `Option<u64>` →
/// `u64`, which let attacker-timed inserts between the two reads
/// produce a `count=0, sum=V` bucket that crashed naive `sum / count`
/// clients with a divide-by-zero. With atomicity now enforced inside
/// `execute_document_average_request` (see module docstring), the only
/// remaining cause of divergence is a real executor bug — treating it
/// as fatal is correct.
///
/// Output is always strictly ascending by `(in_key, key)` (same order
/// the inputs are required to be in).
#[cfg(feature = "server")]
fn zip_entries(
    count_entries: Vec<crate::query::SplitCountEntry>,
    sum_entries: Vec<crate::query::SumEntry>,
) -> Result<Vec<AverageEntry>, Error> {
    use crate::error::drive::DriveError;

    let mut out = Vec::with_capacity(count_entries.len().max(sum_entries.len()));
    let mut c_iter = count_entries.into_iter();
    let mut s_iter = sum_entries.into_iter();
    let mut next_c = c_iter.next();
    let mut next_s = s_iter.next();

    loop {
        match (&next_c, &next_s) {
            (Some(c), Some(s)) => {
                let c_key = (&c.in_key, &c.key);
                let s_key = (&s.in_key, &s.key);
                match c_key.cmp(&s_key) {
                    std::cmp::Ordering::Equal => {
                        let c = next_c.take().expect("checked Some above");
                        let s = next_s.take().expect("checked Some above");
                        out.push(AverageEntry {
                            in_key: c.in_key,
                            key: c.key,
                            count: c.count,
                            sum: s.sum,
                        });
                        next_c = c_iter.next();
                        next_s = s_iter.next();
                    }
                    std::cmp::Ordering::Less => {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "average composition: count executor emitted a (in_key, key) the \
                             sum executor didn't — both executors run identical inputs against \
                             the same grovedb snapshot, so divergence indicates an executor bug",
                        )));
                    }
                    std::cmp::Ordering::Greater => {
                        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "average composition: sum executor emitted a (in_key, key) the \
                             count executor didn't — both executors run identical inputs against \
                             the same grovedb snapshot, so divergence indicates an executor bug",
                        )));
                    }
                }
            }
            (Some(_), None) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "average composition: count executor produced more entries than sum executor \
                     — both executors run identical inputs against the same grovedb snapshot, \
                     so divergence indicates an executor bug",
                )));
            }
            (None, Some(_)) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "average composition: sum executor produced more entries than count executor \
                     — both executors run identical inputs against the same grovedb snapshot, \
                     so divergence indicates an executor bug",
                )));
            }
            (None, None) => break,
        }
    }
    Ok(out)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::query::{SplitCountEntry, SumEntry};

    fn cc(in_key: Option<&[u8]>, key: &[u8], count: u64) -> SplitCountEntry {
        SplitCountEntry {
            in_key: in_key.map(|b| b.to_vec()),
            key: key.to_vec(),
            count: Some(count),
        }
    }
    fn ss(in_key: Option<&[u8]>, key: &[u8], sum: i64) -> SumEntry {
        SumEntry {
            in_key: in_key.map(|b| b.to_vec()),
            key: key.to_vec(),
            sum: Some(sum),
        }
    }

    #[test]
    fn zip_entries_merges_aligned_streams_in_ascending_order() {
        let count_entries = vec![cc(None, b"a", 1), cc(None, b"b", 2), cc(None, b"c", 3)];
        let sum_entries = vec![ss(None, b"a", 10), ss(None, b"b", 20), ss(None, b"c", 30)];
        let out = zip_entries(count_entries, sum_entries).expect("aligned streams must merge");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].key, b"a");
        assert_eq!(out[0].count, Some(1));
        assert_eq!(out[0].sum, Some(10));
        assert_eq!(out[2].key, b"c");
        assert_eq!(out[2].count, Some(3));
        assert_eq!(out[2].sum, Some(30));
    }

    #[test]
    fn zip_entries_errors_when_count_has_an_extra_key() {
        // count has `b` but sum doesn't — strict merge must reject.
        let count_entries = vec![cc(None, b"a", 1), cc(None, b"b", 2)];
        let sum_entries = vec![ss(None, b"a", 10)];
        let err = zip_entries(count_entries, sum_entries)
            .expect_err("divergent streams must surface as CorruptedCodeExecution");
        assert!(
            matches!(err, Error::Drive(DriveError::CorruptedCodeExecution(_))),
            "expected CorruptedCodeExecution, got {err:?}",
        );
    }

    #[test]
    fn zip_entries_errors_when_sum_has_an_extra_key() {
        let count_entries = vec![cc(None, b"a", 1)];
        let sum_entries = vec![ss(None, b"a", 10), ss(None, b"b", 20)];
        let err = zip_entries(count_entries, sum_entries)
            .expect_err("divergent streams must surface as CorruptedCodeExecution");
        assert!(
            matches!(err, Error::Drive(DriveError::CorruptedCodeExecution(_))),
            "expected CorruptedCodeExecution, got {err:?}",
        );
    }

    #[test]
    fn zip_entries_errors_when_streams_disagree_on_a_key_in_the_middle() {
        // count has `b`, sum has `c` between the matching `a` and `d`.
        let count_entries = vec![cc(None, b"a", 1), cc(None, b"b", 2), cc(None, b"d", 4)];
        let sum_entries = vec![ss(None, b"a", 10), ss(None, b"c", 30), ss(None, b"d", 40)];
        let err = zip_entries(count_entries, sum_entries)
            .expect_err("middle-of-stream divergence must surface as CorruptedCodeExecution");
        assert!(matches!(
            err,
            Error::Drive(DriveError::CorruptedCodeExecution(_))
        ));
    }

    #[test]
    fn zip_entries_handles_compound_in_key_ordering() {
        // (Some("X"), "a") < (Some("X"), "b") < (Some("Y"), "a") in
        // lexicographic order — verify the merge follows it.
        let count_entries = vec![
            cc(Some(b"X"), b"a", 1),
            cc(Some(b"X"), b"b", 2),
            cc(Some(b"Y"), b"a", 3),
        ];
        let sum_entries = vec![
            ss(Some(b"X"), b"a", 10),
            ss(Some(b"X"), b"b", 20),
            ss(Some(b"Y"), b"a", 30),
        ];
        let out = zip_entries(count_entries, sum_entries).expect("aligned compound merge");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].in_key.as_deref(), Some(b"X".as_ref()));
        assert_eq!(out[0].key, b"a");
        assert_eq!(out[2].in_key.as_deref(), Some(b"Y".as_ref()));
        assert_eq!(out[2].key, b"a");
    }
}
