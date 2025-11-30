use crate::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::ProtocolError;
use std::collections::BTreeMap;

/// Deducts the fee from outputs or remaining balance of inputs according to the fee strategy.
///
/// The inputs represent the REMAINING BALANCE after the transfer (e.g., if address A had 10
/// and we're sending 3, the input shows A: 7 remaining).
///
/// The fee strategy determines the priority order for deducting the fee:
/// - `DeductFromInput(i)`: Reduce the ith input's remaining balance (address keeps less)
/// - `ReduceOutput(i)`: Reduce the ith output amount (less is sent to that address)
///
/// The strategy steps are applied in order until the fee is fully covered.
pub fn deduct_fee_from_outputs_or_remaining_balance_of_inputs_v0(
    mut inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    mut outputs: BTreeMap<PlatformAddress, Credits>,
    fee_strategy: AddressFundsFeeStrategy,
    fee: Credits,
) -> Result<
    (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    ),
    ProtocolError,
> {
    let mut remaining_fee = fee;

    for step in fee_strategy {
        if remaining_fee == 0 {
            break;
        }

        match step {
            AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                // Reduce the remaining balance of the input at the specified index
                if let Some((&address, &(nonce, amount))) = inputs.iter().nth(index as usize) {
                    let reduction = remaining_fee.min(amount);
                    let new_amount = amount - reduction;
                    remaining_fee -= reduction;

                    if new_amount == 0 {
                        inputs.remove(&address);
                    } else {
                        inputs.insert(address, (nonce, new_amount));
                    }
                }
            }
            AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                // Reduce the output at the specified index
                if let Some((&address, &amount)) = outputs.iter().nth(index as usize) {
                    let reduction = remaining_fee.min(amount);
                    let new_amount = amount - reduction;
                    remaining_fee -= reduction;

                    if new_amount == 0 {
                        outputs.remove(&address);
                    } else {
                        outputs.insert(address, new_amount);
                    }
                }
            }
        }
    }

    Ok((inputs, outputs))
}
