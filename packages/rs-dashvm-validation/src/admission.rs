//! The two validators of preparation generation 0.
//!
//! [`validate_submitted`] applies every admission rule to the bytes an author submitted and
//! returns a [`SubmittedModule`], the proof of admission that is the only input the
//! instrumenter accepts. [`validate_prepared`] re-runs the same validation and measurement on
//! the instrumenter's output and then checks provenance: the output must be exactly the
//! submitted module plus what the instrumenter reported, nothing more and nothing less. Any
//! drift is [`ModuleError::Internal`], a node fault: the build contradicted itself, and no
//! guest is charged for that.

use crate::abi_names::{STACK_BYTES_GLOBAL, STACK_DEPTH_GLOBAL, TRAP_IMPORT, VM_MODULE};
use crate::abi_validation::interface_of_submitted;
use crate::bundle::{ModuleInterface, ValueType};
use crate::errors::ModuleError;
use crate::instrumentation::rewrite::IndexPlan;
use crate::instrumentation::thunks::{
    ENTER_OPERATORS, LEAVE_OPERATORS, THUNK_FIXED_OPERATORS, WRAP_OPERATORS,
};
use crate::instrumentation::{
    InstrumentationReport, HELPER_FUNCTIONS, INJECTED_FUNCTION_IMPORTS, INJECTED_GLOBAL_IMPORTS,
    INJECTED_IMPORTS,
};
use crate::profile::PreparationProfile;
use crate::structure::{measure, ImportKind, ModuleFacts, Stage};
use wasmparser::ExternalKind;

/// A module that passed admission. Holds the facts the instrumenter and the provenance check
/// read; cannot be constructed outside this module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedModule {
    pub(crate) facts: ModuleFacts,
    pub(crate) interface: ModuleInterface,
    pub(crate) plan: IndexPlan,
}

impl SubmittedModule {
    /// The interface admission derived.
    pub fn interface(&self) -> &ModuleInterface {
        &self.interface
    }
}

/// Applies every admission rule to submitted canonical bytes.
pub fn validate_submitted(
    canonical_bytes: &[u8],
    profile: &PreparationProfile,
) -> Result<SubmittedModule, ModuleError> {
    let facts = measure(canonical_bytes, profile, Stage::Submitted)?;
    let interface = interface_of_submitted(&facts, profile)?;
    let plan = IndexPlan::new(&facts, profile.limits.max_logical_stack_bytes)?;
    Ok(SubmittedModule {
        facts,
        interface,
        plan,
    })
}

fn internal(detail: impl Into<String>) -> ModuleError {
    ModuleError::Internal(detail.into())
}

/// Re-validates instrumented bytes and checks their provenance against the submitted module
/// and the instrumenter's report. Returns the prepared facts on success.
pub(crate) fn validate_prepared(
    prepared_bytes: &[u8],
    profile: &PreparationProfile,
    submitted: &SubmittedModule,
    report: &InstrumentationReport,
) -> Result<ModuleFacts, ModuleError> {
    let prepared = measure(prepared_bytes, profile, Stage::Prepared)?;
    let original = &submitted.facts;
    let plan = &submitted.plan;

    // Imports: exactly the three injected entries first, then the submitted imports unchanged.
    let expected_imports = INJECTED_IMPORTS as usize + original.imports.len();
    if prepared.imports.len() != expected_imports {
        return Err(internal(format!(
            "prepared module has {} imports, expected {expected_imports}",
            prepared.imports.len()
        )));
    }
    let injected = &prepared.imports[..INJECTED_IMPORTS as usize];
    let expected_injected = [
        (
            TRAP_IMPORT,
            ImportKind::Function {
                type_index: plan.helper_type,
            },
        ),
        (
            STACK_BYTES_GLOBAL,
            ImportKind::Global {
                value: ValueType::I32,
                mutable: true,
            },
        ),
        (
            STACK_DEPTH_GLOBAL,
            ImportKind::Global {
                value: ValueType::I32,
                mutable: true,
            },
        ),
    ];
    for (position, (import, (name, kind))) in injected.iter().zip(expected_injected).enumerate() {
        if import.module != VM_MODULE || import.name != name || import.kind != kind {
            return Err(internal(format!(
                "injected import {position} is `{}`.`{}` {:?}, expected `{VM_MODULE}`.`{name}` {kind:?}",
                import.module, import.name, import.kind
            )));
        }
    }
    let carried = &prepared.imports[INJECTED_IMPORTS as usize..];
    if carried != original.imports.as_slice() {
        return Err(internal(
            "submitted imports were not carried through unchanged",
        ));
    }
    if prepared
        .imports
        .iter()
        .skip(INJECTED_IMPORTS as usize)
        .any(|import| import.module == VM_MODULE)
    {
        return Err(internal("a submitted import claims the dash_vm module"));
    }

    // Types: the submitted types, plus the helper type only when it was added.
    let expected_types = original.types.len() + usize::from(plan.helper_type_added);
    if prepared.types.len() != expected_types
        || prepared.types[..original.types.len()] != original.types[..]
    {
        return Err(internal("the type section drifted"));
    }

    // Functions: submitted definitions, two helpers, one thunk per reported thunk.
    let expected_defined =
        original.functions.len() + HELPER_FUNCTIONS as usize + report.thunks as usize;
    if prepared.functions.len() != expected_defined
        || report.thunks as usize != plan.thunk_targets.len()
    {
        return Err(internal(format!(
            "prepared module defines {} functions, expected {expected_defined}",
            prepared.functions.len()
        )));
    }
    for (position, (prepared_function, original_function)) in prepared
        .functions
        .iter()
        .zip(&original.functions)
        .enumerate()
    {
        if prepared_function.type_index != original_function.type_index
            || prepared_function.locals != original_function.locals
        {
            return Err(internal(format!(
                "function {position} changed type or locals"
            )));
        }
        let expected_operators = u64::from(original_function.operators)
            + u64::from(original_function.calls_to_defined) * WRAP_OPERATORS;
        if u64::from(prepared_function.operators) != expected_operators {
            return Err(internal(format!(
                "function {position} has {} operators, expected {expected_operators}",
                prepared_function.operators
            )));
        }
    }
    let helpers = &prepared.functions[original.functions.len()..][..HELPER_FUNCTIONS as usize];
    if u64::from(helpers[0].operators) != ENTER_OPERATORS
        || u64::from(helpers[1].operators) != LEAVE_OPERATORS
        || helpers
            .iter()
            .any(|helper| helper.type_index != plan.helper_type)
    {
        return Err(internal("the accounting helpers drifted"));
    }
    let thunks = &prepared.functions[original.functions.len() + HELPER_FUNCTIONS as usize..];
    for (thunk, target) in thunks.iter().zip(&plan.thunk_targets) {
        let target_function = original
            .functions
            .get((*target - plan.imported) as usize)
            .ok_or_else(|| internal("thunk target out of range"))?;
        let signature = original
            .types
            .get(target_function.type_index as usize)
            .ok_or_else(|| internal("thunk type out of range"))?;
        let expected_operators = THUNK_FIXED_OPERATORS + signature.params.len() as u64;
        if thunk.type_index != target_function.type_index
            || u64::from(thunk.operators) != expected_operators
        {
            return Err(internal(format!("the thunk of function {target} drifted")));
        }
    }
    let wrapped: u64 = original
        .functions
        .iter()
        .map(|function| u64::from(function.calls_to_defined))
        .sum();
    if wrapped != u64::from(report.wrapped_call_sites) {
        return Err(internal(format!(
            "report claims {} wrapped call sites, the code has {wrapped}",
            report.wrapped_call_sites
        )));
    }

    // Exports: same names and kinds in the same order; function exports point at thunks,
    // globals shift by the injected globals, the memory is unchanged.
    if prepared.exports.len() != original.exports.len() {
        return Err(internal("the export count drifted"));
    }
    for (prepared_export, original_export) in prepared.exports.iter().zip(&original.exports) {
        if prepared_export.name != original_export.name
            || prepared_export.kind != original_export.kind
        {
            return Err(internal(format!(
                "export `{}` drifted",
                original_export.name
            )));
        }
        let expected_index = match original_export.kind {
            ExternalKind::Func => plan
                .reference_target(original_export.index)
                .map_err(|error| internal(error.to_string()))?,
            ExternalKind::Global => original_export.index + INJECTED_GLOBAL_IMPORTS,
            _ => original_export.index,
        };
        if prepared_export.index != expected_index {
            return Err(internal(format!(
                "export `{}` points at {}, expected {expected_index}",
                original_export.name, prepared_export.index
            )));
        }
    }

    // Memory, table, segments, start and custom sections.
    if prepared.memory != original.memory || prepared.table != original.table {
        return Err(internal("the memory or table shape drifted"));
    }
    if prepared.element_segments != original.element_segments
        || prepared.data_segments != original.data_segments
        || prepared.initialization != original.initialization
    {
        return Err(internal("the segments drifted"));
    }
    if prepared.has_start {
        return Err(internal("a start section appeared"));
    }
    if prepared.has_custom_sections {
        return Err(internal("a custom section survived"));
    }
    // Every function reference in the prepared module lands on a thunk, an import or a helper,
    // never directly on a submitted definition.
    let first_definition = INJECTED_FUNCTION_IMPORTS + plan.imported;
    let first_helper = plan.enter;
    if prepared
        .referenced_functions
        .iter()
        .any(|index| (first_definition..first_helper).contains(index))
    {
        return Err(internal("a reference bypasses the thunks"));
    }
    Ok(prepared)
}
