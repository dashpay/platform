mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::document_type::action_fees::ContractFeePot;
use crate::identity::KeyID;
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::ProtocolError;

/// Pays out one of the two fee pots a data contract's document action fees accumulate in.
///
/// The owner pot is claimed by the contract owner, who receives all of it. The moderators pot
/// is claimed by any member of the contract's moderation team and split equally between the
/// team. A pot is paid out at most once per epoch. Signed with a CRITICAL authentication key,
/// under the signer's contract-scoped nonce.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Encode, Decode, PlatformSignable, Debug, Clone, PartialEq, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContractFeeClaimTransitionV0 {
    /// The claimant: the identity that signs.
    pub owner_id: Identifier,

    /// The contract whose pot is paid out.
    pub data_contract_id: Identifier,

    /// The signer's nonce for this contract, to prevent replay attacks.
    pub identity_contract_nonce: IdentityNonce,

    /// The pot that is paid out.
    pub pot: ContractFeePot,

    /// The fee multiplier
    pub user_fee_increase: UserFeeIncrease,

    /// The ID of the public key used to sign the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    /// Cryptographic signature of the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for ContractFeeClaimTransitionV0 {
    fn default() -> Self {
        ContractFeeClaimTransitionV0 {
            owner_id: Identifier::default(),
            data_contract_id: Identifier::default(),
            identity_contract_nonce: 0,
            pot: ContractFeePot::Owner,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::Signable;
    use crate::state_transition::{
        StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
        StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned,
        StateTransitionType,
    };

    fn make_v0() -> ContractFeeClaimTransitionV0 {
        ContractFeeClaimTransitionV0 {
            owner_id: Identifier::random(),
            data_contract_id: Identifier::random(),
            identity_contract_nonce: 3,
            pot: ContractFeePot::Moderators,
            user_fee_increase: 1,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![9; 65]),
        }
    }

    #[test]
    fn should_exclude_the_signature_from_the_signable_bytes() {
        let a = make_v0();
        let mut b = a.clone();
        b.signature = BinaryData::new(vec![1; 65]);
        b.signature_public_key_id = 7;
        assert_eq!(
            a.signable_bytes().expect("signable"),
            b.signable_bytes().expect("signable")
        );
    }

    #[test]
    fn should_sign_the_pot() {
        // A signature for one pot must not be replayable as a claim of the other.
        let a = make_v0();
        let mut b = a.clone();
        b.pot = ContractFeePot::Owner;
        assert_ne!(
            a.signable_bytes().expect("signable"),
            b.signable_bytes().expect("signable")
        );
    }

    #[test]
    fn should_describe_itself() {
        let mut t = make_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::ContractFeeClaim
        );
        assert_eq!(t.modified_data_ids(), vec![t.data_contract_id]);
        assert_eq!(t.owner_id(), t.owner_id);
        assert_eq!(t.signature_public_key_id(), 2);
        t.set_signature_public_key_id(4);
        assert_eq!(t.signature_public_key_id(), 4);
        assert_eq!(t.user_fee_increase(), 1);
        t.set_user_fee_increase(9);
        assert_eq!(t.user_fee_increase(), 9);
        assert_eq!(t.signature().len(), 65);
        assert_eq!(
            t.security_level_requirement(crate::identity::Purpose::AUTHENTICATION),
            vec![crate::identity::SecurityLevel::CRITICAL]
        );
        let outer: StateTransition = t.into();
        assert!(matches!(outer, StateTransition::ContractFeeClaim(_)));
    }
}
