use bincode::{Decode, Encode};

pub mod v1;

/// The prices of smart-contract work.
///
/// Absent (`FeeVersion::dashvm == None`) on every schedule that predates smart contracts, so a
/// schedule without contract pricing cannot price computation at zero by accident: pricing a
/// unit at zero is never intended, unlike the genuinely free fees elsewhere in the tables.
///
/// One row today, the price of a computation unit. The remaining register rows (deployment
/// validation, readiness verification, host entry, per-byte copy) are added to this group and
/// its `FEE_DASHVM_VERSION*` constant in place while the protocol version that introduces it is
/// unreleased.
///
/// Consumers read this group from the active protocol version
/// (`platform_version.fee_version.dashvm`), never from the persisted epoch fee history or any
/// other lookup by `fee_version_number`: those resolve to the registered fee-history generation,
/// which serves only the storage, processing, hashing and signature groups and carries no
/// contract pricing. `dpp::fee::smart_contract_computation::computation_units_to_credits` takes
/// `&PlatformVersion` for that reason.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeDashVmVersion {
    /// Processing credits charged per computation unit consumed by a contract invocation.
    /// The charge is `units * credits_per_computation_unit` with checked arithmetic
    /// (`dpp::fee::smart_contract_computation::computation_units_to_credits`) and enters the
    /// processing fee of the invocation, which is what Tenderdash's gas fields report.
    pub credits_per_computation_unit: u64,
}

#[cfg(test)]
mod tests {
    use super::FeeDashVmVersion;

    #[test]
    // If this test failed, then a new field was added in FeeDashVmVersion. And the corresponding eq needs to be updated as well
    fn test_fee_dashvm_version_equality() {
        let version1 = FeeDashVmVersion {
            credits_per_computation_unit: 1,
        };

        let version2 = FeeDashVmVersion {
            credits_per_computation_unit: 1,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeDashVmVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
