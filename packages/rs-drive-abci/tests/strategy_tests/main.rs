//! Execution Tests
//!

// Test-code heavy lints. See packages/rs-drive-abci/src/lib.rs for the matching
// set applied to the crate's unit tests.
#![allow(clippy::useless_vec)]
#![allow(clippy::cloned_ref_to_slice_refs)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::unnecessary_mut_passed)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::items_after_test_module)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::result_large_err)]
#![allow(clippy::question_mark)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::infallible_destructuring_match)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::expect_fun_call)]
#![allow(clippy::bool_comparison)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_variables)]

extern crate core;
use dpp::bls_signatures::SecretKey as BlsPrivateKey;
mod addresses_with_balance;
mod execution;
mod failures;
mod masternode_list_item_helpers;
mod masternodes;
mod query;
mod strategy;
mod test_cases;
mod verify_state_transitions;
