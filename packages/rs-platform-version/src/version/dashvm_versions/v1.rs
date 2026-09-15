use crate::version::dashvm_versions::{DashVmLimits, DashVmMeteringWeights, DashVmVersion};

/// The first DashVM configuration, selected by the 5.0 protocol version.
///
/// Every number is the DashVM allocation register's provisional starting value (limits under
/// A06, weights under A15, bundle shape under A08). They are engineering guesses requested by
/// the owner, to be benchmarked before any network proposes the version that carries them; the
/// measurement and revision tasks are recorded in the register.
pub const DASHVM_VERSION_V1: DashVmVersion = DashVmVersion {
    engine_profile: 0,
    preparation: 0,
    metering: 0,
    limits: DashVmLimits {
        max_canonical_module_bytes: 16 * 1024 * 1024,
        max_modules_per_bundle: 16,
        max_module_name_bytes: 64,
        max_functions_per_module: 50_000,
        max_types_per_module: 10_000,
        max_params_per_function: 128,
        max_locals_per_function: 1_024,
        max_exports_per_module: 1_024,
        max_operators_per_module: 2_000_000,
        max_operators_per_function: 100_000,
        max_basic_blocks_per_function: 4_096,
        max_nesting_depth: 512,
        max_prepared_module_bytes: 64 * 1024 * 1024,
        max_initial_memory_pages: 256,          // 16 MiB
        max_memory_pages_per_instance: 2_048,   // 128 MiB
        max_memory_pages_per_invocation: 4_096, // 256 MiB
        max_table_elements: 65_536,
        max_logical_stack_bytes: 1024 * 1024,
        max_activation_depth: 512,
        max_host_calls_per_invocation: 10_000,
        max_argument_bytes: 256 * 1024,
        max_return_bytes: 64 * 1024,
        max_data_segment_bytes_per_module: 16 * 1024 * 1024,
        max_nested_contract_frames: 8,
    },
    weights: DashVmMeteringWeights {
        host_entry: 100,
        copied_byte: 1,
        initialised_byte: 1,
        memory_growth_page: 65_536,
        instance_base: 1_000,
    },
};
