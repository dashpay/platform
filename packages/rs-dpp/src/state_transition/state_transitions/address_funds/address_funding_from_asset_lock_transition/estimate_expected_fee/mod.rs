mod v0;

use crate::fee::Credits;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl AddressFundingFromAssetLockTransition {
    /// Estimate the fee the network is EXPECTED to actually charge for an
    /// address funding, given input and output counts, without needing a
    /// constructed transition.
    ///
    /// This is a client-side DISPLAY/PLANNING estimate of the
    /// GroveDB-metered execution fee — deliberately DISTINCT from
    /// [`calculate_min_required_fee`], which is the consensus floor the
    /// locked value must cover before processing starts (the floor is
    /// several times larger than the metered charge). No consensus path
    /// reads this estimate.
    ///
    /// NOT an upper bound — callers MUST NOT size funding locks (or any
    /// execution budget) from it. The charged fee is metered on live
    /// state, grows with `user_fee_increase` (the SDK's stuck-ST retry
    /// bumps it), and `validate_fees_of_event` additionally requires the
    /// fee strategy to cover an average-case estimate that can exceed
    /// this number. Locks must carry a conservative reserve instead; a
    /// miss here shows a slightly-off display number, nothing more.
    ///
    /// The constants live in the versioned fee tables
    /// (`state_transition_min_fees`) and are pinned against real
    /// `apply = true` execution by the drive-abci
    /// `expected_fee_calibration` tests, with headroom for state-tree
    /// growth (the metered fee drifts logarithmically with tree size).
    ///
    /// [`calculate_min_required_fee`]:
    ///     crate::state_transition::StateTransitionEstimatedFeeValidation::calculate_min_required_fee
    pub fn estimate_expected_fee(
        input_count: usize,
        output_count: usize,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match platform_version
            .dpp
            .methods
            .estimate_address_funding_expected_fee
        {
            0 => Self::estimate_expected_fee_v0(input_count, output_count, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressFundingFromAssetLockTransition::estimate_expected_fee".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
