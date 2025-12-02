use crate::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::v0::deduct_fee_from_outputs_or_remaining_balance_of_inputs_v0;
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;

/// Result of deducting fees from outputs or remaining balance of inputs.
///
/// This struct contains the adjusted balances after applying the fee strategy,
/// along with information about whether the fee was fully covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeDeductionResult {
    /// The remaining balance of input addresses after fee deduction.
    /// If an address has its balance reduced to zero, it may be removed from this map.
    pub remaining_input_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,

    /// The adjusted output amounts after fee deduction.
    /// If an output has its amount reduced to zero, it may be removed from this map.
    pub adjusted_outputs: BTreeMap<PlatformAddress, Credits>,

    /// Whether the fee was fully covered by the available funds.
    /// If false, the fee strategy steps were exhausted before the full fee could be deducted.
    pub fee_fully_covered: bool,
}

pub fn deduct_fee_from_outputs_or_remaining_balance_of_inputs(
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    outputs: BTreeMap<PlatformAddress, Credits>,
    fee_strategy: &AddressFundsFeeStrategy,
    fee: Credits,
    platform_version: &PlatformVersion,
) -> Result<FeeDeductionResult, ProtocolError> {
    match platform_version
        .dpp
        .methods
        .deduct_fee_from_outputs_or_remaining_balance_of_inputs
    {
        0 => deduct_fee_from_outputs_or_remaining_balance_of_inputs_v0(
            inputs,
            outputs,
            fee_strategy,
            fee,
        ),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "deduct_fee_from_outputs_or_remaining_balance_of_inputs".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
