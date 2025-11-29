use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::KeyOfTypeNonce;
use crate::ProtocolError;
use platform_value::Identifier;

pub trait StateTransitionAddressInputs: Sized {
    /// Get inputs
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>;
    /// Get inputs as a mutable map
    fn inputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>;
    /// Set inputs
    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>);
}

pub trait StateTransitionIdentityIdFromInputs: StateTransitionAddressInputs {
    /// Get the identity id from inputs
    fn identity_id_from_inputs(&self) -> Result<Identifier, ProtocolError> {
        if self.inputs().is_empty() {
            return Err(ProtocolError::ParsingError(
                "Identity creation requires at least one input".to_string(),
            ));
        }

        // Build a map containing only (PlatformAddress, KeyOfTypeNonce) pairs,
        // ignoring the Credits in the input values.
        let address_nonce_map: BTreeMap<&PlatformAddress, &KeyOfTypeNonce> = self
            .inputs()
            .iter()
            .map(|(address, (nonce, _credits))| (address, nonce))
            .collect();

        use crate::util::hash::hash_double;

        let input_bytes = bincode::encode_to_vec(&address_nonce_map, bincode::config::standard())
            .map_err(|e| ProtocolError::EncodingError(format!("Failed to encode inputs: {}", e)))?;

        let hash = hash_double(input_bytes);
        Ok(Identifier::new(hash))
    }
}
