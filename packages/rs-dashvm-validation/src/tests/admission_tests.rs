use super::{latest_profile, module_with, wasm, MINIMAL};
use crate::admission::validate_submitted;
use crate::bundle::ModuleName;
use crate::prepared_module::prepare_module;

fn name() -> ModuleName {
    ModuleName::parse("m", 64).expect("valid")
}

#[test]
fn should_admit_the_minimal_module_and_prepare_it() {
    let profile = latest_profile();
    let bytes = wasm(MINIMAL);
    let submitted = validate_submitted(&bytes, &profile).expect("admitted");
    assert_eq!(submitted.interface().exports.len(), 2);
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    assert_eq!(
        prepared.instrumentation.thunks, 2,
        "both exports get a thunk"
    );
    assert_eq!(prepared.instrumentation.wrapped_call_sites, 0);
    assert_ne!(prepared.canonical_hash.0, prepared.prepared_hash.0);
}

#[test]
fn should_wrap_calls_and_thunk_table_entries() {
    let profile = latest_profile();
    let bytes = wasm(&module_with(
        r#"
  (table 2 2 funcref)
  (elem (i32.const 0) $leaf $leaf)
  (func $leaf (param i32) (result i32) local.get 0)
  (func $caller (result i32) (i32.const 1) (call $leaf) (i32.const 2) (call $leaf) i32.add)
  (func $indirect (result i32) (i32.const 3) (i32.const 0) (call_indirect (param i32) (result i32)))
"#,
    ));
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    assert_eq!(prepared.instrumentation.wrapped_call_sites, 2);
    assert_eq!(
        prepared.instrumentation.thunks, 3,
        "dash_alloc, run and leaf"
    );
}
