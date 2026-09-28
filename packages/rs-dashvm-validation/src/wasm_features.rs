//! The admitted WebAssembly feature set of preparation generation 0 and the classification of
//! everything outside it.
//!
//! Admission is an explicit allowlist, never the parser's defaults: the pinned `wasmparser`
//! validates with exactly [`admitted_features`], and independently of it every type, section
//! flag and operator is classified here so a rejection names the proposal it belongs to. The two
//! checks agree by construction; the classification runs first so the typed error wins, and the
//! validator's own message is mapped through [`classify_validator_message`] as a backstop.
//!
//! Admitted (matching what the pinned Rust toolchain emits for `wasm32-unknown-unknown` by
//! default): mutable globals, sign extension, saturating float to int, multi-value, bulk
//! memory, and reference types restricted to nullable `funcref`. Scalar floats are admitted
//! (the engine profile canonicalises NaNs). Everything else is rejected.

use crate::bundle::ValueType;
use crate::errors::ForbiddenFeature;
use wasmparser::{
    AbstractHeapType, GlobalType, HeapType, MemArg, MemoryType, Operator, RefType, TableType,
    ValType, WasmFeatures,
};

/// The feature set the validator runs with.
pub fn admitted_features() -> WasmFeatures {
    WasmFeatures::FLOATS
        | WasmFeatures::MUTABLE_GLOBAL
        | WasmFeatures::SATURATING_FLOAT_TO_INT
        | WasmFeatures::SIGN_EXTENSION
        | WasmFeatures::MULTI_VALUE
        | WasmFeatures::BULK_MEMORY
        | WasmFeatures::REFERENCE_TYPES
}

/// Classifies a value type: the admitted projection, or the proposal it needs.
pub fn classify_val_type(ty: ValType) -> Result<ValueType, ForbiddenFeature> {
    match ty {
        ValType::I32 => Ok(ValueType::I32),
        ValType::I64 => Ok(ValueType::I64),
        ValType::F32 => Ok(ValueType::F32),
        ValType::F64 => Ok(ValueType::F64),
        ValType::V128 => Err(ForbiddenFeature::Simd),
        ValType::Ref(reference) => classify_ref_type(reference),
    }
}

/// Classifies a reference type: nullable `funcref` is the only admitted one.
pub fn classify_ref_type(reference: RefType) -> Result<ValueType, ForbiddenFeature> {
    if !reference.is_nullable() {
        return Err(ForbiddenFeature::FunctionReferences);
    }
    classify_heap_type(reference.heap_type()).map(|()| ValueType::FuncRef)
}

/// Classifies a heap type as used by `ref.null`: only the abstract, unshared `func` heap type
/// is admitted.
pub fn classify_heap_type(heap_type: HeapType) -> Result<(), ForbiddenFeature> {
    match heap_type {
        HeapType::Concrete(_) => Err(ForbiddenFeature::FunctionReferences),
        HeapType::Abstract { shared: true, .. } => Err(ForbiddenFeature::SharedEverythingThreads),
        HeapType::Abstract { shared: false, ty } => match ty {
            AbstractHeapType::Func => Ok(()),
            AbstractHeapType::Extern => Err(ForbiddenFeature::ExternRef),
            AbstractHeapType::Exn | AbstractHeapType::NoExn => Err(ForbiddenFeature::Exceptions),
            AbstractHeapType::Cont | AbstractHeapType::NoCont => {
                Err(ForbiddenFeature::StackSwitching)
            }
            AbstractHeapType::Any
            | AbstractHeapType::None
            | AbstractHeapType::NoExtern
            | AbstractHeapType::NoFunc
            | AbstractHeapType::Eq
            | AbstractHeapType::Struct
            | AbstractHeapType::Array
            | AbstractHeapType::I31 => Err(ForbiddenFeature::Gc),
        },
    }
}

/// Classifies the flags of a memory type. The count of memories and their limits are checked
/// by the shape rules, not here.
pub fn classify_memory_type(memory: &MemoryType) -> Result<(), ForbiddenFeature> {
    if memory.memory64 {
        return Err(ForbiddenFeature::Memory64);
    }
    if memory.shared {
        return Err(ForbiddenFeature::Threads);
    }
    if memory.page_size_log2.is_some() {
        return Err(ForbiddenFeature::CustomPageSizes);
    }
    Ok(())
}

/// Classifies the flags and element type of a table type.
pub fn classify_table_type(table: &TableType) -> Result<(), ForbiddenFeature> {
    if table.table64 {
        return Err(ForbiddenFeature::Memory64);
    }
    if table.shared {
        return Err(ForbiddenFeature::SharedEverythingThreads);
    }
    classify_ref_type(table.element_type).map(|_| ())
}

/// Classifies the flags and content type of a global type.
pub fn classify_global_type(global: &GlobalType) -> Result<ValueType, ForbiddenFeature> {
    if global.shared {
        return Err(ForbiddenFeature::SharedEverythingThreads);
    }
    classify_val_type(global.content_type)
}

/// Classifies one operator of a constant expression: arithmetic needs the extended-const
/// proposal. Every other non-constant operator is left to the validator.
pub fn classify_const_operator(op: &Operator<'_>) -> Option<ForbiddenFeature> {
    match op {
        Operator::I32Add
        | Operator::I32Sub
        | Operator::I32Mul
        | Operator::I64Add
        | Operator::I64Sub
        | Operator::I64Mul => Some(ForbiddenFeature::ExtendedConst),
        other => classify_operator(other),
    }
}

fn check_memarg(memarg: &MemArg) -> Option<ForbiddenFeature> {
    if memarg.memory != 0 {
        return Some(ForbiddenFeature::MultiMemory);
    }
    if memarg.offset > u64::from(u32::MAX) {
        return Some(ForbiddenFeature::Memory64);
    }
    None
}

fn check_memory_index(memory: u32) -> Option<ForbiddenFeature> {
    (memory != 0).then_some(ForbiddenFeature::MultiMemory)
}

/// Maps the proposal group `wasmparser` files an operator under to the feature it needs.
/// Every group the pinned crate knows must be listed; a new group is a compile error, which is
/// the intent: an operator that reaches the engine without a classification here is a hole in
/// the allowlist.
macro_rules! proposal_feature {
    (mvp) => {
        None
    };
    (sign_extension) => {
        None
    };
    (saturating_float_to_int) => {
        None
    };
    (bulk_memory) => {
        None
    };
    (reference_types) => {
        None
    };
    (simd) => {
        Some(ForbiddenFeature::Simd)
    };
    (relaxed_simd) => {
        Some(ForbiddenFeature::RelaxedSimd)
    };
    (threads) => {
        Some(ForbiddenFeature::Threads)
    };
    (shared_everything_threads) => {
        Some(ForbiddenFeature::SharedEverythingThreads)
    };
    (tail_call) => {
        Some(ForbiddenFeature::TailCall)
    };
    (function_references) => {
        Some(ForbiddenFeature::FunctionReferences)
    };
    (gc) => {
        Some(ForbiddenFeature::Gc)
    };
    (exceptions) => {
        Some(ForbiddenFeature::Exceptions)
    };
    (legacy_exceptions) => {
        Some(ForbiddenFeature::LegacyExceptions)
    };
    (stack_switching) => {
        Some(ForbiddenFeature::StackSwitching)
    };
    (wide_arithmetic) => {
        Some(ForbiddenFeature::WideArithmetic)
    };
    (memory_control) => {
        Some(ForbiddenFeature::MemoryControl)
    };
}

/// Checks the immediates of an admitted operator. Field names are the ones `wasmparser` uses
/// in its operator table; any field not listed here needs no check.
macro_rules! check_immediate {
    (memarg, $value:ident) => {
        if let Some(feature) = check_memarg($value) {
            return Some(feature);
        }
    };
    (mem, $value:ident) => {
        if let Some(feature) = check_memory_index(*$value) {
            return Some(feature);
        }
    };
    (dst_mem, $value:ident) => {
        if let Some(feature) = check_memory_index(*$value) {
            return Some(feature);
        }
    };
    (src_mem, $value:ident) => {
        if let Some(feature) = check_memory_index(*$value) {
            return Some(feature);
        }
    };
    (hty, $value:ident) => {
        if let Err(feature) = classify_heap_type(*$value) {
            return Some(feature);
        }
    };
    (ty, $value:ident) => {
        if let Err(feature) = classify_val_type(*$value) {
            return Some(feature);
        }
    };
    (tys, $value:ident) => {
        for ty in $value.iter() {
            if let Err(feature) = classify_val_type(*ty) {
                return Some(feature);
            }
        }
    };
    ($other:ident, $value:ident) => {};
}

macro_rules! classify_by_proposal {
    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        /// Classifies an operator of a function body: `None` when admitted, otherwise the
        /// proposal it needs. Admitted operators also have their immediates checked (memory
        /// index, 64-bit offsets, reference and select types).
        pub fn classify_operator(op: &Operator<'_>) -> Option<ForbiddenFeature> {
            #[allow(unused_variables)]
            match op {
                $(
                    Operator::$op $({ $($arg),* })? => {
                        let feature: Option<ForbiddenFeature> = proposal_feature!($proposal);
                        if feature.is_some() {
                            return feature;
                        }
                        $($( check_immediate!($arg, $arg); )*)?
                        None
                    }
                )*
                _ => Some(ForbiddenFeature::UnknownOperator),
            }
        }
    };
}

wasmparser::for_each_operator!(classify_by_proposal);

/// Maps a validator message to the proposal it names, for the cases the classification above
/// does not reach first (the validator sees some encodings before any section reader does).
pub fn classify_validator_message(message: &str) -> Option<ForbiddenFeature> {
    let checks: [(&str, ForbiddenFeature); 16] = [
        ("relaxed SIMD", ForbiddenFeature::RelaxedSimd),
        ("SIMD", ForbiddenFeature::Simd),
        (
            "shared-everything-threads",
            ForbiddenFeature::SharedEverythingThreads,
        ),
        ("threads", ForbiddenFeature::Threads),
        ("memory64", ForbiddenFeature::Memory64),
        ("tail call", ForbiddenFeature::TailCall),
        ("function references", ForbiddenFeature::FunctionReferences),
        ("gc", ForbiddenFeature::Gc),
        ("exception", ForbiddenFeature::Exceptions),
        ("stack switching", ForbiddenFeature::StackSwitching),
        ("wide arithmetic", ForbiddenFeature::WideArithmetic),
        ("memory control", ForbiddenFeature::MemoryControl),
        ("custom page sizes", ForbiddenFeature::CustomPageSizes),
        ("component", ForbiddenFeature::ComponentModel),
        ("multi-memory", ForbiddenFeature::MultiMemory),
        ("extended const", ForbiddenFeature::ExtendedConst),
    ];
    if let Some(op) = message.strip_prefix("constant expression required: non-constant operator: ")
    {
        if matches!(
            op,
            "i32.add" | "i32.sub" | "i32.mul" | "i64.add" | "i64.sub" | "i64.mul"
        ) {
            return Some(ForbiddenFeature::ExtendedConst);
        }
    }
    checks
        .iter()
        .find(|(needle, _)| message.contains(needle))
        .map(|(_, feature)| *feature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_admit_exactly_the_toolchain_default_proposals() {
        let features = admitted_features();
        assert!(features.mutable_global());
        assert!(features.sign_extension());
        assert!(features.saturating_float_to_int());
        assert!(features.multi_value());
        assert!(features.bulk_memory());
        assert!(features.reference_types());
        assert!(features.floats());
        for (name, enabled) in [
            ("simd", features.simd()),
            ("relaxed_simd", features.relaxed_simd()),
            ("threads", features.threads()),
            (
                "shared_everything_threads",
                features.shared_everything_threads(),
            ),
            ("tail_call", features.tail_call()),
            ("multi_memory", features.multi_memory()),
            ("exceptions", features.exceptions()),
            ("legacy_exceptions", features.legacy_exceptions()),
            ("memory64", features.memory64()),
            ("extended_const", features.extended_const()),
            ("component_model", features.component_model()),
            ("function_references", features.function_references()),
            ("memory_control", features.memory_control()),
            ("gc", features.gc()),
            ("gc_types", features.gc_types()),
            ("custom_page_sizes", features.custom_page_sizes()),
            ("stack_switching", features.stack_switching()),
            ("wide_arithmetic", features.wide_arithmetic()),
        ] {
            assert!(!enabled, "{name} must not be admitted");
        }
    }

    #[test]
    fn should_classify_admitted_operators_as_admitted_and_others_by_proposal() {
        assert_eq!(classify_operator(&Operator::I32Add), None);
        assert_eq!(classify_operator(&Operator::I32Extend8S), None);
        assert_eq!(classify_operator(&Operator::I32TruncSatF32S), None);
        assert_eq!(
            classify_operator(&Operator::MemoryFill { mem: 0 }),
            None,
            "bulk memory on memory 0 is admitted"
        );
        assert_eq!(
            classify_operator(&Operator::MemoryFill { mem: 1 }),
            Some(ForbiddenFeature::MultiMemory)
        );
        assert_eq!(
            classify_operator(&Operator::ReturnCall { function_index: 0 }),
            Some(ForbiddenFeature::TailCall)
        );
        assert_eq!(
            classify_operator(&Operator::RefNull {
                hty: HeapType::EXTERN
            }),
            Some(ForbiddenFeature::ExternRef)
        );
        assert_eq!(
            classify_operator(&Operator::RefNull {
                hty: HeapType::FUNC
            }),
            None
        );
        assert_eq!(
            classify_operator(&Operator::CallRef { type_index: 0 }),
            Some(ForbiddenFeature::FunctionReferences)
        );
        assert_eq!(
            classify_operator(&Operator::I64Add128),
            Some(ForbiddenFeature::WideArithmetic)
        );
        assert_eq!(
            classify_operator(&Operator::MemoryDiscard { mem: 0 }),
            Some(ForbiddenFeature::MemoryControl)
        );
        assert_eq!(
            classify_const_operator(&Operator::I32Add),
            Some(ForbiddenFeature::ExtendedConst)
        );
        assert_eq!(
            classify_const_operator(&Operator::I32Const { value: 1 }),
            None
        );
    }

    #[test]
    fn should_classify_types() {
        assert_eq!(classify_val_type(ValType::I32), Ok(ValueType::I32));
        assert_eq!(classify_val_type(ValType::FUNCREF), Ok(ValueType::FuncRef));
        assert_eq!(
            classify_val_type(ValType::EXTERNREF),
            Err(ForbiddenFeature::ExternRef)
        );
        assert_eq!(
            classify_val_type(ValType::V128),
            Err(ForbiddenFeature::Simd)
        );
        assert_eq!(
            classify_ref_type(RefType::FUNC),
            Err(ForbiddenFeature::FunctionReferences),
            "a non-nullable funcref needs typed function references"
        );
        assert_eq!(
            classify_val_type(ValType::Ref(RefType::ANYREF)),
            Err(ForbiddenFeature::Gc)
        );
    }

    #[test]
    fn should_map_validator_messages_to_proposals() {
        assert_eq!(
            classify_validator_message("relaxed SIMD support is not enabled"),
            Some(ForbiddenFeature::RelaxedSimd)
        );
        assert_eq!(
            classify_validator_message("unexpected SIMD opcode: 0xfd"),
            Some(ForbiddenFeature::Simd)
        );
        assert_eq!(
            classify_validator_message(
                "constant expression required: non-constant operator: i32.add"
            ),
            Some(ForbiddenFeature::ExtendedConst)
        );
        assert_eq!(
            classify_validator_message(
                "constant expression required: non-constant operator: i32.load"
            ),
            None
        );
        assert_eq!(classify_validator_message("type mismatch"), None);
    }
}
