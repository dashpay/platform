use crate::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::ProtocolError;
use std::collections::BTreeMap;

/// Modifies inputs and outputs to cover the fee according to the fee strategy.
///
/// The fee strategy determines the priority order for covering the fee:
/// - `DeductFromInput(i)`: Add to the ith input amount (more is claimed from that address to cover fee)
/// - `ReduceOutput(i)`: Reduce the ith output amount (less is sent to that address)
///
/// The strategy steps are applied in order until the fee is fully covered.
pub fn modify_inputs_and_outputs_for_fee_v0(
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
                // Get the input at the specified index and add to it
                if let Some((&address, &(nonce, amount))) = inputs.iter().nth(index as usize) {
                    let new_amount = amount + remaining_fee;
                    remaining_fee = 0;
                    inputs.insert(address, (nonce, new_amount));
                }
            }
            AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                // Get the output at the specified index and reduce it
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
