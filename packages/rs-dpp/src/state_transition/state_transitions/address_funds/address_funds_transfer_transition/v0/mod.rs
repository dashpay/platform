#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
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
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pub outputs: BTreeMap<PlatformAddress, Credits>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg(test)]
mod test {

    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
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
        use crate::address_funds::{AddressWitness, PlatformAddress};
        use std::collections::BTreeMap;

        // Create some inputs
        let mut inputs = BTreeMap::new();
        let input_address = PlatformAddress::P2pkh([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]);
        inputs.insert(input_address, (1, 1000)); // nonce: 1, credits: 1000

        // Create some outputs
        let mut outputs = BTreeMap::new();
        let output_address = PlatformAddress::P2pkh([
            20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
        ]);
        outputs.insert(output_address, 900); // credits: 900

        let transition = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            user_fee_increase: 0,
            fee_strategy: Default::default(),
            input_witnesses: vec![],
        };

        test_utxo_transfer_transition(transition);
    }
}
