use super::{latest_profile, module_with, profile_with, wasm};
use crate::admission::{validate_prepared, validate_submitted};
use crate::bundle::ModuleName;
use crate::errors::ModuleError;
use crate::instrumentation::frame_cost::FRAME_BASE_BYTES;
use crate::instrumentation::rewrite::instrument;
use crate::instrumentation::thunks::{
    ENTER_OPERATORS, LEAVE_OPERATORS, THUNK_FIXED_OPERATORS, WRAP_OPERATORS,
};
use crate::prepared_module::prepare_module;
use crate::wasm_features::admitted_features;

const THREE_FUNCTIONS: &str = include_str!("fixtures/three_functions.wat");
const THREE_FUNCTIONS_PREPARED: &[u8] = include_bytes!("fixtures/three_functions.prepared.wasm");

fn name() -> ModuleName {
    ModuleName::parse("m", 64).expect("valid")
}

/// The golden output of preparation generation 0 on the three-function fixture. A change in
/// the bytes is a change in the generation: prepared hashes and every compiled artifact key
/// derive from them, so the golden file may only be regenerated together with a new
/// preparation generation.
#[test]
fn should_match_the_checked_in_golden_output() {
    let profile = latest_profile();
    let prepared = prepare_module(name(), &wasm(THREE_FUNCTIONS), &profile).expect("prepared");
    if prepared.prepared_bytes != THREE_FUNCTIONS_PREPARED {
        let text = wasmprinter::print_bytes(&prepared.prepared_bytes).expect("printable");
        panic!("prepared bytes differ from the golden fixture; the current output is:\n{text}");
    }
    assert_eq!(
        prepared.instrumentation.wrapped_call_sites, 3,
        "mid->leaf, run->mid, run->leaf"
    );
    assert_eq!(
        prepared.instrumentation.thunks, 4,
        "dash_alloc, leaf, mid, run"
    );
    // dash_alloc: 32 + 4 (param) + 0 locals + 2 operand slots * 8.
    assert_eq!(
        prepared.instrumentation.frame_bytes[0],
        FRAME_BASE_BYTES + 4 + 16
    );
    // leaf: 32 + 4 + 8 (i64 local) + 2 slots * 8.
    assert_eq!(
        prepared.instrumentation.frame_bytes[1],
        FRAME_BASE_BYTES + 4 + 8 + 16
    );
    // mid: 32 + 4 + 0 + 2 slots * 8.
    assert_eq!(
        prepared.instrumentation.frame_bytes[2],
        FRAME_BASE_BYTES + 4 + 16
    );
    // run: 32 + 8 (two i32 params) + 0 + 4 slots * 8 (i64, i32, i32, i32 before host_call).
    assert_eq!(
        prepared.instrumentation.frame_bytes[3],
        FRAME_BASE_BYTES + 8 + 32
    );
    assert_eq!(
        prepared.instrumentation.max_frame_bytes,
        FRAME_BASE_BYTES + 8 + 32
    );
}

#[test]
fn should_produce_output_that_re_validates_under_the_admitted_features() {
    let profile = latest_profile();
    let prepared = prepare_module(name(), &wasm(THREE_FUNCTIONS), &profile).expect("prepared");
    wasmparser::Validator::new_with_features(admitted_features())
        .validate_all(&prepared.prepared_bytes)
        .expect("prepared bytes validate");
}

#[test]
fn should_route_exports_and_table_entries_through_thunks_and_shift_globals() {
    let profile = latest_profile();
    let prepared = prepare_module(name(), &wasm(THREE_FUNCTIONS), &profile).expect("prepared");
    let text = wasmprinter::print_bytes(&prepared.prepared_bytes).expect("printable");
    // Function index space: 0 trap, 1 host_call, 2..=5 dash_alloc/leaf/mid/run, 6 enter,
    // 7 leave, 8..=11 thunks in ascending order of the wrapped function.
    assert!(text.contains("(export \"dash_alloc\" (func 8))"), "{text}");
    assert!(text.contains("(export \"run\" (func 11))"), "{text}");
    assert!(
        text.contains("(elem (;0;) (i32.const 0) func 9 10)"),
        "{text}"
    );
    // The submitted global sits after the two injected counters.
    assert!(
        text.contains("(global (;2;) (mut i32) i32.const 0)"),
        "{text}"
    );
    assert!(text.contains("global.get 2"), "{text}");
    // The host import keeps its position after the trap import.
    assert!(
        text.contains("(import \"dash_vm\" \"trap\" (func (;0;) (type"),
        "{text}"
    );
    assert!(
        text.contains("(import \"dash_host\" \"host_call\" (func (;1;)"),
        "{text}"
    );
    // A host call is not wrapped: `call 1` appears bare.
    assert!(text.contains("    call 1\n"), "{text}");
}

#[test]
fn should_add_a_known_number_of_operators_per_site_and_thunk() {
    let profile = latest_profile();
    let bytes = wasm(THREE_FUNCTIONS);
    let submitted = validate_submitted(&bytes, &profile).expect("admitted");
    let (prepared_bytes, report) =
        instrument(&bytes, &submitted.facts, &submitted.plan).expect("instrumented");
    let prepared =
        validate_prepared(&prepared_bytes, &profile, &submitted, &report).expect("provenance");
    let submitted_operators = submitted.facts.structure.operators;
    let params: u64 = submitted
        .plan
        .thunk_targets
        .iter()
        .map(|target| {
            submitted
                .facts
                .function_signature(*target)
                .expect("signature")
                .params
                .len() as u64
        })
        .sum();
    let expected = submitted_operators
        + u64::from(report.wrapped_call_sites) * WRAP_OPERATORS
        + ENTER_OPERATORS
        + LEAVE_OPERATORS
        + u64::from(report.thunks) * THUNK_FIXED_OPERATORS
        + params;
    assert_eq!(prepared.structure.operators, expected);
}

#[test]
fn should_reuse_an_existing_helper_type_and_add_one_otherwise() {
    let profile = latest_profile();
    let with_type = wasm(&module_with("(func (param i32))"));
    let without = wasm(&module_with("(func (param i64))"));
    let a = validate_submitted(&with_type, &profile).expect("admitted");
    let b = validate_submitted(&without, &profile).expect("admitted");
    assert!(!a.plan.helper_type_added);
    assert!(b.plan.helper_type_added);
    let (a_bytes, _) = instrument(&with_type, &a.facts, &a.plan).expect("instrumented");
    let (b_bytes, _) = instrument(&without, &b.facts, &b.plan).expect("instrumented");
    let a_text = wasmprinter::print_bytes(&a_bytes).expect("printable");
    let b_text = wasmprinter::print_bytes(&b_bytes).expect("printable");
    assert_eq!(a_text.matches("(type (;").count(), 3, "{a_text}");
    assert_eq!(b_text.matches("(type (;").count(), 4, "{b_text}");
}

#[test]
fn should_thunk_only_functions_reachable_by_table_or_export() {
    let profile = latest_profile();
    let bytes = wasm(&module_with(
        "(table 1 1 funcref) (elem (i32.const 0) $t) (func $t) (func $private) (func $ref (drop (ref.func $rf))) (func $rf) (elem declare func $rf)",
    ));
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    // dash_alloc, run (exports), $t (table), $rf (ref.func + declared elem).
    assert_eq!(prepared.instrumentation.thunks, 4);
}

#[test]
fn should_reject_a_submitted_module_that_claims_the_instrumentation_imports() {
    // The exact names the instrumenter generates, submitted by the author.
    let profile = latest_profile();
    for import in [
        "(import \"dash_vm\" \"stack_bytes\" (global (mut i32)))",
        "(import \"dash_vm\" \"stack_depth\" (global (mut i32)))",
        "(import \"dash_vm\" \"trap\" (func (param i32)))",
    ] {
        let error = validate_submitted(&wasm(&module_with(import)), &profile).expect_err("refused");
        assert!(
            matches!(
                error,
                ModuleError::Import {
                    reason: crate::errors::ImportRejection::ReservedModule,
                    ..
                }
            ),
            "{import}: {error:?}"
        );
    }
}

/// Re-encodes the prepared bytes with a fourth `dash_vm` import appended after the injected
/// three, moving every submitted import back by one.
fn with_extra_vm_import(prepared: &[u8]) -> Vec<u8> {
    struct Extra;
    impl wasm_encoder::reencode::Reencode for Extra {
        type Error = std::convert::Infallible;
        fn parse_import_section(
            &mut self,
            imports: &mut wasm_encoder::ImportSection,
            section: wasmparser::ImportSectionReader<'_>,
        ) -> Result<(), wasm_encoder::reencode::Error<Self::Error>> {
            for (position, import) in section.into_iter().enumerate() {
                let import = import?;
                if position == 3 {
                    imports.import(
                        "dash_vm",
                        "extra",
                        wasm_encoder::EntityType::Global(wasm_encoder::GlobalType {
                            val_type: wasm_encoder::ValType::I32,
                            mutable: true,
                            shared: false,
                        }),
                    );
                }
                self.parse_import(imports, import)?;
            }
            Ok(())
        }
        fn global_index(
            &mut self,
            global: u32,
        ) -> Result<u32, wasm_encoder::reencode::Error<Self::Error>> {
            Ok(if global >= 2 { global + 1 } else { global })
        }
    }
    let mut module = wasm_encoder::Module::new();
    wasm_encoder::reencode::utils::parse_core_module(
        &mut Extra,
        &mut module,
        wasmparser::Parser::new(0),
        prepared,
    )
    .expect("re-encodes");
    module.finish()
}

/// Re-encodes the prepared bytes with the two counter imports swapped.
fn with_swapped_counters(prepared: &[u8]) -> Vec<u8> {
    struct Swap;
    impl wasm_encoder::reencode::Reencode for Swap {
        type Error = std::convert::Infallible;
        fn parse_import(
            &mut self,
            imports: &mut wasm_encoder::ImportSection,
            import: wasmparser::Import<'_>,
        ) -> Result<(), wasm_encoder::reencode::Error<Self::Error>> {
            let name = match import.name {
                "stack_bytes" => "stack_depth",
                "stack_depth" => "stack_bytes",
                other => other,
            };
            imports.import(import.module, name, self.entity_type(import.ty)?);
            Ok(())
        }
    }
    let mut module = wasm_encoder::Module::new();
    wasm_encoder::reencode::utils::parse_core_module(
        &mut Swap,
        &mut module,
        wasmparser::Parser::new(0),
        prepared,
    )
    .expect("re-encodes");
    module.finish()
}

/// Re-encodes the prepared bytes with the trap import renamed.
fn with_renamed_trap(prepared: &[u8]) -> Vec<u8> {
    struct Rename;
    impl wasm_encoder::reencode::Reencode for Rename {
        type Error = std::convert::Infallible;
        fn parse_import(
            &mut self,
            imports: &mut wasm_encoder::ImportSection,
            import: wasmparser::Import<'_>,
        ) -> Result<(), wasm_encoder::reencode::Error<Self::Error>> {
            let name = if import.name == "trap" {
                "abort"
            } else {
                import.name
            };
            imports.import(import.module, name, self.entity_type(import.ty)?);
            Ok(())
        }
    }
    let mut module = wasm_encoder::Module::new();
    wasm_encoder::reencode::utils::parse_core_module(
        &mut Rename,
        &mut module,
        wasmparser::Parser::new(0),
        prepared,
    )
    .expect("re-encodes");
    module.finish()
}

#[test]
fn should_fail_provenance_as_internal_when_the_output_drifts() {
    let profile = latest_profile();
    let bytes = wasm(THREE_FUNCTIONS);
    let submitted = validate_submitted(&bytes, &profile).expect("admitted");
    let (prepared, report) =
        instrument(&bytes, &submitted.facts, &submitted.plan).expect("instrumented");
    validate_prepared(&prepared, &profile, &submitted, &report).expect("the real output passes");

    for (label, drifted) in [
        ("extra dash_vm import", with_extra_vm_import(&prepared)),
        ("swapped counters", with_swapped_counters(&prepared)),
        ("renamed trap", with_renamed_trap(&prepared)),
    ] {
        let error = validate_prepared(&drifted, &profile, &submitted, &report)
            .expect_err("drifted output must fail provenance");
        assert!(
            matches!(error, ModuleError::Internal(_)),
            "{label}: {error:?}"
        );
    }

    // A report that contradicts the code is caught too.
    let mut lying = report.clone();
    lying.wrapped_call_sites += 1;
    assert!(matches!(
        validate_prepared(&prepared, &profile, &submitted, &lying).expect_err("caught"),
        ModuleError::Internal(_)
    ));
    let mut lying = report.clone();
    lying.thunks -= 1;
    assert!(matches!(
        validate_prepared(&prepared, &profile, &submitted, &lying).expect_err("caught"),
        ModuleError::Internal(_)
    ));
}

#[test]
fn should_fail_provenance_when_prepared_bytes_are_the_canonical_bytes() {
    let profile = latest_profile();
    let bytes = wasm(THREE_FUNCTIONS);
    let submitted = validate_submitted(&bytes, &profile).expect("admitted");
    let (_, report) = instrument(&bytes, &submitted.facts, &submitted.plan).expect("instrumented");
    assert!(matches!(
        validate_prepared(&bytes, &profile, &submitted, &report).expect_err("caught"),
        ModuleError::Internal(_)
    ));
}

#[test]
fn should_compute_frame_costs_from_params_locals_and_operand_slots() {
    let profile = latest_profile();
    let bytes = wasm(&module_with(
        "(func (param i64 f32) (local i32 funcref) i64.const 1 i64.const 2 i64.const 3 drop drop drop)",
    ));
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    // 32 + (8 + 4) + (4 + 8) + 3 slots * 8.
    assert_eq!(
        prepared.instrumentation.frame_bytes[0],
        FRAME_BASE_BYTES + 12 + 12 + 24
    );
}

#[test]
fn should_keep_prepared_bytes_free_of_the_limit_value() {
    // Two profiles with different logical stack caps produce identical prepared bytes: the
    // limit lives in the runtime's globals, never in the code.
    let bytes = wasm(THREE_FUNCTIONS);
    let a = prepare_module(
        name(),
        &bytes,
        &profile_with(|limits| limits.max_logical_stack_bytes = 4096),
    )
    .expect("prepared");
    let b = prepare_module(
        name(),
        &bytes,
        &profile_with(|limits| limits.max_logical_stack_bytes = 1 << 20),
    )
    .expect("prepared");
    assert_eq!(a.prepared_bytes, b.prepared_bytes);
}

#[test]
#[ignore = "regenerates the golden fixture; run by hand when the preparation generation changes"]
fn regenerate_golden_fixture() {
    let profile = latest_profile();
    let prepared = prepare_module(name(), &wasm(THREE_FUNCTIONS), &profile).expect("prepared");
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/tests/fixtures/three_functions.prepared.wasm"
    );
    std::fs::write(path, &prepared.prepared_bytes).expect("written");
}
