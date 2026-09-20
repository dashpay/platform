mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::identity::{KeyID, TimestampMillis};
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::ProtocolError;
use std::fmt;

/// What the moderator does to one identity on the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$type", rename_all = "camelCase")
)]
pub enum ContractUserModerationAction {
    /// Puts the identity on the banlist. Removes a suspension it may carry.
    Ban {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
    },
    /// Takes the identity off the banlist.
    Unban {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
    },
    /// Puts the identity on the suspension list until `until` (block time, milliseconds),
    /// replacing a suspension it already carries.
    Suspend {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
        until: TimestampMillis,
    },
    /// Takes the identity off the suspension list, lapsed or not.
    Unsuspend {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
    },
}

impl Default for ContractUserModerationAction {
    fn default() -> Self {
        ContractUserModerationAction::Ban {
            identity_id: Identifier::default(),
        }
    }
}

impl ContractUserModerationAction {
    /// The identity the action targets.
    pub fn identity_id(&self) -> Identifier {
        match self {
            ContractUserModerationAction::Ban { identity_id }
            | ContractUserModerationAction::Unban { identity_id }
            | ContractUserModerationAction::Suspend { identity_id, .. }
            | ContractUserModerationAction::Unsuspend { identity_id } => *identity_id,
        }
    }

    /// The suspension end for a suspend, `None` otherwise.
    pub fn until(&self) -> Option<TimestampMillis> {
        match self {
            ContractUserModerationAction::Suspend { until, .. } => Some(*until),
            _ => None,
        }
    }

    /// The action's name on the wire.
    pub fn name(&self) -> &'static str {
        match self {
            ContractUserModerationAction::Ban { .. } => "ban",
            ContractUserModerationAction::Unban { .. } => "unban",
            ContractUserModerationAction::Suspend { .. } => "suspend",
            ContractUserModerationAction::Unsuspend { .. } => "unsuspend",
        }
    }
}

impl fmt::Display for ContractUserModerationAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractUserModerationAction::Suspend { identity_id, until } => {
                write!(f, "suspend {} until {}", identity_id, until)
            }
            other => write!(f, "{} {}", other.name(), other.identity_id()),
        }
    }
}

// `until` is a u64, but basic structure validation refuses one past
// `SystemLimits::max_contract_suspension_until` (2^53 - 1), so it is exact in JSON.
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractUserModerationAction {}

/// Edits the banlist or the suspension list of a moderated data contract. Signed by the
/// contract owner or a moderator named in the contract's config, with a CRITICAL
/// authentication key, under the signer's contract-scoped nonce.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Encode, Decode, PlatformSignable, Debug, Clone, PartialEq, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct ContractUserModerationTransitionV0 {
    /// The moderator: the identity that signs.
    pub owner_id: Identifier,

    /// The moderated contract.
    pub data_contract_id: Identifier,

    /// The signer's nonce for this contract, to prevent replay attacks.
    pub identity_contract_nonce: IdentityNonce,

    /// What is done, to whom.
    pub action: ContractUserModerationAction,

    /// The fee multiplier
    pub user_fee_increase: UserFeeIncrease,

    /// The ID of the public key used to sign the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    /// Cryptographic signature of the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
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

    fn make_v0() -> ContractUserModerationTransitionV0 {
        ContractUserModerationTransitionV0 {
            owner_id: Identifier::random(),
            data_contract_id: Identifier::random(),
            identity_contract_nonce: 3,
            action: ContractUserModerationAction::Ban {
                identity_id: Identifier::random(),
            },
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
    fn should_describe_itself() {
        let mut t = make_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::ContractUserModeration
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
        assert!(matches!(outer, StateTransition::ContractUserModeration(_)));
    }

    #[test]
    fn should_name_the_action_target() {
        let target = Identifier::random();
        let action = ContractUserModerationAction::Suspend {
            identity_id: target,
            until: 12,
        };
        assert_eq!(action.identity_id(), target);
        assert_eq!(action.until(), Some(12));
        assert_eq!(action.name(), "suspend");
        assert_eq!(
            ContractUserModerationAction::Unban {
                identity_id: target
            }
            .until(),
            None
        );
    }
}
