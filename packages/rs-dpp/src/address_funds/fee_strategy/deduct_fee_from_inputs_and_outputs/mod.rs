use crate::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::v0::modify_inputs_and_outputs_for_fee_v0;
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;

pub fn modify_inputs_and_outputs_for_fee(
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    outputs: BTreeMap<PlatformAddress, Credits>,
    fee_strategy: AddressFundsFeeStrategy,
    fee: Credits,
    platform_version: &PlatformVersion,
) -> Result<
    (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    ),
    ProtocolError,
> {
    match platform_version
        .dpp
        .methods
        .modify_inputs_and_outputs_for_fee
    {
        0 => modify_inputs_and_outputs_for_fee_v0(inputs, outputs, fee_strategy, fee),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "modify_inputs_and_outputs_for_fee".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
