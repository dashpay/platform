//! grovedb PR 670 dependency surface — names the upstream primitives
//! this module targets.
//!
//! PR 670: [`feat: add Element::ProvableCountProvableSumTree +
//! dual-axis crossover proofs`](https://github.com/dashpay/grovedb/pull/670),
//! branch `feat/provable-count-provable-sum-tree`, head SHA
//! `79d45a7d67d91f48a8f4564a6bda07aad22c9afd`. The grovedb pin in
//! every Cargo.toml under `packages/` is currently bumped to that
//! SHA so the surfaces below are callable end-to-end.
//!
//! **Not a real module of callable code** — it's a single-file
//! catalog of what the sum-query infrastructure depends on from
//! PR 670. Each item below points at the corresponding consumer in
//! this crate so renames stay easy to audit.
//!
//! # Catalog
//!
//! ## `GroveDb::verify_aggregate_sum_query`
//!
//! ```ignore
//! impl GroveDb {
//!     pub fn verify_aggregate_sum_query(
//!         proof: &[u8],
//!         path_query: &PathQuery,
//!         grove_version: &GroveVersion,
//!     ) -> Result<([u8; 32], i64), Error>;
//! }
//! ```
//!
//! Sum-side analog of the already-shipped
//! [`GroveDb::verify_aggregate_count_query`]. Walks the proof's merk
//! ops in lockstep with the prover, accumulates the boundary
//! sub-sums committed by `node_hash_with_sum`, and returns
//! `(root_hash, aggregated_sum)`.
//!
//! Consumers:
//! - [`crate::query::drive_document_sum_query::executors::range_proof::Drive::execute_document_sum_range_proof`]
//! - bench's `display_proofs` for chapter queries Q7 and Q8 in
//!   [`packages/rs-drive/benches/document_sum_worst_case.rs`](../../../../../../benches/document_sum_worst_case.rs)
//!
//! ## `QueryItem::AggregateSumOnRange(Box<QueryItem>)` + `Query::new_aggregate_sum_on_range`
//!
//! Sum-side analog of count's `QueryItem::AggregateCountOnRange` /
//! `Query::new_aggregate_count_on_range`. With the additional
//! `sum_limit: u64` and `limit_of_items_to_check: Option<u16>`
//! parameters already on `AggregateSumQuery` — the natural constructor
//! signature:
//!
//! ```ignore
//! impl Query {
//!     pub fn new_aggregate_sum_on_range(
//!         range: QueryItem,
//!         sum_limit: u64,
//!         limit_of_items_to_check: Option<u16>,
//!     ) -> Self;
//! }
//! ```
//!
//! Consumers:
//! - [`crate::query::drive_document_sum_query::path_query::DriveDocumentSumQuery::aggregate_sum_path_query`]
//!
//! ## `GroveDb::verify_aggregate_sum_query_per_key` (for carrier-aggregate)
//!
//! ```ignore
//! impl GroveDb {
//!     pub fn verify_aggregate_sum_query_per_key(
//!         proof: &[u8],
//!         path_query: &PathQuery,
//!         grove_version: &GroveVersion,
//!     ) -> Result<([u8; 32], Vec<(Vec<u8>, i64)>), Error>;
//! }
//! ```
//!
//! Sum-side analog of count's `verify_aggregate_count_query_per_key`.
//! Returns one `(outer_key, sum)` pair per resolved branch of the
//! carrier-aggregate path query. Backs the `GroupByCompound + In on
//! prefix + range on terminator` shape.
//!
//! **Status**: live in grovedb develop (PR #670 merged; head `e98bab5f` as of this PR). Drive-side
//! wired through: path-query builder + executor + verifier all call
//! `verify_aggregate_sum_query_per_key` end-to-end.
//!
//! Consumers:
//! - [`crate::query::drive_document_sum_query::executors::range_aggregate_carrier_proof::Drive::execute_document_sum_range_aggregate_carrier_proof`]
//! - [`crate::query::drive_document_sum_query::path_query::DriveDocumentSumQuery::carrier_aggregate_sum_path_query`]
//! - [`crate::verify::document_sum::verify_carrier_aggregate_sum_proof`]
//!
//! ## `Element::required_item_with_sum_item_space`
//!
//! Cost-estimation helper paralleling
//! `Element::required_item_space` — used in the cost-only
//! `DocumentEstimatedAverageSize` paths in
//! [`packages/rs-drive/src/drive/document/insert/`](../../../document/insert/)
//! when the underlying tree is sum-bearing (adds 8 bytes per i64
//! sum-item contribution).
//!
//! ## `Element::ProvableCountSumTree` constructor exposure
//!
//! Already present as a variant in current grovedb; PR 670 promotes
//! the empty-tree constructor (`Element::empty_provable_count_sum_tree`,
//! `Element::new_provable_count_sum_tree_with_flags`) to public so
//! Drive's [`packages/rs-drive/src/fees/op.rs`](../../../../fees/op.rs)'s
//! `for_known_path_key_empty_provable_count_sum_tree` can call it.
//!
//! ## `Element::ReferenceWithSumItem` variant + constructors
//!
//! ```ignore
//! pub enum Element {
//!     // ... existing variants ...
//!     ReferenceWithSumItem(
//!         ReferencePathType,
//!         MaxReferenceHop,
//!         SumValue,
//!         Option<ElementFlags>,
//!     ),  // discriminant 18
//! }
//!
//! impl Element {
//!     pub fn new_reference_with_sum_item(path: ReferencePathType, sum_value: SumValue) -> Self;
//!     pub fn new_reference_with_sum_item_with_flags(
//!         path: ReferencePathType, sum_value: SumValue, flags: Option<ElementFlags>,
//!     ) -> Self;
//!     pub fn new_reference_with_sum_item_with_hops(
//!         path: ReferencePathType, max_hops: MaxReferenceHop, sum_value: SumValue,
//!     ) -> Self;
//!     pub fn new_reference_with_sum_item_with_max_hops_and_flags(
//!         path: ReferencePathType, max_hops: MaxReferenceHop,
//!         sum_value: SumValue, flags: Option<ElementFlags>,
//!     ) -> Self;
//! }
//! ```
//!
//! Drive uses the 4-arg `_with_max_hops_and_flags` form via
//! [`crate::drive::document::make_document_reference_with_sum_item`]
//! because the count-side `make_document_reference` already passes
//! both `Some(max_reference_hops)` (the documents-keep-history depth
//! bound) and the converted element flags.
//!
//! The load-bearing **sum-side reference** element. Two roles, two
//! element types, kept distinct:
//!
//! - **Primary storage** at `[doctype, 0, doc_id]` is
//!   `Element::ItemWithSumItem(serialized_doc, sum_value, flags)` —
//!   the document body lives inline AND contributes to the
//!   primary-key SumTree.
//! - **Index references** at
//!   `[index_path, index_value, 0, doc_id]` are
//!   `Element::ReferenceWithSumItem(reference_path, sum_value, flags)`
//!   — a true reference (dereferences to the document body in
//!   primary storage, same fetch semantics as `Element::Reference`
//!   on the count side) AND a sum contribution that propagates up
//!   ancestor sum trees.
//!
//! Without `ReferenceWithSumItem`, the alternative would be to
//! either:
//!   - Use `Element::ItemWithSumItem` at the index level too (which
//!     stores the doc_id as bytes but doesn't dereference — breaks
//!     document iteration via index walks), or
//!   - Use plain `Element::Reference` (which doesn't contribute to
//!     ancestor sum trees — breaks the whole sum aggregation
//!     story).
//!
//! Consumers:
//! - [`crate::drive::document::make_document_reference_with_sum_item`]
//!   — the helper that builds `ReferenceWithSumItem` for the
//!   index-reference write path.
//! - [`crate::drive::document::insert::add_reference_for_index_level_for_contract_operations`]
//!   — the v0 of which calls the helper when
//!   `index_type.summable.is_some()`.
//! - The matching delete path (no Drive-side change needed; grovedb
//!   propagates the sum subtraction up the merk path automatically
//!   when the reference element is removed).
//!
//! ## `Element::ProvableCountProvableSumTree` (PCPS) variant
//!
//! ```ignore
//! pub enum Element {
//!     // ... existing variants ...
//!     ProvableCountProvableSumTree(
//!         Option<Vec<u8>>,    // maybe_root_key
//!         CountValue,
//!         SumValue,
//!         Option<ElementFlags>,
//!     ),  // discriminant 20
//! }
//!
//! impl Element {
//!     pub fn empty_provable_count_provable_sum_tree() -> Self;
//!     pub fn empty_provable_count_provable_sum_tree_with_flags(
//!         flags: Option<ElementFlags>,
//!     ) -> Self;
//!     pub fn new_provable_count_provable_sum_tree(maybe_root_key: Option<Vec<u8>>) -> Self;
//!     pub fn new_provable_count_provable_sum_tree_with_flags(
//!         maybe_root_key: Option<Vec<u8>>, flags: Option<ElementFlags>,
//!     ) -> Self;
//! }
//! ```
//!
//! `TreeType::ProvableCountProvableSumTree` is the new tree-type
//! enum variant. Distinct from the pre-PR-670
//! `ProvableCountSumTree` (which is per-node-count + root-only-sum)
//! — PCPS commits BOTH per-node count AND per-node sum, so a single
//! tree can serve `AggregateCountOnRange`, `AggregateSumOnRange`,
//! AND the new `AggregateCountAndSumOnRange` (combined) primitive.
//!
//! Consumers:
//! - [`crate::fees::op::LowLevelDriveOperation::for_known_path_key_empty_provable_count_provable_sum_tree`]
//!   — the empty-tree element-flags-aware constructor wrapper.
//! - [`crate::util::grove_operations::batch_insert_empty_provable_count_sum_tree`]
//!   (the existing-name module) — its `_v0` variant produces
//!   batched inserts.
//! - [`crate::drive::document::primary_key_tree_type`]'s v1 arm
//!   dispatches to `TreeType::ProvableCountProvableSumTree` when both
//!   `range_countable` AND `range_summable` are set at the doctype
//!   level (or when one provable + one root-only side combines —
//!   see the dispatch table there).
//!
//! ## `Element::NotSummed` and `Element::NotCountedOrSummed` wrappers
//!
//! ```ignore
//! pub enum Element {
//!     // ... existing variants ...
//!     NotSummed(Box<Element>),
//!     NotCountedOrSummed(Box<Element>),  // discriminant 17
//! }
//!
//! impl Element {
//!     pub fn new_not_summed(inner: Element) -> Result<Self, ElementError>;
//!     pub fn new_not_counted_or_summed(inner: Element) -> Result<Self, ElementError>;
//! }
//! ```
//!
//! - `NotSummed` only wraps sum-bearing tree variants (SumTree,
//!   BigSumTree, ProvableSumTree, CountSumTree, ProvableCountSumTree,
//!   ProvableCountProvableSumTree). Suppresses sum propagation to
//!   the parent; count contribution still propagates if present.
//! - `NotCountedOrSummed` accepts the same inner-type set and
//!   suppresses BOTH count and sum propagation.
//! - The existing `NonCounted` is now restricted to non-provable
//!   count-bearing parents — provable-count parents reject the
//!   wrapper at the merk-layer insert guard.
//!
//! Consumers:
//! - [`crate::fees::op::LowLevelDriveOperation::for_known_path_key_empty_not_summed_tree`]
//! - [`crate::fees::op::LowLevelDriveOperation::for_known_path_key_empty_not_counted_or_summed_tree`]
//! - Index walker (`add_indices_for_index_level_for_contract_operations_v0`)
//!   picks the right wrapper per the parent value-tree's aggregation
//!   axes (currently uses `NonCounted` everywhere — extending to
//!   pick between the three wrappers based on parent aggregation is
//!   a follow-up).
//!
//! ## `AggregateCountAndSumOnRange` — combined PCPS-only proof primitive
//!
//! The proper end-to-end API:
//!
//! ```ignore
//! pub enum QueryItem {
//!     // ... existing variants ...
//!     AggregateCountAndSumOnRange(Box<QueryItem>),
//! }
//!
//! impl Query {
//!     pub fn new_aggregate_count_and_sum_on_range(range: QueryItem) -> Self;
//! }
//!
//! impl Merk {
//!     // The lowest-layer prover. PR 670 callsite:
//!     // `grovedb/src/operations/proof/generate.rs:1451` —
//!     // invoked by `GroveDb::get_proved_path_query` when the
//!     // path-query's terminal item is `AggregateCountAndSumOnRange`.
//!     pub fn prove_aggregate_count_and_sum_on_range(
//!         range: &QueryItem,
//!         grove_version: &GroveVersion,
//!     ) -> Result<Vec<u8>, MerkError>;
//! }
//!
//! impl GroveDb {
//!     // The verifier. Returns the recovered merk root hash plus
//!     // BOTH metrics (count as `u64`, sum as `i64`). Uses an
//!     // `i128` accumulator internally and rejects results that
//!     // don't fit in `i64` so adversarial extremes can't wrap.
//!     pub fn verify_aggregate_count_and_sum_query(
//!         proof: &[u8],
//!         path_query: &PathQuery,
//!         grove_version: &GroveVersion,
//!     ) -> Result<(CryptoHash, u64, i64), Error>;
//! }
//! ```
//!
//! **PCPS-only**: the terminator tree must be a
//! `ProvableCountProvableSumTree`. `ProvableCountTree` /
//! `ProvableSumTree` / `ProvableCountSumTree` (the per-axis or
//! root-only sum variants) reject the query item at the prover
//! with an explicit "AggregateCountAndSumOnRange is only supported
//! on ProvableCountProvableSumTree" error.
//!
//! **Single-traversal advantage**: returns BOTH metrics from one
//! merk walk + one proof envelope. Strictly cheaper than running
//! two separate `AggregateCountOnRange` + `AggregateSumOnRange`
//! queries when the caller wants both, AND atomic — both metrics
//! are bound to the same merk root, so they can't drift relative
//! to each other across a concurrent write.
//!
//! **Leaf-shape only**: PR 670 explicitly restricts the query to
//! the leaf shape — a single `AggregateCountAndSumOnRange(_)` item
//! at the top level, no subquery branches, no compound carrier
//! shapes. The PR's commit log shows separate follow-up work for
//! carrier shapes; until those land, compound `In + range`
//! combined queries fall back to two separate
//! `verify_aggregate_count_query_per_key` +
//! `verify_aggregate_sum_query_per_key` calls.
//!
//! Drive consumer:
//! - [`crate::query::drive_document_sum_query::DriveDocumentSumQuery::aggregate_count_and_sum_path_query`]
//!   — builds the `PathQuery` shared by prover and verifier.
//!   Currently lives in the sum-query module for proximity to
//!   `aggregate_sum_path_query`; when a `DriveDocumentCountSumQuery`
//!   module lands alongside the combined-feature contract example,
//!   this builder moves there.
//! - The matching executor (`execute_document_count_and_sum_range_proof`)
//!   doesn't exist yet — the bench / book chapters target either
//!   pure-sum or pure-count, so the combined executor lands as
//!   focused follow-up alongside the combined-feature contract
//!   (donation-log / creator-analytics — see
//!   [`book/src/drive/document-sum-trees.md`](../../../../../../book/src/drive/document-sum-trees.md)'s
//!   "Choosing What to Set" table's "both" row for the forward
//!   reference).
//!
//! **Carrier variant**: live in grovedb develop (PR #670 merged; head `e98bab5f` as of this PR).
//! Drive-side wired through
//! ([`crate::query::drive_document_sum_query::DriveDocumentSumQuery::carrier_aggregate_count_and_sum_path_query`]
//! +
//! [`crate::query::drive_document_sum_query::DriveDocumentSumQuery::execute_carrier_aggregate_count_and_sum_with_proof`]
//! +
//! [`crate::verify::document_sum::verify_carrier_aggregate_count_and_sum_proof`]).
//!
//! ## What's still NotSupported after the grovedb bump
//!
//! The grovedb dependency now resolves to PR 670's HEAD, so most
//! upstream surfaces above are callable. What remains stubbed on the
//! Drive / SDK side:
//!
//! 1. **Executor bodies** in
//!    [`crate::query::drive_document_sum_query::executors`] —
//!    six of seven still return `Error::Drive(DriveError::NotSupported)`
//!    pending line-by-line port from the count-side analogs
//!    documented in [`executors::mod`].
//! 2. **`distinct_sum_path_query`** in
//!    [`crate::query::drive_document_sum_query::path_query`] —
//!    pending port from count's analog (~280 lines).
//! 3. **`Drive::dispatch_sum_v1`** in
//!    `packages/rs-drive-abci/src/query/document_query/v1/mod.rs`
//!    — routing in place; ~120-line body pending mirror of
//!    `dispatch_count_v1`.
//! 4. **`FromProof` bodies** for [`drive_proof_verifier::DocumentSum`]
//!    and [`drive_proof_verifier::DocumentSplitSums`] —
//!    scaffolded, bodies pending.
//! 5. **Sum-aware continuation wrapping** in the index walker
//!    (`add_indices_for_index_level_for_contract_operations_v0`) —
//!    currently uses `NonCounted` for all continuations. Pick
//!    between `NonCounted` / `NotSummed` / `NotCountedOrSummed`
//!    based on the parent value-tree's aggregation axes. ~30 lines
//!    of focused branching.
// (was: carrier-sum primitives pending grovedb — now live in
// grovedb develop (PR #670 merged; head `e98bab5f` as of this PR) and wired through end-to-end.)
//!
//! Activation sequence once those land:
//! 1. Run `cargo bench --bench document_sum_worst_case` to validate
//!    + backfill the TBD proof bytes/timings in
//!    [`book/src/drive/sum-index-examples.md`](../../../../../../book/src/drive/sum-index-examples.md).
//! 2. The `primary_key_tree_type: 1` bump in
//!    [`drive_document_method_versions/v2.rs`](../../../../../rs-platform-version/src/version/drive_versions/drive_document_method_versions/v2.rs)
//!    already activates the v1 dispatch arm under platform v12.
