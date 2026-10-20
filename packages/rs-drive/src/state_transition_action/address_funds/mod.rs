/// Address credit withdrawal transition action
pub mod address_credit_withdrawal;
/// Address funding from asset lock transition action
pub mod address_funding_from_asset_lock;
/// Address funds transfer transition action
pub mod address_funds_transfer;

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use std::collections::BTreeMap;

/// Gives each input of a failed transition back the amount it would have spent.
///
/// `inputs_with_remaining_balance` holds each input's balance after its spend (the stored balance
/// minus the requested amount). A failed transition spends nothing, so each input keeps its whole
/// balance. Both maps come from the same inputs, so an input missing from either, or a sum above
/// the stored balance's range, is a code error, not a transition error.
pub fn restore_input_spends_for_failed_transition(
    inputs_with_remaining_balance: &mut BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    requested_inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
) -> Result<(), Error> {
    if inputs_with_remaining_balance.len() != requested_inputs.len() {
        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
            "a failed transition's remaining balances must cover exactly its inputs",
        )));
    }
    for (address, (_nonce, balance)) in inputs_with_remaining_balance.iter_mut() {
        let (_, requested_spend) = requested_inputs.get(address).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "a failed transition's remaining balances must cover exactly its inputs",
            ),
        ))?;
        *balance = balance.checked_add(*requested_spend).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "an input's remaining balance plus its spend exceeds the credit range",
            ),
        ))?;
    }
    Ok(())
}
