use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::state_transition::StateTransitionWitnessSigned;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use platform_value::Identifier;

pub trait StateTransitionIdentityIdFromInputs: StateTransitionWitnessSigned {
    /// Get the identity id from inputs.
    ///
    /// Inputs should represent state after creation of the identity (eg. be incremented by 1).
    fn identity_id_from_inputs(&self) -> Result<Identifier, ProtocolError> {
        let inputs = self.inputs();
        identity_id_from_input_addresses(inputs)
    }
}

/// Helper that computes the identity ID from input addresses and nonces.
/// Nonces should represent state after creation of the identity (eg. be incremented by 1).
///
/// Internal use only; see `StateTransitionIdentityIdFromInputs` trait.
pub(crate) fn identity_id_from_input_addresses(
    input_addresses: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
) -> Result<Identifier, ProtocolError> {
    if input_addresses.is_empty() {
        return Err(ProtocolError::ParsingError(
            "Identity creation requires at least one input".to_string(),
        ));
    }
    // Build a map containing only (PlatformAddress, KeyOfTypeNonce) pairs,
    // ignoring the Credits in the input values.
    let address_nonce_map: BTreeMap<&PlatformAddress, &AddressNonce> = input_addresses
        .iter()
        .map(|(address, (nonce, _credits))| (address, nonce))
        .collect();
    let input_bytes = bincode::encode_to_vec(&address_nonce_map, bincode::config::standard())
        .map_err(|e| ProtocolError::EncodingError(format!("Failed to encode inputs: {}", e)))?;

    let hash = hash_double(input_bytes);
    Ok(Identifier::new(hash))
}
