//! Pricing of smart-contract computation.
//!
//! Contract work is metered by the runtime in [`ComputationUnits`], a deterministic count under
//! the active metering generation, and bounded per invocation and per block by
//! [`SmartContractComputationLimits`] in the protocol version's system limits. This module turns
//! the units an invocation consumed into credits at the protocol-versioned price of the fee
//! schedule (`FeeVersion::dashvm`).
//!
//! The price is read from the fee schedule of the **active protocol version**
//! (`platform_version.fee_version.dashvm`), never from the persisted epoch fee history. That
//! history is keyed by `fee_version_number`, records a schedule only when that number changes,
//! and is restored from saved state through `FeeVersion::get(number)`; it serves the storage,
//! processing, hashing and signature groups (`KnownCostItem`) and the storage refund rates, and
//! nothing else. A schedule that adds contract pricing does not change the number, so the history
//! never carries the `dashvm` group, exactly as it never carried `data_contract_registration`,
//! `state_transition_min_fees` or `vote_resolution_fund_fees`, all of which are likewise read from
//! the active protocol version. The function below therefore takes `&PlatformVersion`, so a
//! history entry cannot be passed to it by mistake.
//!
//! The charge enters the processing fee of the invocation's `FeeResult`, exactly like every other
//! processing charge, and therefore reaches Tenderdash through the existing `gas_used` and
//! `gas_wanted` fields, which report `FeeResult::total_base_fee()` in credits. Gas stays
//! denominated in credits; computation units are never reported to Tenderdash and no unit
//! equivalence between them and Tenderdash gas exists.

use crate::fee::Credits;
use crate::ProtocolError;
pub use platform_version::version::system_limits::smart_contract::{
    ComputationUnits, SmartContractComputationLimits,
};
use platform_version::version::PlatformVersion;

/// Prices `units` of smart-contract computation in credits at the active protocol version's
/// rate (`platform_version.fee_version.dashvm.credits_per_computation_unit`).
///
/// The table is the versioned part: a schedule that prices computation differently is a new
/// `FEE_VERSION*` with a different `dashvm` group, not a new generation of this function.
/// Callers on a block path pass the version from platform state
/// (`platform_state.current_platform_version()`), never `PlatformVersion::latest()` and never a
/// schedule taken from the epoch fee history (see the module documentation).
///
/// # Errors
///
/// * `ProtocolError::CorruptedCodeExecution` when the protocol version has no smart-contract
///   pricing. A caller only reaches this function after the protocol version admitted contract
///   execution, and the tables guarantee that such a version prices computation, so a missing
///   price is a broken build rather than a user mistake.
/// * `ProtocolError::Overflow` when the charge does not fit in `Credits`. With the provisional
///   rate of one credit per unit and limits far below `u64::MAX` this is unreachable, but the
///   arithmetic is checked so that no revision of either table can wrap a fee.
pub fn computation_units_to_credits(
    units: ComputationUnits,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let price = platform_version
        .fee_version
        .dashvm
        .as_ref()
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(format!(
                "computation_units_to_credits requires fee_version.dashvm, which protocol version {} does not carry",
                platform_version.protocol_version
            ))
        })?;

    units
        .checked_mul(price.credits_per_computation_unit)
        .ok_or(ProtocolError::Overflow(
            "smart-contract computation charge overflowed credits",
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::fee::dashvm::FeeDashVmVersion;
    use platform_version::version::fee::FeeVersion;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_price_computation_units_at_the_schedule_rate() {
        let platform_version = PlatformVersion::latest();
        let limits = platform_version
            .system_limits
            .smart_contract_computation
            .as_ref()
            .expect("the latest protocol version bounds smart-contract computation");
        let price = platform_version
            .fee_version
            .dashvm
            .as_ref()
            .expect("the latest protocol version prices smart-contract computation");

        let units = limits.max_computation_units_per_invocation;

        let credits = computation_units_to_credits(units, platform_version)
            .expect("a maximal invocation must be priceable");

        assert_eq!(credits, units * price.credits_per_computation_unit);
    }

    #[test]
    fn should_price_zero_units_as_zero_credits() {
        let platform_version = PlatformVersion::latest();

        let credits = computation_units_to_credits(0, platform_version)
            .expect("zero units must be priceable");

        assert_eq!(credits, 0);
    }

    #[test]
    fn should_fail_with_overflow_when_the_charge_does_not_fit_in_credits() {
        let latest = PlatformVersion::latest();
        let platform_version = PlatformVersion {
            fee_version: FeeVersion {
                dashvm: Some(FeeDashVmVersion {
                    credits_per_computation_unit: 2,
                }),
                ..latest.fee_version.clone()
            },
            ..latest.clone()
        };

        let result = computation_units_to_credits(u64::MAX, &platform_version);

        assert!(
            matches!(result, Err(ProtocolError::Overflow(_))),
            "expected an overflow error, got {result:?}"
        );
    }

    #[test]
    fn should_report_corrupted_code_execution_when_the_protocol_version_has_no_smart_contract_pricing(
    ) {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        assert!(
            platform_version.fee_version.dashvm.is_none(),
            "protocol version 14 predates smart-contract pricing"
        );

        let result = computation_units_to_credits(1, platform_version);

        assert!(
            matches!(result, Err(ProtocolError::CorruptedCodeExecution(_))),
            "expected a corrupted code execution error, got {result:?}"
        );
    }
}
