//! Every way preparation can refuse a module or a bundle.
//!
//! Rejections are deterministic consensus outcomes: two nodes preparing the same bytes under the
//! same protocol version reach the same variant. The one exception is [`ModuleError::Internal`],
//! which means the instrumenter contradicted its own report; it is a node fault (the build is
//! wrong), never a paid rejection, and callers must treat it as such.

use crate::bundle::{FuncSignature, ModuleName};
use platform_version::version::FeatureVersion;
use platform_version::version::ProtocolVersion;
use thiserror::Error;

/// A preparation profile could not be built from a protocol version.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ProfileError {
    /// The protocol version carries no DashVM table: smart contracts do not exist at it.
    #[error("protocol version {protocol_version} has no DashVM table")]
    NotActive {
        /// The protocol version that was asked for.
        protocol_version: ProtocolVersion,
    },
    /// The table names a preparation generation this crate does not implement.
    #[error("unknown preparation generation {generation}")]
    UnknownGeneration {
        /// The generation the table selects.
        generation: FeatureVersion,
    },
}

/// A WebAssembly proposal, type or encoding outside the admitted feature set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForbiddenFeature {
    /// The binary is a component, not a core module.
    ComponentModel,
    /// Fixed-width SIMD (`v128` types or `0xfd` operators).
    Simd,
    /// Relaxed SIMD.
    RelaxedSimd,
    /// Threads: shared memories and atomic operators.
    Threads,
    /// Shared-everything threads: shared types, globals or tables.
    SharedEverythingThreads,
    /// 64-bit memories or tables.
    Memory64,
    /// More than one linear memory.
    MultiMemory,
    /// Tail calls (`return_call`, `return_call_indirect`).
    TailCall,
    /// Arithmetic in constant expressions.
    ExtendedConst,
    /// Typed function references: concrete or non-nullable reference types, `call_ref`, table
    /// initialisers.
    FunctionReferences,
    /// Garbage-collected types: struct, array, recursion groups, subtyping, `anyref` and the
    /// other abstract heap types.
    Gc,
    /// `externref`: a host reference the runtime does not expose.
    ExternRef,
    /// Exception handling: tags, `throw`, `try_table`.
    Exceptions,
    /// The legacy exception handling encoding.
    LegacyExceptions,
    /// Stack switching: continuations.
    StackSwitching,
    /// 128-bit wide arithmetic.
    WideArithmetic,
    /// Memory control (`memory.discard`).
    MemoryControl,
    /// Custom page sizes.
    CustomPageSizes,
    /// An operator this crate cannot classify (a newer proposal than the pinned parser knows
    /// by name).
    UnknownOperator,
}

/// The structural cap a module exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StructuralCap {
    /// `max_functions_per_module`, imports and definitions together.
    Functions,
    /// `max_types_per_module`.
    Types,
    /// `max_params_per_function`.
    Params,
    /// `max_params_per_function`, applied to results as well.
    Results,
    /// `max_locals_per_function`.
    Locals,
    /// `max_exports_per_module`.
    Exports,
    /// `max_operators_per_module`.
    OperatorsPerModule,
    /// `max_operators_per_function`.
    OperatorsPerFunction,
    /// `max_basic_blocks_per_function`.
    BasicBlocksPerFunction,
    /// `max_nesting_depth`.
    NestingDepth,
    /// `max_initial_memory_pages`.
    InitialMemoryPages,
    /// `max_memory_pages_per_instance`, applied to a declared memory maximum.
    MemoryMaximumPages,
    /// `max_table_elements`, applied to the table's initial size and maximum.
    TableElements,
    /// `max_data_segment_bytes_per_module`.
    DataSegmentBytes,
}

/// A rule on the module's linear memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryRule {
    /// The module defines no memory; the host envelope needs one.
    Missing,
    /// The module imports its memory; memories are always instance-private.
    Imported,
    /// The memory is not exported under the `memory` name.
    NotExported,
}

/// A rule on the module's function table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableRule {
    /// The module imports a table; tables are always instance-private.
    Imported,
    /// The module defines more than one table.
    Multiple,
    /// The table's element type is not nullable `funcref`.
    NotFuncRef,
    /// The table declares no maximum, so `table.grow` would be unbounded.
    MaximumMissing,
}

/// Why an import was refused.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ImportRejection {
    /// The import module is neither the host envelope nor a bundle-internal binding.
    #[error("unknown import module")]
    UnknownModule,
    /// The import module is reserved for the preparation generation's own instrumentation.
    #[error("the dash_vm module is reserved for instrumentation")]
    ReservedModule,
    /// Only functions may be imported.
    #[error("only functions may be imported")]
    NotAFunction,
    /// The host envelope has no function of that name.
    #[error("the host envelope has no such function")]
    UnknownHostFunction,
    /// The host function is imported with the wrong signature.
    #[error("host function signature {actual} does not match {expected}")]
    HostSignature {
        /// The signature the envelope defines.
        expected: Box<FuncSignature>,
        /// The signature the module imports.
        actual: Box<FuncSignature>,
    },
    /// The target module name of an internal binding is not a valid module name.
    #[error("invalid binding target name: {0}")]
    InvalidTargetName(Box<BundleError>),
}

/// Why an export was refused.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ExportRejection {
    /// Tables are instance-private and may not be exported.
    #[error("export `{name}` is a table")]
    Table {
        /// The export name.
        name: String,
    },
    /// An export names an imported function; exports must name defined functions.
    #[error("export `{name}` re-exports an imported function")]
    OfImportedFunction {
        /// The export name.
        name: String,
    },
    /// A second memory export, or a memory exported under a name other than `memory`.
    #[error("memory must be exported exactly once under the name `memory`, found `{name}`")]
    MemoryName {
        /// The export name.
        name: String,
    },
    /// The `memory` export is not a memory.
    #[error("export `memory` is not a memory")]
    MemoryNotMemory,
    /// The `dash_alloc` export is missing.
    #[error("missing the `dash_alloc` export")]
    MissingAlloc,
    /// The `dash_alloc` export is not a function or has the wrong signature.
    #[error("`dash_alloc` must be a function of signature {expected}")]
    AllocSignature {
        /// The signature the envelope defines.
        expected: FuncSignature,
    },
    /// No exported function has the entry signature.
    #[error("no exported function has the entry signature {expected}")]
    NoEntry {
        /// The entry signature.
        expected: FuncSignature,
    },
}

/// A module was refused, or its preparation failed.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ModuleError {
    /// The canonical bytes exceed the per-module cap. Checked before any decoding.
    #[error("canonical module of {actual} bytes exceeds the cap of {max} bytes")]
    TooLarge {
        /// The submitted length.
        actual: u64,
        /// `max_canonical_module_bytes`.
        max: u32,
    },
    /// The prepared bytes exceed the instrumented-size cap.
    #[error("prepared module of {actual} bytes exceeds the cap of {max} bytes")]
    PreparedTooLarge {
        /// The prepared length.
        actual: u64,
        /// `max_prepared_module_bytes`.
        max: u32,
    },
    /// The pinned validator rejected the module under the admitted feature set.
    #[error("invalid module at offset {offset}: {message}")]
    Invalid {
        /// Byte offset of the error.
        offset: usize,
        /// The validator's message.
        message: String,
    },
    /// The module uses a feature outside the admitted set.
    #[error("forbidden feature {feature:?} at offset {offset}")]
    ForbiddenFeature {
        /// The feature.
        feature: ForbiddenFeature,
        /// Byte offset of the first use.
        offset: usize,
    },
    /// The module exceeds a structural cap.
    #[error("{cap:?} of {actual} exceeds the cap of {max}")]
    CapExceeded {
        /// The cap.
        cap: StructuralCap,
        /// The measured value.
        actual: u64,
        /// The cap's value in the profile.
        max: u64,
    },
    /// The memory does not have the required shape.
    #[error("memory rule violated: {0:?}")]
    Memory(MemoryRule),
    /// The table does not have the required shape.
    #[error("table rule violated: {0:?}")]
    Table(TableRule),
    /// The module has a start section. Generation 0 rejects starts outright (provisional):
    /// initialisation must be memory, table and global setup only.
    #[error("start sections are not admitted")]
    StartSection,
    /// An import was refused.
    #[error("import `{module}`.`{name}` refused: {reason}")]
    Import {
        /// The import module.
        module: String,
        /// The import name.
        name: String,
        /// Why.
        reason: ImportRejection,
    },
    /// An export was refused.
    #[error("export refused: {0}")]
    Export(ExportRejection),
    /// A function's frame is larger than the whole logical stack, so it could never be called.
    #[error("function {function} needs {frame_bytes} bytes of logical stack, more than the {max} byte budget")]
    FrameExceedsLogicalStack {
        /// The function index.
        function: u32,
        /// The frame cost.
        frame_bytes: u64,
        /// `max_logical_stack_bytes`.
        max: u32,
    },
    /// The instrumenter produced output that contradicts its own report. A node fault, never a
    /// paid rejection.
    #[error("internal preparation error: {0}")]
    Internal(String),
}

/// A bundle-level refusal.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BundleError {
    /// A module name is empty, too long or has a character outside `[a-z0-9_]`.
    #[error("invalid module name `{name}`: {reason}")]
    InvalidModuleName {
        /// The offending name.
        name: String,
        /// Why.
        reason: &'static str,
    },
    /// Two modules share a name.
    #[error("duplicate module name `{name}`")]
    DuplicateModuleName {
        /// The name.
        name: ModuleName,
    },
    /// The bundle has no modules or more than the cap.
    #[error("bundle of {actual} modules is outside 1..={max}")]
    ModuleCount {
        /// The module count.
        actual: usize,
        /// `max_modules_per_bundle`.
        max: u16,
    },
    /// A module imports from a bundle-internal module that does not exist.
    #[error("module `{importer}` imports from unknown module `{target}`")]
    UnknownBindingTarget {
        /// The importing module.
        importer: ModuleName,
        /// The named target.
        target: ModuleName,
    },
    /// A module imports an export the target does not provide.
    #[error("module `{importer}` imports `{export}` which module `{target}` does not export as a function")]
    MissingBindingExport {
        /// The importing module.
        importer: ModuleName,
        /// The target module.
        target: ModuleName,
        /// The export name.
        export: String,
    },
    /// The importer's declared signature differs from the target's export.
    #[error("binding `{importer}` -> `{target}`.`{export}` imports {imported} but the target exports {exported}")]
    BindingSignature {
        /// The importing module.
        importer: ModuleName,
        /// The target module.
        target: ModuleName,
        /// The export name.
        export: String,
        /// The importer's signature.
        imported: Box<FuncSignature>,
        /// The target's signature.
        exported: Box<FuncSignature>,
    },
    /// The declared binding list differs from the bindings the code establishes.
    #[error("declared bindings do not match the module imports: {detail}")]
    BindingsMismatch {
        /// A description of the first difference.
        detail: String,
    },
    /// The dependency graph has a cycle.
    #[error("dependency cycle among modules {modules:?}")]
    DependencyCycle {
        /// Every module on or downstream of the cycle, in canonical order.
        modules: Vec<ModuleName>,
    },
    /// A declared entry names a module the bundle does not contain.
    #[error("entry `{module}`.`{export}` names an unknown module")]
    EntryModuleUnknown {
        /// The module.
        module: ModuleName,
        /// The export.
        export: String,
    },
    /// A declared entry names an export the module does not have with the entry signature.
    #[error("entry `{module}`.`{export}` is not an exported function with the entry signature")]
    EntryNotFound {
        /// The module.
        module: ModuleName,
        /// The export.
        export: String,
    },
    /// The same entry is declared twice.
    #[error("duplicate entry `{module}`.`{export}`")]
    DuplicateEntry {
        /// The module.
        module: ModuleName,
        /// The export.
        export: String,
    },
    /// No entry is declared.
    #[error("a bundle needs at least one entry")]
    NoEntries,
}

/// Preparation of a bundle failed.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PreparationError {
    /// One module was refused.
    #[error("module `{name}`: {source}")]
    Module {
        /// The module.
        name: ModuleName,
        /// The refusal.
        #[source]
        source: ModuleError,
    },
    /// The bundle as a whole was refused.
    #[error(transparent)]
    Bundle(#[from] BundleError),
}
