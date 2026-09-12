//! The `Reencode` implementation that carries the injected imports through every index space
//! and inserts the accounting around calls, table entries and exports.
//!
//! Prepared index spaces, given `I` imported and `D` defined functions in the submitted module:
//!
//! | space     | prepared index                                                  |
//! |-----------|-----------------------------------------------------------------|
//! | functions | `0` trap; `1..=I` submitted imports; `I+1..=I+D` submitted definitions; `I+D+1` enter; `I+D+2` leave; `I+D+3..` thunks in ascending order of the function they wrap |
//! | globals   | `0` stack bytes; `1` stack depth; `2..` submitted globals        |
//! | types     | submitted types unchanged; the `(i32) -> ()` helper type appended only when the module has none |
//!
//! Tables, memories, data and element segment indices are unchanged.

use crate::abi_names::{STACK_BYTES_GLOBAL, STACK_DEPTH_GLOBAL, TRAP_IMPORT, VM_MODULE};
use crate::bundle::FuncSignature;
use crate::errors::ModuleError;
use crate::instrumentation::frame_cost::frame_bytes;
use crate::instrumentation::thunks::{enter_body, leave_body, thunk_body, COUNTER_TYPE};
use crate::instrumentation::{
    InstrumentationReport, HELPER_FUNCTIONS, INJECTED_FUNCTION_IMPORTS, INJECTED_GLOBAL_IMPORTS,
};
use crate::structure::ModuleFacts;
use crate::wasm_features::admitted_features;
use std::fmt;
use wasm_encoder::reencode::{utils, Error, Reencode};
use wasm_encoder::{
    CodeSection, Elements, EntityType, ExportKind, ExportSection, FunctionSection, GlobalType,
    ImportSection, Instruction, Module, SectionId, TypeSection, ValType,
};
use wasmparser::{ElementItems, ExternalKind, Operator, Parser};

/// A contradiction between the facts the measurement pass recorded and what the rewriter
/// encounters while re-reading the same bytes. Always an internal error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteError(String);

impl fmt::Display for RewriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RewriteError {}

/// The index plan of one instrumentation run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexPlan {
    /// Submitted imported functions.
    pub imported: u32,
    /// Submitted defined functions.
    pub defined: u32,
    /// Type index of `(i32) -> ()`.
    pub helper_type: u32,
    /// Whether the helper type was appended (the submitted module had no such type).
    pub helper_type_added: bool,
    /// Prepared index of `enter`.
    pub enter: u32,
    /// Prepared index of `leave`.
    pub leave: u32,
    /// Prepared index of the first thunk.
    pub first_thunk: u32,
    /// Submitted function indices that get a thunk, ascending.
    pub thunk_targets: Vec<u32>,
    /// Frame cost per submitted defined function, in function-section order.
    pub frame_costs: Vec<i32>,
}

impl IndexPlan {
    /// Computes the plan from the measured facts. `logical_stack_cap` bounds every frame.
    pub(crate) fn new(facts: &ModuleFacts, logical_stack_cap: u32) -> Result<Self, ModuleError> {
        let imported = facts.imported_functions();
        let defined = facts.functions.len() as u32;
        let helper_signature = crate::abi_names::trap_signature();
        let (helper_type, helper_type_added) = match facts
            .types
            .iter()
            .position(|signature| *signature == helper_signature)
        {
            Some(index) => (index as u32, false),
            None => (facts.types.len() as u32, true),
        };
        let enter = INJECTED_FUNCTION_IMPORTS + imported + defined;
        let leave = enter + 1;
        let first_thunk = leave + 1;
        let thunk_targets: Vec<u32> =
            facts
                .referenced_functions
                .iter()
                .copied()
                .chain(facts.exports.iter().filter_map(|export| {
                    (export.kind == ExternalKind::Func).then_some(export.index)
                }))
                .filter(|index| *index >= imported)
                .collect::<std::collections::BTreeSet<u32>>()
                .into_iter()
                .collect();
        let cap = u64::from(logical_stack_cap).min(i32::MAX as u64);
        let mut frame_costs = Vec::with_capacity(facts.functions.len());
        for (position, function) in facts.functions.iter().enumerate() {
            let signature = facts
                .types
                .get(function.type_index as usize)
                .ok_or_else(|| ModuleError::Internal("function type out of range".to_owned()))?;
            let bytes = frame_bytes(signature, &function.locals, function.max_operand_slots);
            if bytes > cap {
                return Err(ModuleError::FrameExceedsLogicalStack {
                    function: imported + position as u32,
                    frame_bytes: bytes,
                    max: logical_stack_cap,
                });
            }
            frame_costs.push(bytes as i32);
        }
        Ok(Self {
            imported,
            defined,
            helper_type,
            helper_type_added,
            enter,
            leave,
            first_thunk,
            thunk_targets,
            frame_costs,
        })
    }

    /// Prepared index of a submitted function index.
    pub fn shifted(&self, submitted: u32) -> u32 {
        submitted + INJECTED_FUNCTION_IMPORTS
    }

    /// Prepared index a reference (table entry, export, `ref.func`) to a submitted function
    /// resolves to: the thunk for a defined function, the shifted index for an import.
    pub fn reference_target(&self, submitted: u32) -> Result<u32, RewriteError> {
        if submitted < self.imported {
            return Ok(self.shifted(submitted));
        }
        self.thunk_targets
            .binary_search(&submitted)
            .map(|position| self.first_thunk + position as u32)
            .map_err(|_| RewriteError(format!("function {submitted} has no thunk")))
    }

    /// The report the plan implies once the call sites are counted.
    pub fn report(&self, wrapped_call_sites: u32) -> InstrumentationReport {
        InstrumentationReport {
            wrapped_call_sites,
            thunks: self.thunk_targets.len() as u32,
            frame_bytes: self.frame_costs.iter().map(|cost| *cost as u64).collect(),
            max_frame_bytes: self
                .frame_costs
                .iter()
                .map(|cost| *cost as u64)
                .max()
                .unwrap_or(0),
        }
    }
}

struct Rewriter<'f> {
    facts: &'f ModuleFacts,
    plan: &'f IndexPlan,
    types_emitted: bool,
    imports_emitted: bool,
    wrapped_call_sites: u32,
}

impl Rewriter<'_> {
    fn inject_imports(&mut self, imports: &mut ImportSection) {
        if self.imports_emitted {
            return;
        }
        imports.import(
            VM_MODULE,
            TRAP_IMPORT,
            EntityType::Function(self.plan.helper_type),
        );
        let counter = GlobalType {
            val_type: COUNTER_TYPE,
            mutable: true,
            shared: false,
        };
        imports.import(VM_MODULE, STACK_BYTES_GLOBAL, EntityType::Global(counter));
        imports.import(VM_MODULE, STACK_DEPTH_GLOBAL, EntityType::Global(counter));
        self.imports_emitted = true;
    }

    fn frame_cost(&self, submitted: u32) -> Result<i32, Error<RewriteError>> {
        submitted
            .checked_sub(self.plan.imported)
            .and_then(|position| self.plan.frame_costs.get(position as usize).copied())
            .ok_or_else(|| {
                Error::UserError(RewriteError(format!(
                    "no frame cost for function {submitted}"
                )))
            })
    }

    fn signature(&self, submitted: u32) -> Result<&FuncSignature, Error<RewriteError>> {
        self.facts.function_signature(submitted).ok_or_else(|| {
            Error::UserError(RewriteError(format!(
                "no signature for function {submitted}"
            )))
        })
    }
}

impl Reencode for Rewriter<'_> {
    type Error = RewriteError;

    fn function_index(&mut self, func: u32) -> Result<u32, Error<Self::Error>> {
        Ok(self.plan.shifted(func))
    }

    fn global_index(&mut self, global: u32) -> Result<u32, Error<Self::Error>> {
        Ok(global + INJECTED_GLOBAL_IMPORTS)
    }

    fn instruction<'a>(
        &mut self,
        arg: Operator<'a>,
    ) -> Result<Instruction<'a>, Error<Self::Error>> {
        match arg {
            Operator::RefFunc { function_index } => Ok(Instruction::RefFunc(
                self.plan
                    .reference_target(function_index)
                    .map_err(Error::UserError)?,
            )),
            other => utils::instruction(self, other),
        }
    }

    fn intersperse_section_hook(
        &mut self,
        module: &mut Module,
        _after: Option<SectionId>,
        before: Option<SectionId>,
    ) -> Result<(), Error<Self::Error>> {
        if !self.types_emitted && before.is_none_or(|next| next > SectionId::Type) {
            // A submitted module without a type section (nothing admitted looks like this,
            // but the rewriter must stay correct on its own) still needs the helper type.
            let mut types = TypeSection::new();
            types.ty().function([ValType::I32], []);
            module.section(&types);
            self.types_emitted = true;
        }
        if !self.imports_emitted && before.is_none_or(|next| next > SectionId::Import) {
            let mut imports = ImportSection::new();
            self.inject_imports(&mut imports);
            module.section(&imports);
        }
        Ok(())
    }

    fn parse_type_section(
        &mut self,
        types: &mut TypeSection,
        section: wasmparser::TypeSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        utils::parse_type_section(self, types, section)?;
        if self.plan.helper_type_added {
            types.ty().function([ValType::I32], []);
        }
        self.types_emitted = true;
        Ok(())
    }

    fn parse_import_section(
        &mut self,
        imports: &mut ImportSection,
        section: wasmparser::ImportSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        self.inject_imports(imports);
        utils::parse_import_section(self, imports, section)
    }

    fn parse_function_section(
        &mut self,
        functions: &mut FunctionSection,
        section: wasmparser::FunctionSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        utils::parse_function_section(self, functions, section)?;
        for _ in 0..HELPER_FUNCTIONS {
            functions.function(self.plan.helper_type);
        }
        for target in &self.plan.thunk_targets {
            let position = (*target - self.plan.imported) as usize;
            let fact = self.facts.functions.get(position).ok_or_else(|| {
                Error::UserError(RewriteError(format!(
                    "thunk target {target} is not defined"
                )))
            })?;
            functions.function(fact.type_index);
        }
        Ok(())
    }

    fn parse_export(
        &mut self,
        exports: &mut ExportSection,
        export: wasmparser::Export<'_>,
    ) -> Result<(), Error<Self::Error>> {
        let (kind, index) = match export.kind {
            ExternalKind::Func => (
                ExportKind::Func,
                self.plan
                    .reference_target(export.index)
                    .map_err(Error::UserError)?,
            ),
            ExternalKind::Memory => (ExportKind::Memory, export.index),
            ExternalKind::Global => (ExportKind::Global, self.global_index(export.index)?),
            ExternalKind::Table | ExternalKind::Tag => {
                return Err(Error::UserError(RewriteError(format!(
                    "export `{}` of a kind admission refuses",
                    export.name
                ))));
            }
        };
        exports.export(export.name, kind, index);
        Ok(())
    }

    fn element_items<'a>(
        &mut self,
        items: ElementItems<'a>,
    ) -> Result<Elements<'a>, Error<Self::Error>> {
        match items {
            ElementItems::Functions(functions) => {
                let mut targets = Vec::new();
                for function in functions {
                    targets.push(
                        self.plan
                            .reference_target(function?)
                            .map_err(Error::UserError)?,
                    );
                }
                Ok(Elements::Functions(targets.into()))
            }
            expressions @ ElementItems::Expressions(..) => utils::element_items(self, expressions),
        }
    }

    fn parse_custom_section(
        &mut self,
        _module: &mut Module,
        _section: wasmparser::CustomSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        // Custom sections carry no semantics and are stripped so the prepared bytes are
        // canonical: names, producers and debug data never reach the engine.
        Ok(())
    }

    fn parse_function_body(
        &mut self,
        code: &mut CodeSection,
        func: wasmparser::FunctionBody<'_>,
    ) -> Result<(), Error<Self::Error>> {
        let mut function = utils::new_function_with_parsed_locals(self, &func)?;
        let mut reader = func.get_operators_reader()?;
        while !reader.eof() {
            let op = reader.read()?;
            if let Operator::Call { function_index } = op {
                if function_index >= self.plan.imported {
                    let cost = self.frame_cost(function_index)?;
                    function.instruction(&Instruction::I32Const(cost));
                    function.instruction(&Instruction::Call(self.plan.enter));
                    function.instruction(&Instruction::Call(self.plan.shifted(function_index)));
                    function.instruction(&Instruction::I32Const(cost));
                    function.instruction(&Instruction::Call(self.plan.leave));
                    self.wrapped_call_sites += 1;
                    continue;
                }
            }
            function.instruction(&self.instruction(op)?);
        }
        code.function(&function);
        Ok(())
    }

    fn parse_code_section(
        &mut self,
        code: &mut CodeSection,
        section: wasmparser::CodeSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        utils::parse_code_section(self, code, section)?;
        code.function(&enter_body());
        code.function(&leave_body());
        for target in &self.plan.thunk_targets {
            let signature = self.signature(*target)?.clone();
            let cost = self.frame_cost(*target)?;
            code.function(&thunk_body(
                &signature,
                cost,
                self.plan.shifted(*target),
                self.plan.enter,
                self.plan.leave,
            ));
        }
        Ok(())
    }

    fn start_section(&mut self, _start: u32) -> Result<u32, Error<Self::Error>> {
        Err(Error::UserError(RewriteError(
            "start section reached the rewriter".to_owned(),
        )))
    }
}

/// Rewrites validated canonical bytes into prepared bytes.
pub(crate) fn instrument(
    canonical_bytes: &[u8],
    facts: &ModuleFacts,
    plan: &IndexPlan,
) -> Result<(Vec<u8>, InstrumentationReport), ModuleError> {
    let mut rewriter = Rewriter {
        facts,
        plan,
        types_emitted: false,
        imports_emitted: false,
        wrapped_call_sites: 0,
    };
    let mut module = Module::new();
    let mut parser = Parser::new(0);
    parser.set_features(admitted_features());
    utils::parse_core_module(&mut rewriter, &mut module, parser, canonical_bytes).map_err(
        |error| match error {
            Error::UserError(error) => ModuleError::Internal(error.to_string()),
            other => ModuleError::Internal(format!("re-encoding validated bytes failed: {other}")),
        },
    )?;
    if !rewriter.imports_emitted {
        return Err(ModuleError::Internal(
            "the injected imports were never emitted".to_owned(),
        ));
    }
    let report = plan.report(rewriter.wrapped_call_sites);
    Ok((module.finish(), report))
}
