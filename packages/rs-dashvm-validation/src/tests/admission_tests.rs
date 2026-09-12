use super::{latest_profile, module_with, profile_with, wasm, MINIMAL};
use crate::abi_names::{alloc_signature, entry_signature, host_envelope};
use crate::admission::validate_submitted;
use crate::bundle::FuncSignature;
use crate::bundle::{ModuleName, ValueType};
use crate::errors::{
    ExportRejection, ForbiddenFeature, ImportRejection, MemoryRule, ModuleError, StructuralCap,
    TableRule,
};
use crate::prepared_module::prepare_module;
use crate::profile::PreparationProfile;

fn name() -> ModuleName {
    ModuleName::parse("m", 64).expect("valid")
}

fn reject(wat: &str, profile: &PreparationProfile) -> ModuleError {
    let bytes = wasm(wat);
    validate_submitted(&bytes, profile).expect_err("the module must be refused")
}

fn reject_bytes(bytes: &[u8], profile: &PreparationProfile) -> ModuleError {
    validate_submitted(bytes, profile).expect_err("the module must be refused")
}

fn feature_of(error: ModuleError) -> ForbiddenFeature {
    match error {
        ModuleError::ForbiddenFeature { feature, .. } => feature,
        other => panic!("expected a forbidden feature, got {other:?}"),
    }
}

fn cap_of(error: ModuleError) -> (StructuralCap, u64, u64) {
    match error {
        ModuleError::CapExceeded { cap, actual, max } => (cap, actual, max),
        other => panic!("expected a cap, got {other:?}"),
    }
}

#[test]
fn should_admit_the_minimal_module_and_prepare_it() {
    let profile = latest_profile();
    let bytes = wasm(MINIMAL);
    let submitted = validate_submitted(&bytes, &profile).expect("admitted");
    assert_eq!(submitted.interface().exports.len(), 2);
    assert!(submitted.interface().host_imports.is_empty());
    assert_eq!(submitted.interface().memory.initial_pages, 1);
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    assert_eq!(
        prepared.instrumentation.thunks, 2,
        "both exports get a thunk"
    );
    assert_eq!(prepared.instrumentation.wrapped_call_sites, 0);
    assert_ne!(prepared.canonical_hash.0, prepared.prepared_hash.0);
    assert_eq!(prepared.preparation_generation, profile.generation);
    assert!(prepared.structure.prepared_bytes > prepared.structure.canonical_bytes);
}

#[test]
fn should_admit_every_host_envelope_import_with_its_exact_signature() {
    let profile = latest_profile();
    let bytes = wasm(&module_with(
        r#"
  (import "dash_host" "host_call" (func (param i32 i32 i32) (result i64)))
  (import "dash_host" "response_len" (func (param i32) (result i32)))
  (import "dash_host" "response_read" (func (param i32 i32 i32) (result i32)))
  (import "dash_host" "response_release" (func (param i32)))
"#,
    ));
    let submitted = validate_submitted(&bytes, &profile).expect("admitted");
    let imported: Vec<(&str, &FuncSignature)> = submitted
        .interface()
        .host_imports
        .iter()
        .map(|import| (import.name.as_str(), &import.signature))
        .collect();
    let expected = host_envelope();
    let expected: Vec<(&str, &FuncSignature)> = expected
        .iter()
        .map(|(name, signature)| (*name, signature))
        .collect();
    assert_eq!(imported, expected);
}

#[test]
fn should_admit_every_admitted_operator_family() {
    // Sign extension, saturating conversions, multi-value, bulk memory, reference types
    // (funcref table operations), floats: the toolchain's default emissions.
    let profile = latest_profile();
    let bytes = wasm(&module_with(
        r#"
  (table 4 8 funcref)
  (data (i32.const 0) "abcd")
  (data "passive")
  (elem func $pair)
  (func $pair (result i32 i32) i32.const 1 i32.const 2)
  (func $ops (result i64)
    (local f64)
    (i32.extend8_s (i32.const 200)) drop
    (i64.extend32_s (i64.const 5)) drop
    (i32.trunc_sat_f32_s (f32.const 1.5)) drop
    (i64.trunc_sat_f64_u (f64.const 3.5)) drop
    (memory.copy (i32.const 0) (i32.const 8) (i32.const 4))
    (memory.fill (i32.const 16) (i32.const 0) (i32.const 4))
    (memory.init 1 (i32.const 32) (i32.const 0) (i32.const 4))
    data.drop 1
    (table.set (i32.const 0) (ref.func $pair))
    (table.get (i32.const 0)) ref.is_null drop
    (table.grow (ref.null func) (i32.const 1)) drop
    table.size drop
    (table.fill (i32.const 0) (ref.null func) (i32.const 1))
    (select (i32.const 1) (i32.const 2) (i32.const 0)) drop
    call $pair i32.add drop
    (f64.sqrt (f64.const 2)) local.set 0
    i64.const 7)
"#,
    ));
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    assert_eq!(prepared.initialization.active_data_bytes, 4);
    assert_eq!(prepared.initialization.passive_data_bytes, 7);
    assert_eq!(prepared.initialization.initial_table_elements, 4);
    assert_eq!(
        prepared.interface.table.map(|t| t.maximum_elements),
        Some(8)
    );
}

#[test]
fn should_reject_every_forbidden_feature_with_its_proposal() {
    let profile = latest_profile();
    let cases: Vec<(&str, String, ForbiddenFeature)> = vec![
        (
            "simd",
            module_with("(func (result v128) v128.const i64x2 0 0)"),
            ForbiddenFeature::Simd,
        ),
        (
            "simd operator",
            module_with("(func (param i32) (result i32) local.get 0 i32x4.splat i32x4.extract_lane 0)"),
            ForbiddenFeature::Simd,
        ),
        (
            "threads",
            "(module (memory (export \"memory\") 1 1 shared) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))".to_owned(),
            ForbiddenFeature::Threads,
        ),
        (
            "atomic operator",
            module_with("(func (result i32) i32.const 0 i32.atomic.load)"),
            ForbiddenFeature::Threads,
        ),
        (
            "memory64",
            "(module (memory (export \"memory\") i64 1) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))".to_owned(),
            ForbiddenFeature::Memory64,
        ),
        (
            "multi-memory",
            module_with("(memory 1)"),
            ForbiddenFeature::MultiMemory,
        ),
        (
            "tail call",
            module_with("(func $f (result i32) return_call $f)"),
            ForbiddenFeature::TailCall,
        ),
        (
            "extended const",
            module_with("(global i32 (i32.add (i32.const 1) (i32.const 2)))"),
            ForbiddenFeature::ExtendedConst,
        ),
        (
            "function references: call_ref",
            module_with("(type $t (func)) (func (param (ref null $t)) local.get 0 call_ref $t)"),
            ForbiddenFeature::FunctionReferences,
        ),
        (
            "function references: non-nullable funcref",
            module_with("(func (param (ref func)))"),
            ForbiddenFeature::FunctionReferences,
        ),
        (
            "gc: struct type",
            module_with("(type (struct (field i32)))"),
            ForbiddenFeature::Gc,
        ),
        (
            "gc: anyref",
            module_with("(func (param anyref))"),
            ForbiddenFeature::Gc,
        ),
        (
            "externref",
            module_with("(func (param externref))"),
            ForbiddenFeature::ExternRef,
        ),
        (
            "externref table",
            module_with("(table 1 1 externref)"),
            ForbiddenFeature::ExternRef,
        ),
        (
            "exceptions: tag",
            module_with("(tag (param i32))"),
            ForbiddenFeature::Exceptions,
        ),
        (
            "exceptions: try_table",
            module_with("(func (block (try_table (catch_all 0))))"),
            ForbiddenFeature::Exceptions,
        ),
        (
            "wide arithmetic",
            module_with("(func (result i64 i64) i64.const 1 i64.const 2 i64.const 3 i64.const 4 i64.add128)"),
            ForbiddenFeature::WideArithmetic,
        ),
        (
            "memory control",
            module_with("(func (memory.discard (i32.const 0) (i32.const 0)))"),
            ForbiddenFeature::MemoryControl,
        ),
    ];
    for (label, wat, expected) in cases {
        let bytes = match wat::parse_str(&wat) {
            Ok(bytes) => bytes,
            Err(error) => panic!("fixture `{label}` does not assemble: {error}"),
        };
        let error = reject_bytes(&bytes, &profile);
        assert_eq!(feature_of(error), expected, "fixture `{label}`");
    }
}

#[test]
fn should_reject_a_component_binary() {
    let profile = latest_profile();
    // A component header: magic, then version 0x0d 0x00 and layer 0x01 0x00.
    let bytes = b"\0asm\x0d\0\x01\0";
    assert_eq!(
        feature_of(reject_bytes(bytes, &profile)),
        ForbiddenFeature::ComponentModel
    );
}

#[test]
fn should_reject_a_start_section() {
    let profile = latest_profile();
    assert_eq!(
        reject(&module_with("(func $init) (start $init)"), &profile),
        ModuleError::StartSection
    );
}

#[test]
fn should_reject_malformed_bytes_as_invalid_not_forbidden() {
    let profile = latest_profile();
    assert!(matches!(
        reject_bytes(b"\0asm\x01\0\0\0\x01\x05\x01\x60", &profile),
        ModuleError::Invalid { .. }
    ));
    assert!(matches!(
        reject(&module_with("(func (result i32) i64.const 1)"), &profile),
        ModuleError::Invalid { .. }
    ));
}

#[test]
fn should_reject_oversized_canonical_bytes_before_decoding() {
    let bytes = wasm(MINIMAL);
    let profile = profile_with(|limits| limits.max_canonical_module_bytes = bytes.len() as u32 - 1);
    assert_eq!(
        reject_bytes(&bytes, &profile),
        ModuleError::TooLarge {
            actual: bytes.len() as u64,
            max: bytes.len() as u32 - 1
        }
    );
    let at_cap = profile_with(|limits| limits.max_canonical_module_bytes = bytes.len() as u32);
    validate_submitted(&bytes, &at_cap).expect("a module at the cap is admitted");
}

#[test]
fn should_reject_prepared_bytes_over_the_instrumented_cap() {
    let bytes = wasm(MINIMAL);
    let profile = latest_profile();
    let prepared = prepare_module(name(), &bytes, &profile).expect("prepared");
    let prepared_len = prepared.prepared_bytes.len() as u32;
    let tight = profile_with(|limits| {
        limits.max_prepared_module_bytes = prepared_len - 1;
        limits.max_canonical_module_bytes = prepared_len - 1;
    });
    assert_eq!(
        prepare_module(name(), &bytes, &tight).expect_err("over the prepared cap"),
        ModuleError::PreparedTooLarge {
            actual: u64::from(prepared_len),
            max: prepared_len - 1
        }
    );
    let at_cap = profile_with(|limits| {
        limits.max_prepared_module_bytes = prepared_len;
        limits.max_canonical_module_bytes = prepared_len;
    });
    prepare_module(name(), &bytes, &at_cap).expect("at the prepared cap");
}

/// Builds a module with `n` extra functions.
fn with_functions(n: usize) -> String {
    let functions: String = (0..n).map(|_| "(func)\n").collect();
    module_with(&functions)
}

#[test]
fn should_enforce_the_function_cap_at_cap_and_cap_plus_one() {
    // MINIMAL defines 2 functions.
    let profile = profile_with(|limits| limits.max_functions_per_module = 5);
    validate_submitted(&wasm(&with_functions(3)), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(&with_functions(4), &profile)),
        (StructuralCap::Functions, 6, 5)
    );
}

#[test]
fn should_count_imported_functions_against_the_function_cap() {
    let profile = profile_with(|limits| limits.max_functions_per_module = 3);
    let wat = module_with(
        r#"(import "dash_host" "response_len" (func (param i32) (result i32)))
           (import "dash_host" "response_release" (func (param i32)))"#,
    );
    assert_eq!(
        cap_of(reject(&wat, &profile)),
        (StructuralCap::Functions, 4, 3)
    );
}

#[test]
fn should_enforce_the_type_cap_at_cap_and_cap_plus_one() {
    // MINIMAL declares 2 types.
    let profile = profile_with(|limits| limits.max_types_per_module = 3);
    validate_submitted(&wasm(&module_with("(type (func (param i64)))")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(
            &module_with("(type (func (param i64))) (type (func (param f32)))"),
            &profile
        )),
        (StructuralCap::Types, 4, 3)
    );
}

#[test]
fn should_enforce_the_param_cap_on_params_and_results() {
    let profile = profile_with(|limits| limits.max_params_per_function = 3);
    validate_submitted(
        &wasm(&module_with(
            "(func (param i32 i32 i32) (result i32 i32 i32) local.get 0 local.get 1 local.get 2)",
        )),
        &profile,
    )
    .expect("at cap");
    assert_eq!(
        cap_of(reject(
            &module_with("(func (param i32 i32 i32 i32))"),
            &profile
        )),
        (StructuralCap::Params, 4, 3)
    );
    assert_eq!(
        cap_of(reject(
            &module_with(
                "(func (result i32 i32 i32 i32) i32.const 0 i32.const 0 i32.const 0 i32.const 0)"
            ),
            &profile
        )),
        (StructuralCap::Results, 4, 3)
    );
}

#[test]
fn should_enforce_the_locals_cap_at_cap_and_cap_plus_one() {
    let profile = profile_with(|limits| limits.max_locals_per_function = 4);
    validate_submitted(
        &wasm(&module_with("(func (local i32 i32) (local i64 i64))")),
        &profile,
    )
    .expect("at cap");
    assert_eq!(
        cap_of(reject(
            &module_with("(func (local i32 i32) (local i64 i64 f32))"),
            &profile
        )),
        (StructuralCap::Locals, 5, 4)
    );
}

#[test]
fn should_enforce_the_export_cap_at_cap_and_cap_plus_one() {
    let profile = profile_with(|limits| limits.max_exports_per_module = 4);
    validate_submitted(&wasm(&module_with("(func (export \"a\"))")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(
            &module_with("(func (export \"a\")) (func (export \"b\"))"),
            &profile
        )),
        (StructuralCap::Exports, 5, 4)
    );
}

#[test]
fn should_enforce_the_per_function_operator_cap() {
    // Body: 4 nops + implicit end = 5 operators.
    let profile = profile_with(|limits| limits.max_operators_per_function = 5);
    validate_submitted(&wasm(&module_with("(func nop nop nop nop)")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(&module_with("(func nop nop nop nop nop)"), &profile)),
        (StructuralCap::OperatorsPerFunction, 6, 5)
    );
}

#[test]
fn should_enforce_the_per_module_operator_cap_across_functions() {
    // MINIMAL's two bodies are 2 operators each (const + end): 4 total.
    let profile = profile_with(|limits| limits.max_operators_per_module = 7);
    validate_submitted(&wasm(&module_with("(func nop nop)")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(&module_with("(func nop nop nop)"), &profile)),
        (StructuralCap::OperatorsPerModule, 8, 7)
    );
}

#[test]
fn should_enforce_the_basic_block_cap() {
    // One block per body plus one per block-starting operator: `block end` adds 2.
    let profile = profile_with(|limits| limits.max_basic_blocks_per_function = 3);
    validate_submitted(&wasm(&module_with("(func (block))")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(&module_with("(func (block) (block))"), &profile)),
        (StructuralCap::BasicBlocksPerFunction, 4, 3)
    );
}

#[test]
fn should_enforce_the_nesting_cap_at_cap_and_cap_plus_one() {
    let profile = profile_with(|limits| limits.max_nesting_depth = 2);
    validate_submitted(&wasm(&module_with("(func (block (block)))")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(
            &module_with("(func (block (block (block))))"),
            &profile
        )),
        (StructuralCap::NestingDepth, 3, 2)
    );
}

#[test]
fn should_enforce_memory_page_caps() {
    let profile = profile_with(|limits| {
        limits.max_initial_memory_pages = 2;
        limits.max_memory_pages_per_instance = 4;
    });
    let at_cap = "(module (memory (export \"memory\") 2 4) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    validate_submitted(&wasm(at_cap), &profile).expect("at cap");
    let initial_over = at_cap.replace(
        "(memory (export \"memory\") 2 4)",
        "(memory (export \"memory\") 3 4)",
    );
    assert_eq!(
        cap_of(reject(&initial_over, &profile)),
        (StructuralCap::InitialMemoryPages, 3, 2)
    );
    let maximum_over = at_cap.replace(
        "(memory (export \"memory\") 2 4)",
        "(memory (export \"memory\") 2 5)",
    );
    assert_eq!(
        cap_of(reject(&maximum_over, &profile)),
        (StructuralCap::MemoryMaximumPages, 5, 4)
    );
    let no_maximum = at_cap.replace(
        "(memory (export \"memory\") 2 4)",
        "(memory (export \"memory\") 2)",
    );
    let submitted = validate_submitted(&wasm(&no_maximum), &profile).expect("no maximum is fine");
    assert_eq!(submitted.interface().memory.maximum_pages, None);
}

#[test]
fn should_enforce_the_table_element_cap_and_require_a_maximum() {
    let profile = profile_with(|limits| limits.max_table_elements = 8);
    validate_submitted(&wasm(&module_with("(table 8 8 funcref)")), &profile).expect("at cap");
    assert_eq!(
        cap_of(reject(&module_with("(table 9 9 funcref)"), &profile)),
        (StructuralCap::TableElements, 9, 8)
    );
    assert_eq!(
        cap_of(reject(&module_with("(table 1 9 funcref)"), &profile)),
        (StructuralCap::TableElements, 9, 8)
    );
    assert_eq!(
        reject(&module_with("(table 1 funcref)"), &profile),
        ModuleError::Table(TableRule::MaximumMissing)
    );
}

#[test]
fn should_enforce_the_data_segment_byte_cap() {
    let profile = profile_with(|limits| limits.max_data_segment_bytes_per_module = 6);
    validate_submitted(
        &wasm(&module_with("(data (i32.const 0) \"abc\") (data \"def\")")),
        &profile,
    )
    .expect("at cap");
    assert_eq!(
        cap_of(reject(
            &module_with("(data (i32.const 0) \"abc\") (data \"defg\")"),
            &profile
        )),
        (StructuralCap::DataSegmentBytes, 7, 6)
    );
}

#[test]
fn should_reject_a_frame_larger_than_the_logical_stack() {
    let profile = profile_with(|limits| limits.max_logical_stack_bytes = 64);
    // 32 base + 8 locals * 8 bytes = 96 > 64.
    let error = reject(
        &module_with("(func (local i64 i64 i64 i64 i64 i64 i64 i64))"),
        &profile,
    );
    assert_eq!(
        error,
        ModuleError::FrameExceedsLogicalStack {
            function: 0,
            frame_bytes: 96,
            max: 64
        }
    );
}

#[test]
fn should_enforce_memory_shape_rules() {
    let profile = latest_profile();
    let no_memory = "(module (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(no_memory, &profile),
        ModuleError::Memory(MemoryRule::Missing)
    );
    let imported = "(module (import \"env\" \"memory\" (memory 1)) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(imported, &profile),
        ModuleError::Memory(MemoryRule::Imported)
    );
    let unexported = "(module (memory 1) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(unexported, &profile),
        ModuleError::Memory(MemoryRule::NotExported)
    );
    let misnamed = "(module (memory (export \"mem\") 1) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(misnamed, &profile),
        ModuleError::Export(ExportRejection::MemoryName {
            name: "mem".to_owned()
        })
    );
}

#[test]
fn should_enforce_table_shape_rules() {
    let profile = latest_profile();
    assert_eq!(
        reject(
            &module_with("(import \"env\" \"t\" (table 1 1 funcref))"),
            &profile
        ),
        ModuleError::Table(TableRule::Imported)
    );
    assert_eq!(
        reject(
            &module_with("(table 1 1 funcref) (table 1 1 funcref)"),
            &profile
        ),
        ModuleError::Table(TableRule::Multiple)
    );
    assert_eq!(
        reject(&module_with("(table (export \"t\") 1 1 funcref)"), &profile),
        ModuleError::Export(ExportRejection::Table {
            name: "t".to_owned()
        })
    );
}

#[test]
fn should_apply_the_import_allowlist() {
    let profile = latest_profile();
    let cases = [
        (
            "(import \"env\" \"f\" (func))",
            "env",
            "f",
            ImportRejection::UnknownModule,
        ),
        (
            "(import \"dash_vm\" \"stack_bytes\" (global (mut i32)))",
            "dash_vm",
            "stack_bytes",
            ImportRejection::ReservedModule,
        ),
        (
            "(import \"dash_vm\" \"trap\" (func (param i32)))",
            "dash_vm",
            "trap",
            ImportRejection::ReservedModule,
        ),
        (
            "(import \"dash_host\" \"g\" (global i32))",
            "dash_host",
            "g",
            ImportRejection::NotAFunction,
        ),
        (
            "(import \"dash_host\" \"nope\" (func))",
            "dash_host",
            "nope",
            ImportRejection::UnknownHostFunction,
        ),
        (
            "(import \"dash_host\" \"response_len\" (func (param i64) (result i32)))",
            "dash_host",
            "response_len",
            ImportRejection::HostSignature {
                expected: FuncSignature::new(vec![ValueType::I32], vec![ValueType::I32]),
                actual: FuncSignature::new(vec![ValueType::I64], vec![ValueType::I32]),
            },
        ),
    ];
    for (import, module, name, reason) in cases {
        assert_eq!(
            reject(&module_with(import), &profile),
            ModuleError::Import {
                module: module.to_owned(),
                name: name.to_owned(),
                reason,
            },
            "import `{import}`"
        );
    }
    let bad_target = module_with("(import \"dash:Helper\" \"f\" (func))");
    assert!(matches!(
        reject(&bad_target, &profile),
        ModuleError::Import {
            reason: ImportRejection::InvalidTargetName(_),
            ..
        }
    ));
    let good_target = module_with("(import \"dash:helper\" \"f\" (func (param i32)))");
    let submitted = validate_submitted(&wasm(&good_target), &profile).expect("admitted");
    assert_eq!(submitted.interface().internal_imports.len(), 1);
    assert_eq!(
        submitted.interface().internal_imports[0].target.as_str(),
        "helper"
    );
}

#[test]
fn should_apply_the_export_rules() {
    let profile = latest_profile();
    let no_alloc = "(module (memory (export \"memory\") 1) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(no_alloc, &profile),
        ModuleError::Export(ExportRejection::MissingAlloc)
    );
    let bad_alloc = "(module (memory (export \"memory\") 1) (func (export \"dash_alloc\") (param i64) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(bad_alloc, &profile),
        ModuleError::Export(ExportRejection::AllocSignature {
            expected: alloc_signature()
        })
    );
    let no_entry = "(module (memory (export \"memory\") 1) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(no_entry, &profile),
        ModuleError::Export(ExportRejection::NoEntry {
            expected: entry_signature()
        })
    );
    let reexport = module_with("(import \"dash_host\" \"response_release\" (func $r (param i32))) (export \"rel\" (func $r))");
    assert_eq!(
        reject(&reexport, &profile),
        ModuleError::Export(ExportRejection::OfImportedFunction {
            name: "rel".to_owned()
        })
    );
    let func_named_memory = "(module (memory 1) (func (export \"memory\")) (func (export \"dash_alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) i64.const 0))";
    assert_eq!(
        reject(func_named_memory, &profile),
        ModuleError::Export(ExportRejection::MemoryNotMemory)
    );
    let submitted = validate_submitted(
        &wasm(&module_with("(global (export \"g\") i32 (i32.const 1))")),
        &profile,
    )
    .expect("an exported global is not part of the interface but is admitted");
    assert_eq!(submitted.interface().exports.len(), 2);
}

#[test]
fn should_strip_custom_sections_so_only_the_canonical_hash_changes() {
    let profile = latest_profile();
    let plain = wasm(MINIMAL);
    let named = wasm(&MINIMAL.replace("(func (export \"run\")", "(func $run (export \"run\")"));
    let mut with_custom = plain.clone();
    // A custom section at the end: id 0, size 6, name length 4 "test", one data byte.
    with_custom.extend_from_slice(&[0, 6, 4, b't', b'e', b's', b't', 0xaa]);
    assert_ne!(plain, with_custom);
    let a = prepare_module(name(), &plain, &profile).expect("prepared");
    let b = prepare_module(name(), &with_custom, &profile).expect("prepared");
    let c = prepare_module(name(), &named, &profile).expect("prepared");
    assert_ne!(a.canonical_hash, b.canonical_hash);
    assert_eq!(a.prepared_hash, b.prepared_hash);
    assert_eq!(a.prepared_bytes, b.prepared_bytes);
    assert_eq!(
        a.prepared_hash, c.prepared_hash,
        "the name section is stripped"
    );
}

#[test]
fn should_produce_identical_output_for_identical_input() {
    let profile = latest_profile();
    let bytes = wasm(&module_with(
        "(table 1 1 funcref) (elem (i32.const 0) $f) (func $f (call $f))",
    ));
    let a = prepare_module(name(), &bytes, &profile).expect("prepared");
    let b = prepare_module(name(), &bytes, &profile).expect("prepared");
    assert_eq!(a, b);
}
