//! One pass over a module: full validation under the admitted feature set, typed feature
//! classification, structural measurement against the profile's caps, and the facts every
//! later stage (import and export rules, instrumentation, provenance checks) reads.
//!
//! The pass runs the pinned `wasmparser` validator payload by payload. Before each payload
//! reaches the validator it is classified here, so a module outside the admitted set fails with
//! the proposal it needs rather than the validator's message; anything the classification does
//! not reach first is still refused by the validator, whose message is mapped back to a
//! proposal where it names one. Cheap checks run first: the byte cap before any decoding, and
//! every section in binary order so a module is refused at the first violation.

use crate::bundle::{
    FuncSignature, InitializationMeasurements, MemoryShape, StructuralMeasurements, TableShape,
    ValueType,
};
use crate::errors::{ForbiddenFeature, MemoryRule, ModuleError, StructuralCap, TableRule};
use crate::profile::PreparationProfile;
use crate::wasm_features::{
    admitted_features, classify_const_operator, classify_global_type, classify_memory_type,
    classify_operator, classify_ref_type, classify_table_type, classify_val_type,
    classify_validator_message,
};
use platform_version::version::dashvm_versions::DashVmLimits;
use std::collections::BTreeSet;
use wasmparser::{
    BinaryReaderError, CompositeInnerType, ConstExpr, ElementItems, ElementKind, Encoding,
    ExternalKind, FuncValidatorAllocations, Operator, Parser, Payload, TableInit, TypeRef,
    ValidPayload, Validator,
};

/// Which byte cap and which structural rules the pass enforces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Submitted canonical bytes: the canonical byte cap and every structural cap apply.
    Submitted,
    /// Instrumented bytes: the prepared byte cap applies; structural caps were enforced on the
    /// canonical bytes and instrumentation grows the module by design, so they are measured
    /// but not re-applied. Provenance is checked by the caller against the submitted facts.
    Prepared,
}

/// The kind and target of an import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ImportKind {
    /// A function import with the given type index.
    Function { type_index: u32 },
    /// A mutable or immutable global import.
    Global { value: ValueType, mutable: bool },
}

/// One import as the module declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImportFact {
    /// Import module.
    pub module: String,
    /// Import name.
    pub name: String,
    /// Kind.
    pub kind: ImportKind,
}

/// One export as the module declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExportFact {
    /// Export name.
    pub name: String,
    /// Kind.
    pub kind: ExternalKind,
    /// Index in the kind's index space.
    pub index: u32,
}

/// What the pass learned about one defined function body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FunctionFact {
    /// Type index in the type section.
    pub type_index: u32,
    /// Declared locals in declaration order, expanded.
    pub locals: Vec<ValueType>,
    /// Most operand-stack slots the body reaches.
    pub max_operand_slots: u32,
    /// Decoded operators.
    pub operators: u32,
    /// Direct `call` operators whose target is a defined function.
    pub calls_to_defined: u32,
}

/// Every fact the pass records about a module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleFacts {
    /// Function types in type-section order.
    pub types: Vec<FuncSignature>,
    /// Imports in import-section order.
    pub imports: Vec<ImportFact>,
    /// Defined functions in function-section order.
    pub functions: Vec<FunctionFact>,
    /// Exports in export-section order.
    pub exports: Vec<ExportFact>,
    /// The single memory, if defined.
    pub memory: Option<MemoryShape>,
    /// The single table, if defined.
    pub table: Option<TableShape>,
    /// Function indices referenced by `ref.func` anywhere or listed in an element segment.
    pub referenced_functions: BTreeSet<u32>,
    /// Whether a start section is present.
    pub has_start: bool,
    /// Whether any custom section is present.
    pub has_custom_sections: bool,
    /// Element segments.
    pub element_segments: u32,
    /// Data segments.
    pub data_segments: u32,
    /// Structural measurements.
    pub structure: StructuralMeasurements,
    /// Initialisation measurements.
    pub initialization: InitializationMeasurements,
}

impl ModuleFacts {
    /// Imported functions, which occupy the first function indices.
    pub fn imported_functions(&self) -> u32 {
        self.imports
            .iter()
            .filter(|import| matches!(import.kind, ImportKind::Function { .. }))
            .count() as u32
    }

    /// The signature of the function at `index` in the function index space.
    pub fn function_signature(&self, index: u32) -> Option<&FuncSignature> {
        let imported = self.imported_functions();
        if index < imported {
            let type_index = self
                .imports
                .iter()
                .filter_map(|import| match import.kind {
                    ImportKind::Function { type_index } => Some(type_index),
                    ImportKind::Global { .. } => None,
                })
                .nth(index as usize)?;
            return self.types.get(type_index as usize);
        }
        let function = self.functions.get((index - imported) as usize)?;
        self.types.get(function.type_index as usize)
    }
}

fn invalid(error: BinaryReaderError) -> ModuleError {
    match classify_validator_message(error.message()) {
        Some(feature) => ModuleError::ForbiddenFeature {
            feature,
            offset: error.offset(),
        },
        None => ModuleError::Invalid {
            offset: error.offset(),
            message: error.message().to_owned(),
        },
    }
}

fn forbidden(feature: ForbiddenFeature, offset: usize) -> ModuleError {
    ModuleError::ForbiddenFeature { feature, offset }
}

fn cap(cap: StructuralCap, actual: u64, max: u64) -> Result<(), ModuleError> {
    if actual > max {
        Err(ModuleError::CapExceeded { cap, actual, max })
    } else {
        Ok(())
    }
}

/// Runs the pass.
pub(crate) fn measure(
    bytes: &[u8],
    profile: &PreparationProfile,
    stage: Stage,
) -> Result<ModuleFacts, ModuleError> {
    let limits = &profile.limits;
    let byte_cap = match stage {
        Stage::Submitted => limits.max_canonical_module_bytes,
        Stage::Prepared => limits.max_prepared_module_bytes,
    };
    if bytes.len() as u64 > u64::from(byte_cap) {
        return Err(match stage {
            Stage::Submitted => ModuleError::TooLarge {
                actual: bytes.len() as u64,
                max: byte_cap,
            },
            Stage::Prepared => ModuleError::PreparedTooLarge {
                actual: bytes.len() as u64,
                max: byte_cap,
            },
        });
    }

    let features = admitted_features();
    let mut parser = Parser::new(0);
    parser.set_features(features);
    let mut validator = Validator::new_with_features(features);
    let mut pass = Pass {
        limits,
        stage,
        facts: ModuleFacts {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            exports: Vec::new(),
            memory: None,
            table: None,
            referenced_functions: BTreeSet::new(),
            has_start: false,
            has_custom_sections: false,
            element_segments: 0,
            data_segments: 0,
            structure: StructuralMeasurements {
                canonical_bytes: bytes.len() as u64,
                prepared_bytes: 0,
                imported_functions: 0,
                defined_functions: 0,
                types: 0,
                exports: 0,
                operators: 0,
                max_operators_per_function: 0,
                max_basic_blocks_per_function: 0,
                max_nesting_depth: 0,
                max_params: 0,
                max_locals: 0,
                max_operand_slots: 0,
            },
            initialization: InitializationMeasurements {
                initial_memory_pages: 0,
                active_data_bytes: 0,
                passive_data_bytes: 0,
                initial_table_elements: 0,
                active_element_items: 0,
            },
        },
        memories: 0,
        tables: 0,
        data_bytes: 0,
        allocations: FuncValidatorAllocations::default(),
    };

    for payload in parser.parse_all(bytes) {
        let payload = payload.map_err(invalid)?;
        pass.classify(&payload)?;
        match validator.payload(&payload).map_err(invalid)? {
            ValidPayload::Ok | ValidPayload::End(_) => {}
            ValidPayload::Parser(_) => {
                // Only reachable with the component model, which is not compiled in.
                return Err(forbidden(ForbiddenFeature::ComponentModel, 0));
            }
            ValidPayload::Func(function, body) => pass.function_body(function, body)?,
        }
    }
    Ok(pass.facts)
}

struct Pass<'p> {
    limits: &'p DashVmLimits,
    stage: Stage,
    facts: ModuleFacts,
    memories: u32,
    tables: u32,
    data_bytes: u64,
    allocations: FuncValidatorAllocations,
}

impl Pass<'_> {
    fn enforce_caps(&self) -> bool {
        self.stage == Stage::Submitted
    }

    fn classify(&mut self, payload: &Payload<'_>) -> Result<(), ModuleError> {
        match payload {
            Payload::Version {
                encoding: Encoding::Component,
                range,
                ..
            } => Err(forbidden(ForbiddenFeature::ComponentModel, range.start)),
            Payload::Version { .. } => Ok(()),
            Payload::TypeSection(section) => self.type_section(section.clone()),
            Payload::ImportSection(section) => self.import_section(section.clone()),
            Payload::FunctionSection(section) => self.function_section(section.clone()),
            Payload::TableSection(section) => self.table_section(section.clone()),
            Payload::MemorySection(section) => self.memory_section(section.clone()),
            Payload::TagSection(section) => Err(forbidden(
                ForbiddenFeature::Exceptions,
                section.range().start,
            )),
            Payload::GlobalSection(section) => self.global_section(section.clone()),
            Payload::ExportSection(section) => self.export_section(section.clone()),
            Payload::StartSection { .. } => {
                self.facts.has_start = true;
                Err(ModuleError::StartSection)
            }
            Payload::ElementSection(section) => self.element_section(section.clone()),
            Payload::DataCountSection { .. } => Ok(()),
            Payload::DataSection(section) => self.data_section(section.clone()),
            Payload::CodeSectionStart { .. } | Payload::CodeSectionEntry(_) => Ok(()),
            Payload::CustomSection(_) => {
                self.facts.has_custom_sections = true;
                Ok(())
            }
            // The validator refuses unknown sections; nothing to classify.
            _ => Ok(()),
        }
    }

    fn type_section(
        &mut self,
        section: wasmparser::TypeSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for group in section.into_iter_with_offsets() {
            let (offset, group) = group.map_err(invalid)?;
            if group.is_explicit_rec_group() || group.types().len() != 1 {
                return Err(forbidden(ForbiddenFeature::Gc, offset));
            }
            let sub_type = group
                .types()
                .next()
                .ok_or_else(|| ModuleError::Internal("empty rec group".to_owned()))?;
            if !sub_type.is_final || sub_type.supertype_idx.is_some() {
                return Err(forbidden(ForbiddenFeature::Gc, offset));
            }
            if sub_type.composite_type.shared {
                return Err(forbidden(ForbiddenFeature::SharedEverythingThreads, offset));
            }
            let func_type = match &sub_type.composite_type.inner {
                CompositeInnerType::Func(func_type) => func_type,
                CompositeInnerType::Array(_) | CompositeInnerType::Struct(_) => {
                    return Err(forbidden(ForbiddenFeature::Gc, offset));
                }
                CompositeInnerType::Cont(_) => {
                    return Err(forbidden(ForbiddenFeature::StackSwitching, offset));
                }
            };
            let params = func_type
                .params()
                .iter()
                .map(|ty| classify_val_type(*ty).map_err(|feature| forbidden(feature, offset)))
                .collect::<Result<Vec<_>, _>>()?;
            let results = func_type
                .results()
                .iter()
                .map(|ty| classify_val_type(*ty).map_err(|feature| forbidden(feature, offset)))
                .collect::<Result<Vec<_>, _>>()?;
            if self.enforce_caps() {
                cap(
                    StructuralCap::Params,
                    params.len() as u64,
                    u64::from(self.limits.max_params_per_function),
                )?;
                cap(
                    StructuralCap::Results,
                    results.len() as u64,
                    u64::from(self.limits.max_params_per_function),
                )?;
            }
            self.facts.structure.max_params =
                self.facts.structure.max_params.max(params.len() as u32);
            self.facts.types.push(FuncSignature::new(params, results));
            self.facts.structure.types = self.facts.types.len() as u32;
            if self.enforce_caps() {
                cap(
                    StructuralCap::Types,
                    u64::from(self.facts.structure.types),
                    u64::from(self.limits.max_types_per_module),
                )?;
            }
        }
        Ok(())
    }

    fn import_section(
        &mut self,
        section: wasmparser::ImportSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for import in section.into_iter_with_offsets() {
            let (offset, import) = import.map_err(invalid)?;
            let kind = match import.ty {
                TypeRef::Func(type_index) => {
                    if self.facts.types.get(type_index as usize).is_none() {
                        return Err(ModuleError::Invalid {
                            offset,
                            message: format!("unknown type {type_index}: type index out of bounds"),
                        });
                    }
                    ImportKind::Function { type_index }
                }
                TypeRef::Table(_) => return Err(ModuleError::Table(TableRule::Imported)),
                TypeRef::Memory(_) => return Err(ModuleError::Memory(MemoryRule::Imported)),
                TypeRef::Global(global) => {
                    let value = classify_global_type(&global)
                        .map_err(|feature| forbidden(feature, offset))?;
                    ImportKind::Global {
                        value,
                        mutable: global.mutable,
                    }
                }
                TypeRef::Tag(_) => return Err(forbidden(ForbiddenFeature::Exceptions, offset)),
            };
            self.facts.imports.push(ImportFact {
                module: import.module.to_owned(),
                name: import.name.to_owned(),
                kind,
            });
        }
        self.facts.structure.imported_functions = self.facts.imported_functions();
        if self.enforce_caps() {
            cap(
                StructuralCap::Functions,
                u64::from(self.facts.structure.imported_functions),
                u64::from(self.limits.max_functions_per_module),
            )?;
        }
        Ok(())
    }

    fn function_section(
        &mut self,
        section: wasmparser::FunctionSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for type_index in section.into_iter_with_offsets() {
            let (offset, type_index) = type_index.map_err(invalid)?;
            if self.facts.types.get(type_index as usize).is_none() {
                return Err(ModuleError::Invalid {
                    offset,
                    message: format!("unknown type {type_index}: type index out of bounds"),
                });
            }
            self.facts.functions.push(FunctionFact {
                type_index,
                locals: Vec::new(),
                max_operand_slots: 0,
                operators: 0,
                calls_to_defined: 0,
            });
        }
        self.facts.structure.defined_functions = self.facts.functions.len() as u32;
        if self.enforce_caps() {
            let total = u64::from(self.facts.structure.imported_functions)
                + u64::from(self.facts.structure.defined_functions);
            cap(
                StructuralCap::Functions,
                total,
                u64::from(self.limits.max_functions_per_module),
            )?;
        }
        Ok(())
    }

    fn table_section(
        &mut self,
        section: wasmparser::TableSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for table in section.into_iter_with_offsets() {
            let (offset, table) = table.map_err(invalid)?;
            self.tables += 1;
            if self.tables > 1 {
                return Err(ModuleError::Table(TableRule::Multiple));
            }
            classify_table_type(&table.ty).map_err(|feature| forbidden(feature, offset))?;
            if let TableInit::Expr(_) = table.init {
                return Err(forbidden(ForbiddenFeature::FunctionReferences, offset));
            }
            let Some(maximum) = table.ty.maximum else {
                return Err(ModuleError::Table(TableRule::MaximumMissing));
            };
            if self.enforce_caps() {
                cap(
                    StructuralCap::TableElements,
                    table.ty.initial,
                    u64::from(self.limits.max_table_elements),
                )?;
                cap(
                    StructuralCap::TableElements,
                    maximum,
                    u64::from(self.limits.max_table_elements),
                )?;
            }
            // Both fit in u32 after the cap check on the submitted stage; on the prepared stage
            // they equal the submitted values. A 32-bit table's limits are u32 by encoding.
            self.facts.table = Some(TableShape {
                initial_elements: table.ty.initial as u32,
                maximum_elements: maximum as u32,
            });
            self.facts.initialization.initial_table_elements = table.ty.initial as u32;
        }
        Ok(())
    }

    fn memory_section(
        &mut self,
        section: wasmparser::MemorySectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for memory in section.into_iter_with_offsets() {
            let (offset, memory) = memory.map_err(invalid)?;
            self.memories += 1;
            if self.memories > 1 {
                return Err(forbidden(ForbiddenFeature::MultiMemory, offset));
            }
            classify_memory_type(&memory).map_err(|feature| forbidden(feature, offset))?;
            if self.enforce_caps() {
                cap(
                    StructuralCap::InitialMemoryPages,
                    memory.initial,
                    u64::from(self.limits.max_initial_memory_pages),
                )?;
                if let Some(maximum) = memory.maximum {
                    cap(
                        StructuralCap::MemoryMaximumPages,
                        maximum,
                        u64::from(self.limits.max_memory_pages_per_instance),
                    )?;
                }
            }
            // A 32-bit memory's limits are u32 by encoding.
            self.facts.memory = Some(MemoryShape {
                initial_pages: memory.initial as u32,
                maximum_pages: memory.maximum.map(|maximum| maximum as u32),
            });
            self.facts.initialization.initial_memory_pages = memory.initial as u32;
        }
        Ok(())
    }

    fn const_expr(&mut self, expr: &ConstExpr<'_>, offset: usize) -> Result<(), ModuleError> {
        let mut reader = expr.get_operators_reader();
        while !reader.is_end_then_eof() {
            let op = reader.read().map_err(invalid)?;
            if let Some(feature) = classify_const_operator(&op) {
                return Err(forbidden(feature, offset));
            }
            if let Operator::RefFunc { function_index } = op {
                self.facts.referenced_functions.insert(function_index);
            }
        }
        Ok(())
    }

    fn global_section(
        &mut self,
        section: wasmparser::GlobalSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for global in section.into_iter_with_offsets() {
            let (offset, global) = global.map_err(invalid)?;
            classify_global_type(&global.ty).map_err(|feature| forbidden(feature, offset))?;
            self.const_expr(&global.init_expr, offset)?;
        }
        Ok(())
    }

    fn export_section(
        &mut self,
        section: wasmparser::ExportSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for export in section.into_iter_with_offsets() {
            let (offset, export) = export.map_err(invalid)?;
            if export.kind == ExternalKind::Tag {
                return Err(forbidden(ForbiddenFeature::Exceptions, offset));
            }
            self.facts.exports.push(ExportFact {
                name: export.name.to_owned(),
                kind: export.kind,
                index: export.index,
            });
            self.facts.structure.exports = self.facts.exports.len() as u32;
            if self.enforce_caps() {
                cap(
                    StructuralCap::Exports,
                    u64::from(self.facts.structure.exports),
                    u64::from(self.limits.max_exports_per_module),
                )?;
            }
        }
        Ok(())
    }

    fn element_section(
        &mut self,
        section: wasmparser::ElementSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for element in section.into_iter_with_offsets() {
            let (offset, element) = element.map_err(invalid)?;
            self.facts.element_segments += 1;
            let active = match &element.kind {
                ElementKind::Active { offset_expr, .. } => {
                    self.const_expr(offset_expr, offset)?;
                    true
                }
                ElementKind::Passive | ElementKind::Declared => false,
            };
            let items = match element.items {
                ElementItems::Functions(functions) => {
                    let mut count = 0u64;
                    for function in functions {
                        let function = function.map_err(invalid)?;
                        self.facts.referenced_functions.insert(function);
                        count += 1;
                    }
                    count
                }
                ElementItems::Expressions(ref_type, expressions) => {
                    classify_ref_type(ref_type).map_err(|feature| forbidden(feature, offset))?;
                    let mut count = 0u64;
                    for expression in expressions {
                        let expression = expression.map_err(invalid)?;
                        self.const_expr(&expression, offset)?;
                        count += 1;
                    }
                    count
                }
            };
            if active {
                self.facts.initialization.active_element_items += items;
            }
        }
        Ok(())
    }

    fn data_section(
        &mut self,
        section: wasmparser::DataSectionReader<'_>,
    ) -> Result<(), ModuleError> {
        for datum in section.into_iter_with_offsets() {
            let (offset, datum) = datum.map_err(invalid)?;
            self.facts.data_segments += 1;
            let bytes = datum.data.len() as u64;
            match &datum.kind {
                wasmparser::DataKind::Active {
                    memory_index,
                    offset_expr,
                } => {
                    if *memory_index != 0 {
                        return Err(forbidden(ForbiddenFeature::MultiMemory, offset));
                    }
                    self.const_expr(offset_expr, offset)?;
                    self.facts.initialization.active_data_bytes += bytes;
                }
                wasmparser::DataKind::Passive => {
                    self.facts.initialization.passive_data_bytes += bytes;
                }
            }
            self.data_bytes += bytes;
            if self.enforce_caps() {
                cap(
                    StructuralCap::DataSegmentBytes,
                    self.data_bytes,
                    u64::from(self.limits.max_data_segment_bytes_per_module),
                )?;
            }
        }
        Ok(())
    }

    fn function_body(
        &mut self,
        function: wasmparser::FuncToValidate<wasmparser::ValidatorResources>,
        body: wasmparser::FunctionBody<'_>,
    ) -> Result<(), ModuleError> {
        let index = function.index;
        let imported = self.facts.structure.imported_functions;
        let defined_index = index.checked_sub(imported).ok_or_else(|| {
            ModuleError::Internal(format!("code entry {index} precedes the defined functions"))
        })? as usize;
        if defined_index >= self.facts.functions.len() {
            return Err(ModuleError::Invalid {
                offset: body.range().start,
                message: "function and code section have inconsistent lengths".to_owned(),
            });
        }
        let allocations = std::mem::take(&mut self.allocations);
        let mut validator = function.into_validator(allocations);

        let mut locals = Vec::new();
        let mut locals_reader = body.get_locals_reader().map_err(invalid)?;
        for _ in 0..locals_reader.get_count() {
            let offset = locals_reader.original_position();
            let (count, ty) = locals_reader.read().map_err(invalid)?;
            let value = classify_val_type(ty).map_err(|feature| forbidden(feature, offset))?;
            let total = locals.len() as u64 + u64::from(count);
            if self.enforce_caps() {
                cap(
                    StructuralCap::Locals,
                    total,
                    u64::from(self.limits.max_locals_per_function),
                )?;
            }
            validator
                .define_locals(offset, count, ty)
                .map_err(invalid)?;
            locals.extend(std::iter::repeat_n(value, count as usize));
        }

        let mut reader = body.get_operators_reader().map_err(invalid)?;
        let mut operators = 0u64;
        let mut basic_blocks = 1u64;
        let mut max_nesting = 0u32;
        let mut max_operand_slots = 0u32;
        let mut calls_to_defined = 0u32;
        while !reader.eof() {
            let (op, offset) = reader.read_with_offset().map_err(invalid)?;
            if let Some(feature) = classify_operator(&op) {
                return Err(forbidden(feature, offset));
            }
            operators += 1;
            if self.enforce_caps() {
                cap(
                    StructuralCap::OperatorsPerFunction,
                    operators,
                    u64::from(self.limits.max_operators_per_function),
                )?;
                cap(
                    StructuralCap::OperatorsPerModule,
                    self.facts.structure.operators + operators,
                    u64::from(self.limits.max_operators_per_module),
                )?;
            }
            // The validator's control stack holds the implicit function frame, so the height
            // before an operator is 1 at the top level; the function's closing `end` is the one
            // `end` seen at that height and it begins no block.
            let frames_before = validator.control_stack_height();
            let begins_block = match &op {
                Operator::Block { .. }
                | Operator::Loop { .. }
                | Operator::If { .. }
                | Operator::Else
                | Operator::Br { .. }
                | Operator::BrIf { .. }
                | Operator::BrTable { .. }
                | Operator::Return => true,
                Operator::End => frames_before > 1,
                _ => false,
            };
            if begins_block {
                basic_blocks += 1;
                if self.enforce_caps() {
                    cap(
                        StructuralCap::BasicBlocksPerFunction,
                        basic_blocks,
                        u64::from(self.limits.max_basic_blocks_per_function),
                    )?;
                }
            }
            match &op {
                Operator::Call { function_index } => {
                    if *function_index >= imported {
                        calls_to_defined += 1;
                    }
                }
                Operator::RefFunc { function_index } => {
                    self.facts.referenced_functions.insert(*function_index);
                }
                _ => {}
            }
            validator.op(offset, &op).map_err(invalid)?;
            // Nesting counts the frames inside the function frame.
            let nesting = validator.control_stack_height().saturating_sub(1);
            if nesting > max_nesting {
                max_nesting = nesting;
                if self.enforce_caps() {
                    cap(
                        StructuralCap::NestingDepth,
                        u64::from(nesting),
                        u64::from(self.limits.max_nesting_depth),
                    )?;
                }
            }
            max_operand_slots = max_operand_slots.max(validator.operand_stack_height());
        }
        reader.finish().map_err(invalid)?;
        self.allocations = validator.into_allocations();

        let structure = &mut self.facts.structure;
        structure.operators += operators;
        structure.max_operators_per_function =
            structure.max_operators_per_function.max(operators as u32);
        structure.max_basic_blocks_per_function = structure
            .max_basic_blocks_per_function
            .max(basic_blocks as u32);
        structure.max_nesting_depth = structure.max_nesting_depth.max(max_nesting);
        structure.max_locals = structure.max_locals.max(locals.len() as u32);
        structure.max_operand_slots = structure.max_operand_slots.max(max_operand_slots);
        let fact = &mut self.facts.functions[defined_index];
        fact.locals = locals;
        fact.max_operand_slots = max_operand_slots;
        fact.operators = operators as u32;
        fact.calls_to_defined = calls_to_defined;
        Ok(())
    }
}
