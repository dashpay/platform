#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::collections::BTreeMap;
use std::convert::TryFrom;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::address_funds::AddressWitness;
use crate::fee::Credits;
use crate::identity::KeyOfType;
use crate::prelude::{Identifier, KeyOfTypeNonce, UserFeeIncrease};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::ProtocolError;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "IdentityCreateFromAddressesTransitionV0Inner")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside "bincode_derive"
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityCreateFromAddressesTransitionV0 {
    // When signing, we don't sign the signatures for keys
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    pub inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Deserialize),
    serde(rename_all = "camelCase")
)]
struct IdentityCreateFromAddressesTransitionV0Inner {
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
    user_fee_increase: UserFeeIncrease,
    input_witnesses: Vec<AddressWitness>,
}

impl TryFrom<IdentityCreateFromAddressesTransitionV0Inner>
    for IdentityCreateFromAddressesTransitionV0
{
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateFromAddressesTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateFromAddressesTransitionV0Inner {
            public_keys,
            inputs,
            user_fee_increase,
            input_witnesses,
        } = value;

        Ok(Self {
            public_keys,
            inputs,
            user_fee_increase,
            input_witnesses,
        })
    }
}
