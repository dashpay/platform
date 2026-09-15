//! The DashVM table: which engine profile, preparation generation and metering generation a
//! protocol version selects, together with the numeric limits and metering weights those
//! generations read.
//!
//! The table is the single home of every DashVM number. The two runtime crates
//! (`dashvm-validation` and `dashvm`) project it into their own profile types and never define
//! a limit or a weight themselves, so a reviewer looking for "what bounds a contract at
//! protocol version N" finds the answer here and nowhere else. This follows the precedent of
//! `DriveAbciWithdrawalConstants`: a dedicated constants table for one domain, referenced from
//! the platform version, backfilled on every shipped version.
//!
//! `PlatformVersion::dashvm` is an `Option`: `None` on every protocol version that predates
//! smart contracts (the exact pre-feature value, so those versions stay behaviour-preserving),
//! `Some` from the first version that can prepare and execute contract code. A node asked to
//! prepare a bundle under a version whose slot is `None` gets a versioning error, never a paid
//! rejection.

use versioned_feature_core::FeatureVersion;

pub mod v1;

/// The DashVM configuration one protocol version selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashVmVersion {
    /// Selects the engine profile the runtime builds: the pinned Wasmtime release, the Cranelift
    /// flags, the target CPU feature policy, the memory layout and the admitted feature set at
    /// the engine level. Profile `N` is `dashvm::engine::profile::v{N}`. A new engine release
    /// is a new profile version added alongside the old one, never an edit of an existing one,
    /// so historical blocks replay under the engine that ran them.
    pub engine_profile: FeatureVersion,
    /// Selects the preparation generation of `dashvm-validation`: the admission rules, the
    /// structural bounds, the import and export rules and the logical-stack instrumentation
    /// that turn submitted canonical code into prepared code. Prepared bytes are a pure function
    /// of the canonical bytes and this number.
    pub preparation: FeatureVersion,
    /// Selects the computation schedule the runtime meters with. Generation 0 is Wasmtime's fuel
    /// schedule (one unit per metered operator, audited against the pinned engine) plus the
    /// weighted host charges in [`DashVmMeteringWeights`]. Weighted per-operator schedules are
    /// later generations.
    pub metering: FeatureVersion,
    /// The numeric limits the preparation and engine generations enforce.
    pub limits: DashVmLimits,
    /// The weights of the host-side work the metering generation charges.
    pub weights: DashVmMeteringWeights,
}

/// Numeric limits on contract code and contract execution.
///
/// Every value is provisional: the allocation register supplies engineering starting values and
/// asks for measurement before a network is asked to run them. Structural limits are enforced at
/// preparation, memory and invocation limits at run time, and both kinds are consensus rules:
/// a module that exceeds a cap is rejected identically on every node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashVmLimits {
    /// Most bytes one canonical (submitted, un-instrumented) module may have. Checked before any
    /// decoding so an oversized submission costs nothing but the length comparison.
    pub max_canonical_module_bytes: u32,
    /// Most named modules one bundle may contain; also bounds the instances one invocation can
    /// create.
    pub max_modules_per_bundle: u16,
    /// Most bytes in one module name.
    pub max_module_name_bytes: u8,
    /// Most functions (imported plus defined) in one module.
    pub max_functions_per_module: u32,
    /// Most types in one module's type section.
    pub max_types_per_module: u32,
    /// Most parameters of one function type.
    pub max_params_per_function: u32,
    /// Most declared locals of one function body (parameters excluded).
    pub max_locals_per_function: u32,
    /// Most exports of one module.
    pub max_exports_per_module: u32,
    /// Most decoded operators across every function body of one module.
    pub max_operators_per_module: u32,
    /// Most decoded operators in one function body.
    pub max_operators_per_function: u32,
    /// Most basic blocks in one function body, counted as the operators that begin one
    /// (`block`, `loop`, `if`, `else`, `end` and every branch).
    pub max_basic_blocks_per_function: u32,
    /// Deepest structural nesting of control frames in one function body.
    pub max_nesting_depth: u32,
    /// Most bytes the prepared (instrumented) module may have. Instrumentation grows a module by
    /// a bounded factor; this cap rejects growth that would burden compilation.
    pub max_prepared_module_bytes: u32,
    /// Most 64 KiB pages a module may declare as its initial memory.
    pub max_initial_memory_pages: u32,
    /// Most 64 KiB pages one instance's memory may reach through `memory.grow`; also the upper
    /// bound a module may declare as its memory maximum.
    pub max_memory_pages_per_instance: u32,
    /// Most 64 KiB pages every instance of one outer invocation may hold together, nested
    /// contract calls included.
    pub max_memory_pages_per_invocation: u32,
    /// Most elements of a module's function table.
    pub max_table_elements: u32,
    /// Most bytes of logical stack (frames, locals and operand slots as accounted by the
    /// preparation generation) one outer invocation may hold across every module and nested
    /// call.
    pub max_logical_stack_bytes: u32,
    /// Most guest activation frames one outer invocation may hold at once.
    pub max_activation_depth: u32,
    /// Most host calls one outer invocation may make, nested work included.
    pub max_host_calls_per_invocation: u32,
    /// Most bytes of arguments one entry invocation or host call may pass.
    pub max_argument_bytes: u32,
    /// Most bytes one entry invocation or host call may return.
    pub max_return_bytes: u32,
    /// Most bytes of active data segments one module may initialise its memory with.
    pub max_data_segment_bytes_per_module: u32,
    /// Most active contract frames one outer invocation may open, the root included.
    pub max_nested_contract_frames: u8,
}

/// Weights, in computation units, of the host-side work the metering generation charges on top
/// of the guest operator schedule.
///
/// Provisional register values; measured and revised before activation. All charges use checked
/// arithmetic against the invocation budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashVmMeteringWeights {
    /// Charged on every entry into a host function, before the operation's own cost.
    pub host_entry: u64,
    /// Charged per byte copied between guest and host memory or between two module memories.
    pub copied_byte: u64,
    /// Charged per byte a module's data and element segments initialise at instantiation.
    pub initialised_byte: u64,
    /// Charged per 64 KiB page a successful `memory.grow` adds.
    pub memory_growth_page: u64,
    /// Charged once per module instance created for an invocation.
    pub instance_base: u64,
}

#[cfg(test)]
mod tests {
    use crate::version::protocol_version::PLATFORM_VERSIONS;
    use crate::version::v17::PROTOCOL_VERSION_17;
    use crate::version::{PlatformVersion, LATEST_VERSION};

    /// The limits and weights only make sense as a whole, so this checks the invariants that
    /// must hold whatever the measured numbers turn out to be, on every registered version that
    /// carries the table, rather than restating the literal: the per-instance memory bound
    /// contains the initial memory and is contained by the per-invocation bound, the
    /// per-function operator cap fits the per-module cap, a canonical module fits the prepared
    /// cap, the logical stack has room for the activation depth, every cap is non-zero (a zero
    /// cap would reject every module) and nothing before the 5.0 protocol version carries a
    /// table at all.
    #[test]
    fn dashvm_limits_are_well_formed_wherever_the_table_exists() {
        assert_eq!(
            PLATFORM_VERSIONS.len(),
            LATEST_VERSION as usize,
            "the protocol version registry does not hold every declared version"
        );
        assert!(
            PlatformVersion::latest().dashvm.is_some(),
            "the latest protocol version must carry the DashVM table"
        );
        for platform_version in PLATFORM_VERSIONS {
            let Some(dashvm) = platform_version.dashvm.as_ref() else {
                continue;
            };
            let version = platform_version.protocol_version;
            assert!(
                version >= PROTOCOL_VERSION_17,
                "protocol version {version} predates smart contracts and must not carry a DashVM table"
            );
            let limits = &dashvm.limits;
            assert!(
                limits.max_initial_memory_pages <= limits.max_memory_pages_per_instance,
                "protocol version {version}: the initial memory must fit in an instance"
            );
            assert!(
                limits.max_memory_pages_per_instance <= limits.max_memory_pages_per_invocation,
                "protocol version {version}: one instance must fit in the invocation reservation"
            );
            assert!(
                limits.max_operators_per_function <= limits.max_operators_per_module,
                "protocol version {version}: one function must fit in the module operator cap"
            );
            assert!(
                limits.max_canonical_module_bytes <= limits.max_prepared_module_bytes,
                "protocol version {version}: instrumentation must be allowed to grow a module"
            );
            assert!(
                limits.max_activation_depth <= limits.max_logical_stack_bytes,
                "protocol version {version}: every activation frame needs at least one byte"
            );
            assert!(
                limits.max_return_bytes <= limits.max_argument_bytes,
                "protocol version {version}: a return value is expected to fit where arguments do"
            );
            let caps = [
                u64::from(limits.max_canonical_module_bytes),
                u64::from(limits.max_modules_per_bundle),
                u64::from(limits.max_module_name_bytes),
                u64::from(limits.max_functions_per_module),
                u64::from(limits.max_types_per_module),
                u64::from(limits.max_params_per_function),
                u64::from(limits.max_locals_per_function),
                u64::from(limits.max_exports_per_module),
                u64::from(limits.max_operators_per_module),
                u64::from(limits.max_operators_per_function),
                u64::from(limits.max_basic_blocks_per_function),
                u64::from(limits.max_nesting_depth),
                u64::from(limits.max_prepared_module_bytes),
                u64::from(limits.max_initial_memory_pages),
                u64::from(limits.max_memory_pages_per_instance),
                u64::from(limits.max_memory_pages_per_invocation),
                u64::from(limits.max_table_elements),
                u64::from(limits.max_logical_stack_bytes),
                u64::from(limits.max_activation_depth),
                u64::from(limits.max_host_calls_per_invocation),
                u64::from(limits.max_argument_bytes),
                u64::from(limits.max_return_bytes),
                u64::from(limits.max_data_segment_bytes_per_module),
                u64::from(limits.max_nested_contract_frames),
            ];
            assert!(
                caps.iter().all(|cap| *cap > 0),
                "protocol version {version}: a zero cap would reject every module"
            );
            assert!(
                dashvm.weights.host_entry > 0 && dashvm.weights.instance_base > 0,
                "protocol version {version}: host entry and instantiation must cost something"
            );
        }
    }
}
