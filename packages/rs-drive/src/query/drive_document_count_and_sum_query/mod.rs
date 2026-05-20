//! Joint count-and-sum executor surface for the AVG no-prove path.
//!
//! [`Drive::execute_document_count_and_sum_request`] consumes the same
//! [`DocumentAverageRequest`] / [`DocumentAverageResponse`] pair used
//! by the prove path and dispatches to one of three per-mode executors
//! depending on the resolved [`DocumentSumMode`].
//!
//! ## Per-shape execution
//!
//! - **`Total` / `PerInValue`** (empty-where, or Equal/`In` on a
//!   `summable + countable` index): point-lookup walk against the
//!   index, decoding `(count, sum)` from each visited `CountSumTree` /
//!   `ProvableCountSumTree` terminator in one call via
//!   [`Element::count_sum_value_or_default`]. One grovedb read per
//!   In branch yields both metrics together.
//! - **`RangeNoProof` distinct shapes** (`GroupByRange` /
//!   `GroupByCompound` + range on a `rangeAverageable` index): one
//!   grovedb walk against
//!   [`crate::query::drive_document_sum_query::DriveDocumentSumQuery::distinct_sum_path_query`]'s
//!   `ProvableCountProvableSumTree` terminators, emitting one
//!   `(count, sum)` per visited distinct in-range key. Bounded by the
//!   request's `limit` (default falls back to
//!   `drive_config.default_query_limit`, explicit limits are clamped to
//!   `drive_config.max_query_limit`).
//! - **`RangeNoProof` aggregate shapes** (`Aggregate` / `GroupByIn` +
//!   range): grovedb's merk-internal accumulators —
//!   `query_aggregate_count` against the count path query AND
//!   `query_aggregate_sum` against the sum path query — under a shared
//!   read transaction. Two O(log n) merk-internal reads regardless of
//!   how many documents the range matches. Compound `In + range` per-In
//!   fans out (≤100 branches per the `In::in_values()` validator cap)
//!   and issues one count + one sum accumulator call per branch.
//!
//! ## Routing
//!
//! Mode resolution reuses sum's versioned routing table
//! ([`crate::query::drive_document_sum_query::mode_detection::detect_sum_mode_from_inputs`])
//! — the routing decision is consensus-relevant and identical for
//! count, sum, and joint count+sum on the no-prove path, so forking
//! the table here would invite the drift bug PR #3661 caught (count's
//! `GroupByCompound` row had diverged from sum's). A trivial
//! [`AverageMode`] → [`SumMode`] adapter feeds sum's mode detector;
//! `prove = false` is passed unconditionally because the prove path
//! never enters this module.
//!
//! ## Engine-side primitive note
//!
//! grovedb exposes `query_aggregate_sum` and `query_aggregate_count`
//! as no-prove engine-side accumulators (their proof-side analogs are
//! `AggregateSumOnRange` / `AggregateCountOnRange`, plus the combined
//! `AggregateCountAndSumOnRange` the prove path uses). The aggregate
//! `RangeNoProof` branch issues both no-prove accumulators in
//! parallel because no combined no-prove primitive exists. A future
//! `query_aggregate_count_and_sum` grovedb addition could collapse
//! those two calls into a single merk-internal read, but the current
//! two-accumulator shape is already in the same O(log n) cost class
//! and bounded against DAPI amplification.

#[cfg(feature = "server")]
pub mod drive_dispatcher;

#[cfg(feature = "server")]
pub mod executors;
