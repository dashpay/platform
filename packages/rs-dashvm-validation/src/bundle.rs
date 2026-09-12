//! Descriptor types for prepared modules and bundles.
//!
//! These are the provisional home of the manifest-facing descriptors (module names, interfaces,
//! bindings, entries) until the ABI crate takes over the manifest shape; the runtime consumes
//! them as they are. None of them leaks a `wasmparser` type: the runtime and the tooling above
//! this crate see only DashVM-owned values.

use crate::errors::BundleError;
use crate::hashing::{BundleDigest, CanonicalHash, PreparedHash};
use crate::instrumentation::InstrumentationReport;
use platform_version::version::FeatureVersion;
use std::fmt;

/// A WebAssembly value type admitted by the profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ValueType {
    /// 32-bit integer.
    I32,
    /// 64-bit integer.
    I64,
    /// 32-bit float.
    F32,
    /// 64-bit float.
    F64,
    /// Nullable function reference.
    FuncRef,
}

impl ValueType {
    /// The one-byte tag used in canonical encodings (bundle digest).
    pub(crate) fn tag(self) -> u8 {
        match self {
            ValueType::I32 => 0x7f,
            ValueType::I64 => 0x7e,
            ValueType::F32 => 0x7d,
            ValueType::F64 => 0x7c,
            ValueType::FuncRef => 0x70,
        }
    }

    /// Bytes one value of this type occupies in the logical stack accounting: 4 for 32-bit
    /// scalars, 8 for 64-bit scalars and for references (pointer-sized on every supported host).
    pub(crate) fn logical_bytes(self) -> u64 {
        match self {
            ValueType::I32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::F64 | ValueType::FuncRef => 8,
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ValueType::I32 => "i32",
            ValueType::I64 => "i64",
            ValueType::F32 => "f32",
            ValueType::F64 => "f64",
            ValueType::FuncRef => "funcref",
        })
    }
}

/// A function signature: parameter and result types.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncSignature {
    /// Parameter types in order.
    pub params: Vec<ValueType>,
    /// Result types in order.
    pub results: Vec<ValueType>,
}

impl FuncSignature {
    /// Builds a signature.
    pub fn new(params: Vec<ValueType>, results: Vec<ValueType>) -> Self {
        Self { params, results }
    }

    pub(crate) fn encode_into(&self, out: &mut Vec<u8>) {
        encode_u32(out, self.params.len() as u32);
        out.extend(self.params.iter().map(|ty| ty.tag()));
        encode_u32(out, self.results.len() as u32);
        out.extend(self.results.iter().map(|ty| ty.tag()));
    }
}

impl fmt::Display for FuncSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(")?;
        for (i, ty) in self.params.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{ty}")?;
        }
        f.write_str(") -> (")?;
        for (i, ty) in self.results.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{ty}")?;
        }
        f.write_str(")")
    }
}

/// A validated module name: one to `max_module_name_bytes` characters from `[a-z0-9_]`.
///
/// Names order canonically (byte order), which is the tie-break for every deterministic
/// ordering in a bundle.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleName(String);

impl ModuleName {
    /// Parses and validates a name against the profile's length cap.
    pub fn parse(name: &str, max_bytes: u8) -> Result<Self, BundleError> {
        let invalid = |reason| BundleError::InvalidModuleName {
            name: name.to_owned(),
            reason,
        };
        if name.is_empty() {
            return Err(invalid("empty"));
        }
        if name.len() > usize::from(max_bytes) {
            return Err(invalid("longer than the cap"));
        }
        if !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(invalid("character outside [a-z0-9_]"));
        }
        Ok(Self(name.to_owned()))
    }

    /// The name as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A function the module imports from the host envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostImport {
    /// The envelope function name.
    pub name: String,
    /// Its signature, equal to the envelope's definition.
    pub signature: FuncSignature,
}

/// A function the module imports from another module of the same bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InternalImport {
    /// The target module.
    pub target: ModuleName,
    /// The target's export name.
    pub export: String,
    /// The signature the importer declares.
    pub signature: FuncSignature,
}

/// A function the module exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedFunction {
    /// The export name.
    pub name: String,
    /// Its signature.
    pub signature: FuncSignature,
    /// Whether the signature is the entry signature, so the export may be named as an entry.
    pub is_entry_candidate: bool,
}

/// The shape of the module's single linear memory, in 64 KiB pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryShape {
    /// Initial size.
    pub initial_pages: u32,
    /// Declared maximum, if any.
    pub maximum_pages: Option<u32>,
}

/// The shape of the module's function table, if it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableShape {
    /// Initial element count.
    pub initial_elements: u32,
    /// Declared maximum element count (required).
    pub maximum_elements: u32,
}

/// Everything another module, the host or the runtime needs to link against a module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleInterface {
    /// Host envelope functions imported, in import order.
    pub host_imports: Vec<HostImport>,
    /// Bundle-internal functions imported, in import order.
    pub internal_imports: Vec<InternalImport>,
    /// Exported functions, in export order.
    pub exports: Vec<ExportedFunction>,
    /// The memory.
    pub memory: MemoryShape,
    /// The table.
    pub table: Option<TableShape>,
}

impl ModuleInterface {
    /// The exported function of that name, if any.
    pub fn export(&self, name: &str) -> Option<&ExportedFunction> {
        self.exports.iter().find(|export| export.name == name)
    }
}

/// What preparation measured about a module's structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralMeasurements {
    /// Length of the canonical bytes.
    pub canonical_bytes: u64,
    /// Length of the prepared bytes.
    pub prepared_bytes: u64,
    /// Imported functions.
    pub imported_functions: u32,
    /// Defined functions in the canonical module.
    pub defined_functions: u32,
    /// Types in the canonical module.
    pub types: u32,
    /// Exports.
    pub exports: u32,
    /// Decoded operators across every canonical function body.
    pub operators: u64,
    /// Most operators in one canonical function body.
    pub max_operators_per_function: u32,
    /// Most basic blocks in one canonical function body.
    pub max_basic_blocks_per_function: u32,
    /// Deepest control nesting in any canonical function body.
    pub max_nesting_depth: u32,
    /// Most parameters of any function type.
    pub max_params: u32,
    /// Most declared locals of any canonical function body.
    pub max_locals: u32,
    /// Most operand-stack slots any canonical function body reaches.
    pub max_operand_slots: u32,
}

/// What instantiation of the prepared module initialises, for the runtime to charge before
/// creating the instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitializationMeasurements {
    /// Pages the memory starts with.
    pub initial_memory_pages: u32,
    /// Bytes of active data segments copied into memory at instantiation.
    pub active_data_bytes: u64,
    /// Bytes of passive data segments held for `memory.init`.
    pub passive_data_bytes: u64,
    /// Elements of the table at instantiation.
    pub initial_table_elements: u32,
    /// Items of active element segments written at instantiation.
    pub active_element_items: u64,
}

/// One prepared module: the output of admission and instrumentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedModule {
    /// The module's name in its bundle.
    pub name: ModuleName,
    /// Hash of the submitted canonical bytes.
    pub canonical_hash: CanonicalHash,
    /// Hash of the prepared bytes.
    pub prepared_hash: PreparedHash,
    /// The prepared bytes the engine compiles.
    pub prepared_bytes: Vec<u8>,
    /// The generation that produced the prepared bytes.
    pub preparation_generation: FeatureVersion,
    /// The interface.
    pub interface: ModuleInterface,
    /// Structural measurements.
    pub structure: StructuralMeasurements,
    /// Initialisation measurements.
    pub initialization: InitializationMeasurements,
    /// What the instrumenter added.
    pub instrumentation: InstrumentationReport,
}

/// A resolved binding: `importer` imports `dash:<target>`.`export` with `signature`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BundleBinding {
    /// The importing module.
    pub importer: ModuleName,
    /// The module providing the export.
    pub target: ModuleName,
    /// The export name on the target.
    pub export: String,
    /// The signature, equal on both sides.
    pub signature: FuncSignature,
}

/// An entry the bundle exposes: an exported function with the entry signature.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryRef {
    /// The module.
    pub module: ModuleName,
    /// The export name.
    pub export: String,
}

/// A prepared bundle: every module prepared, every binding resolved, the graph acyclic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedBundle {
    /// The modules, in canonical name order.
    pub modules: Vec<PreparedModule>,
    /// The bindings, sorted.
    pub bindings: Vec<BundleBinding>,
    /// The entries, sorted.
    pub entries: Vec<EntryRef>,
    /// Every module in an order where each module comes after all of its dependencies, with
    /// canonical name order as the tie-break. The runtime instantiates the selected entry's
    /// dependency closure in this order.
    pub initialization_order: Vec<ModuleName>,
    /// The generation that prepared every module.
    pub preparation_generation: FeatureVersion,
    /// The metering generation the protocol version selects, carried for artifact keying.
    pub metering_generation: FeatureVersion,
    /// The digest over names, hashes, bindings, entries and generations.
    pub digest: BundleDigest,
}

impl PreparedBundle {
    /// The prepared module of that name, if any.
    pub fn module(&self, name: &ModuleName) -> Option<&PreparedModule> {
        self.modules
            .binary_search_by(|module| module.name.cmp(name))
            .ok()
            .map(|index| &self.modules[index])
    }

    /// The modules `name` imports from, in canonical order, without duplicates.
    pub fn dependencies_of(&self, name: &ModuleName) -> Vec<ModuleName> {
        let mut targets: Vec<ModuleName> = self
            .bindings
            .iter()
            .filter(|binding| &binding.importer == name)
            .map(|binding| binding.target.clone())
            .collect();
        targets.dedup();
        targets
    }

    /// The canonical encoding the digest is computed over.
    pub(crate) fn digest_input(&self) -> Vec<u8> {
        let mut out = Vec::new();
        encode_u32(&mut out, u32::from(self.preparation_generation));
        encode_u32(&mut out, u32::from(self.metering_generation));
        encode_u32(&mut out, self.modules.len() as u32);
        for module in &self.modules {
            encode_str(&mut out, module.name.as_str());
            out.extend_from_slice(&module.canonical_hash.0);
            out.extend_from_slice(&module.prepared_hash.0);
        }
        encode_u32(&mut out, self.bindings.len() as u32);
        for binding in &self.bindings {
            encode_str(&mut out, binding.importer.as_str());
            encode_str(&mut out, binding.target.as_str());
            encode_str(&mut out, &binding.export);
            binding.signature.encode_into(&mut out);
        }
        encode_u32(&mut out, self.entries.len() as u32);
        for entry in &self.entries {
            encode_str(&mut out, entry.module.as_str());
            encode_str(&mut out, &entry.export);
        }
        out
    }
}

pub(crate) fn encode_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn encode_str(out: &mut Vec<u8>, value: &str) {
    encode_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_accept_lower_case_alnum_and_underscore_names_up_to_the_cap() {
        assert!(ModuleName::parse("counter_v2", 16).is_ok());
        assert!(ModuleName::parse("a", 1).is_ok());
    }

    #[test]
    fn should_reject_empty_long_and_badly_charactered_names() {
        for (name, max, reason) in [
            ("", 16, "empty"),
            ("ab", 1, "longer than the cap"),
            ("Counter", 16, "character outside [a-z0-9_]"),
            ("dash:x", 16, "character outside [a-z0-9_]"),
            ("naïve", 16, "character outside [a-z0-9_]"),
        ] {
            assert_eq!(
                ModuleName::parse(name, max),
                Err(BundleError::InvalidModuleName {
                    name: name.to_owned(),
                    reason
                })
            );
        }
    }

    #[test]
    fn should_display_signatures_in_a_readable_form() {
        let signature = FuncSignature::new(
            vec![ValueType::I32, ValueType::I64],
            vec![ValueType::F64, ValueType::FuncRef],
        );
        assert_eq!(signature.to_string(), "(i32, i64) -> (f64, funcref)");
        assert_eq!(FuncSignature::new(vec![], vec![]).to_string(), "() -> ()");
    }
}
