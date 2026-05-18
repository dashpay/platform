//! Per-`DocumentSumMode` executor modules. Each module owns a single
//! executor function and the helpers it needs. Mirrors count's
//! `executors/` layout — file names parallel byte-for-byte.

pub mod per_in_value;
pub mod point_lookup_proof;
pub mod range_aggregate_carrier_proof;
pub mod range_distinct_proof;
pub mod range_no_proof;
pub mod range_proof;
pub mod total;

// TODO(sum-feature): each sub-module is currently a stub that returns
// NotSupported. To fill them in:
//   1. Copy the corresponding count executor (`executors/<name>.rs` in
//      `drive_document_count_query/`).
//   2. Rename `Count` → `Sum`, `u64` → `i64`, `count_value_or_default`
//      → `sum_value_or_default`.
//   3. Swap the grovedb primitives:
//        AggregateCountQuery → AggregateSumQuery
//        verify_aggregate_count_query → verify_aggregate_sum_query
//        Element::CountTree → Element::SumTree
//        Element::ProvableCountTree → Element::ProvableSumTree
//        Element::NonCounted → Element::NonCountedItemWithSumItem
//          where the operand is `ItemWithSumItem`
//   4. Cross-validate the request's `sum_property` against the
//      chosen index's `summable: "<x>"` at the executor entry.
//   5. Run the bench (`document_sum_worst_case`) against the
//      tip-jar fixture and verify the chapter's expected numbers.
