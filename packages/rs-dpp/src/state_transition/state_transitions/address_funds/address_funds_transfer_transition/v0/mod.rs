#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::identity::{KeyID, KeyOfType};
use std::collections::BTreeMap;

use crate::prelude::{Identifier, IdentityNonce, KeyOfTypeNonce, UserFeeIncrease};

use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
#[derive(Default)]
pub struct AddressFundsTransferTransitionV0 {
    pub inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
    pub outputs: BTreeMap<KeyOfType, Credits>,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<BinaryData>,
}

#[cfg(test)]
mod test {

    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use platform_value::Identifier;
    use rand::Rng;
    use std::fmt::Debug;

    fn test_utxo_transfer_transition<
        T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq,
    >(
        transition: T,
    ) where
        <T as PlatformSerializable>::Error: std::fmt::Debug,
    {
        let serialized = T::serialize_to_bytes(&transition).expect("expected to serialize");
        let deserialized =
            T::deserialize_from_bytes(serialized.as_slice()).expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }

    #[test]
    fn test_utxo_transfer_transition1() {
        let mut rng = rand::thread_rng();
        let transition = AddressFundsTransferTransitionV0 {
            identity_id: Identifier::random(),
            recipient_keys: Identifier::random(),
            amount: rng.gen(),
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };

        test_utxo_transfer_transition(transition);
    }
}
