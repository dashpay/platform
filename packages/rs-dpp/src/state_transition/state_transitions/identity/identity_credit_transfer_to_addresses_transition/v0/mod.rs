mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::address_funds::PlatformAddress;
use crate::identity::KeyID;
use std::collections::BTreeMap;

use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};

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
pub struct IdentityCreditTransferToAddressesTransitionV0 {
    // Own ST fields
    pub identity_id: Identifier,
    pub recipient_addresses: BTreeMap<PlatformAddress, Credits>,
    pub nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {

    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
    use platform_value::Identifier;
    use rand::Rng;
    use std::fmt::Debug;

    fn test_identity_credit_transfer_to_addresses_transition<
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
    fn test_identity_credit_transfer_to_addresses_transition1() {
        use crate::address_funds::PlatformAddress;
        use std::collections::BTreeMap;

        let mut rng = rand::thread_rng();
        let mut recipient_addresses = BTreeMap::new();
        recipient_addresses.insert(PlatformAddress::P2pkh([1u8; 20]), rng.gen::<u64>());

        let transition = IdentityCreditTransferToAddressesTransitionV0 {
            identity_id: Identifier::random(),
            recipient_addresses,
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };

        test_identity_credit_transfer_to_addresses_transition(transition);
    }
}
