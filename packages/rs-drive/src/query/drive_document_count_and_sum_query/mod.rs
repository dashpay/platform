//! Joint count-and-sum executor surface for the AVG no-prove path.
//!
//! Resolves [issue #3687](https://github.com/dashpay/platform/issues/3687):
//! the AVG dispatcher previously composed `DocumentCountRequest +
//! DocumentSumRequest` on the no-prove path and zipped the two
//! responses, which double-walked grovedb for every AVG query.
//!
//! This module exposes a single [`Drive::execute_document_count_and_sum_request`]
//! entry that consumes the same [`DocumentAverageRequest`] /
//! [`DocumentAverageResponse`] pair used by the prove path and reads
//! both metrics from each visited PCPS / CountSumTree element in one
//! grovedb walk via [`Element::count_sum_value_or_default`].
//!
//! ## Routing
//!
//! Mode resolution reuses sum's versioned routing table
//! ([`crate::query::drive_document_sum_query::mode_detection::detect_sum_mode_from_inputs`])
//! — the routing decision is consensus-relevant and identical for
//! count, sum, and joint count+sum on the no-prove path, so forking
//! the table here would invite the same drift bug PR #3661 caught
//! (count's `GroupByCompound` row had diverged from sum's). A trivial
//! [`AverageMode`] → [`SumMode`] adapter feeds sum's mode detector;
//! `prove = false` is passed unconditionally because the prove path
//! never enters this module.
//!
//! ## Perf characteristic
//!
//! On the no-prove path this halves the grovedb work per AVG query
//! versus the pre-#3687 composition: one walk yielding `(count, sum)`
//! per visited element instead of two parallel walks zipped at the
//! dispatcher layer.
//!
//! ## A note on the engine-side combined primitive
//!
//! grovedb's pinned rev exposes `query_aggregate_sum` and
//! `query_aggregate_count` as no-prove engine-side accumulators
//! (their proof-side analogs are `AggregateSumOnRange` /
//! `AggregateCountOnRange` / the combined `AggregateCountAndSumOnRange`
//! used by the prove path). There is no engine-side no-prove
//! `query_aggregate_count_and_sum` primitive yet. As a result the
//! range-no-proof "flat summed" branches here walk PCPS elements via
//! `grove_get_raw_path_query` and fold in Rust — see
//! [`executors::range_no_proof`] for the implementation note. This is
//! still one grovedb walk per query (versus sum + count parallel walks
//! before), just not the further reduction of "one merk-internal
//! accumulator call" the prove path benefits from. A future optional
//! grovedb optimization could collapse the Rust-side fold into an
//! engine-side accumulator; #3687 captures the follow-up.

#[cfg(feature = "server")]
pub mod drive_dispatcher;

#[cfg(feature = "server")]
pub mod executors;
