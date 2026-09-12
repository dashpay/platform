use crate::version::system_limits::smart_contract::SmartContractComputationLimits;
use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 17 (5.0) and above.
///
/// Identical to [`SYSTEM_LIMITS_V4`] except that smart-contract computation is bounded: one
/// outer invocation may consume at most 25 million computation units and all invocations in a
/// block at most 250 million. The numbers are the provisional starting values of the DashVM
/// shared allocation register (the "Compute" row); they are measured and revised before any
/// network is asked to run this protocol version. See `SmartContractComputationLimits` for what
/// a unit is and how the two limits are enforced.
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    smart_contract_computation: Some(SmartContractComputationLimits {
        // provisional: allocation register "Compute" row, 25 million units per outer invocation
        max_computation_units_per_invocation: 25_000_000,
        // provisional: allocation register "Compute" row, 250 million contract units per block
        max_computation_units_per_block: 250_000_000,
    }),
    ..SYSTEM_LIMITS_V4
};

// Whatever the measured revision of the numbers above is, both limits stay non-zero and one
// maximal invocation still fits in a block; otherwise the per-block reservation could never admit
// an invocation that uses its full per-invocation budget.
const _: () = assert!(
    match SYSTEM_LIMITS_V5.smart_contract_computation {
        Some(ref limits) => limits.is_well_formed(),
        None => false,
    },
    "SYSTEM_LIMITS_V5 must carry well-formed smart-contract computation limits"
);
