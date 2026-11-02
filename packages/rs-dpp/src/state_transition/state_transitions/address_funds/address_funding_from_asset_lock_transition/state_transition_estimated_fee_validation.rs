use crate::balances::credits::CREDITS_PER_DUFF;
use crate::fee::Credits;
use crate::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionWitnessSigned,
};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Calculate the static minimum required fee for address funding from an asset lock.
///
/// This is the count-based DPP admission-floor formula used by
/// `AddressFundingFromAssetLockTransition::calculate_min_required_fee`, exposed for
/// callers that need to reserve funds before constructing the transition.
pub fn calculate_address_funding_from_asset_lock_min_required_fee(
    input_count: usize,
    output_count: usize,
    platform_version: &PlatformVersion,
) -> Credits {
    let min_fees = &platform_version.fee_version.state_transition_min_fees;
    let asset_lock_base_cost = platform_version
        .dpp
        .state_transitions
        .identities
        .asset_locks
        .required_asset_lock_duff_balance_for_processing_start_for_address_funding
        * CREDITS_PER_DUFF;
    let output_count = output_count.max(1);

    asset_lock_base_cost.saturating_add(
        min_fees
            .address_funds_transfer_input_cost
            .saturating_mul(input_count as u64)
            .saturating_add(
                min_fees
                    .address_funds_transfer_output_cost
                    .saturating_mul(output_count as u64),
            ),
    )
}

impl StateTransitionEstimatedFeeValidation for AddressFundingFromAssetLockTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(calculate_address_funding_from_asset_lock_min_required_fee(
            self.inputs().len(),
            self.outputs().len(),
            platform_version,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::PlatformAddress;
    use crate::version::LATEST_PLATFORM_VERSION;
    use std::collections::BTreeMap;

    fn platform_address(seed: usize) -> PlatformAddress {
        let mut hash = [0u8; 20];
        hash[12..20].copy_from_slice(&(seed as u64).to_be_bytes());
        PlatformAddress::P2pkh(hash)
    }

    fn transition_with_counts(
        input_count: usize,
        output_count: usize,
    ) -> AddressFundingFromAssetLockTransition {
        let mut transition =
            AddressFundingFromAssetLockTransition::default_versioned(LATEST_PLATFORM_VERSION)
                .expect("latest address funding transition");

        let inputs = (0..input_count)
            .map(|index| (platform_address(index), (index as u32, 1_000_000)))
            .collect::<BTreeMap<_, _>>();
        let outputs = (0..output_count)
            .map(|index| {
                let amount = if index + 1 == output_count {
                    None
                } else {
                    Some(1_000_000)
                };
                (platform_address(1_000 + index), amount)
            })
            .collect::<BTreeMap<_, _>>();

        transition.set_inputs(inputs);
        transition.set_outputs(outputs);
        transition
    }

    #[test]
    fn count_based_helper_matches_current_static_address_funding_fee() {
        let cases = [
            (0, 0, 56_000_000),
            (0, 1, 56_000_000),
            (0, 2, 62_000_000),
            (1, 1, 56_500_000),
            (1, 2, 62_500_000),
            (3, 5, 81_500_000),
            (10, 100, 655_000_000),
        ];

        for (input_count, output_count, expected_fee) in cases {
            let fee = calculate_address_funding_from_asset_lock_min_required_fee(
                input_count,
                output_count,
                LATEST_PLATFORM_VERSION,
            );

            assert_eq!(fee, expected_fee);
        }
    }

    #[test]
    fn count_based_helper_matches_transition_min_required_fee() {
        let cases = [(0, 0), (0, 1), (0, 2), (1, 2), (3, 5), (10, 100)];

        for (input_count, output_count) in cases {
            let transition = transition_with_counts(input_count, output_count);
            let helper_fee = calculate_address_funding_from_asset_lock_min_required_fee(
                input_count,
                output_count,
                LATEST_PLATFORM_VERSION,
            );
            let transition_fee = transition
                .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
                .expect("transition fee");

            assert_eq!(helper_fee, transition_fee);
        }
    }
}
