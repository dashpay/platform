use crate::version::fee::dashvm::FeeDashVmVersion;

/// Introduced with protocol version 17 (5.0).
pub const FEE_DASHVM_VERSION1: FeeDashVmVersion = FeeDashVmVersion {
    // provisional: allocation register "Provisional price", 1 processing credit per computation
    // unit; measured and revised before activation
    credits_per_computation_unit: 1,
};
