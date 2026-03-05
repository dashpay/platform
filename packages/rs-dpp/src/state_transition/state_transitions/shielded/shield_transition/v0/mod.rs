mod state_transition_like;
mod state_transition_validation;
mod types;
#[cfg(feature = "state-transition-signing")]
pub(super) mod v0_methods;
mod version;

use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::shielded::SerializedAction;
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
pub struct ShieldTransitionV0 {
    /// Address inputs funding the shield (address -> nonce + max contribution).
    /// The total across all inputs must cover |value_balance| + fees.
    /// Excess credits remain in the source addresses.
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Amount of credits being shielded (entering the shielded pool).
    pub amount: u64,
    /// Sinsemilla root of the note commitment tree (Orchard Anchor)
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(with = "crate::shielded::serde_bytes_64")
    )]
    pub binding_signature: [u8; 64],
    /// Fee payment strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Address witness signatures (excluded from sig hash)
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use std::fmt::Debug;

    fn test_round_trip<T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq>(
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
    fn test_shield_transition_v0_serialization_round_trip() {
        let mut inputs = BTreeMap::new();
        let address = PlatformAddress::P2pkh([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]);
        inputs.insert(address, (1u32, 1000u64)); // nonce, credits

        let transition = ShieldTransitionV0 {
            inputs,
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 692],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            amount: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0u16,
            input_witnesses: vec![],
        };

        test_round_trip(transition);
    }
}
