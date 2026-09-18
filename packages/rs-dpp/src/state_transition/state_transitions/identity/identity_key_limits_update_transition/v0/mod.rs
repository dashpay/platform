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

use crate::fee::Credits;
use crate::identity::{KeyID, TimestampMillis};
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::ProtocolError;

/// Raises the limits of one authentication key of the identity. Both limit fields carry the new
/// absolute value, so what the transition claims is exactly what a proof of its execution shows.
/// The identity's revision is not claimed and not bumped: the transition names an existing key
/// and allocates nothing, so a stale view of the identity cannot make it collide.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Encode, Decode, PlatformSignable, Debug, Clone, PartialEq, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityKeyLimitsUpdateTransitionV0 {
    /// Unique identifier of the identity whose key is updated
    pub identity_id: Identifier,

    /// Identity nonce for this transition to prevent replay attacks
    pub nonce: IdentityNonce,

    /// The key whose limits are raised
    pub key_id: KeyID,

    /// The new total budget of the key: greater than the current one, `None` to leave it as it is.
    /// The remaining budget grows by the same amount.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub total_budget: Option<Credits>,

    /// The new expiry of the key, in block time milliseconds: later than the current one, `None`
    /// to leave it as it is.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub expires_at: Option<TimestampMillis>,

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

    fn make_update_v0() -> IdentityKeyLimitsUpdateTransitionV0 {
        IdentityKeyLimitsUpdateTransitionV0 {
            identity_id: Identifier::random(),
            nonce: 5,
            key_id: 3,
            total_budget: Some(200_000_000),
            expires_at: Some(1_800_000_000_000),
            user_fee_increase: 3,
            signature_public_key_id: 0,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn should_default_to_no_change() {
        let t = IdentityKeyLimitsUpdateTransitionV0::default();
        assert_eq!(t.nonce, 0);
        assert_eq!(t.key_id, 0);
        assert_eq!(t.total_budget, None);
        assert_eq!(t.expires_at, None);
    }

    #[test]
    fn should_report_its_type_and_owner() {
        let t = make_update_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityKeyLimitsUpdate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
        assert_eq!(t.owner_id(), t.identity_id);
        assert_eq!(t.unique_identifiers().len(), 1);
    }

    #[test]
    fn should_accept_master_and_critical_signing_keys() {
        use crate::identity::{Purpose, SecurityLevel};
        let mut t = make_update_v0();
        assert_eq!(t.signature_public_key_id(), 0);
        t.set_signature_public_key_id(42);
        assert_eq!(t.signature_public_key_id(), 42);
        assert_eq!(
            t.security_level_requirement(Purpose::AUTHENTICATION),
            vec![SecurityLevel::MASTER, SecurityLevel::CRITICAL]
        );
    }

    #[test]
    fn should_expose_the_user_fee_increase_and_signature() {
        let mut t = make_update_v0();
        assert_eq!(t.user_fee_increase(), 3);
        t.set_user_fee_increase(10);
        assert_eq!(t.user_fee_increase(), 10);
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn should_convert_into_the_state_transition_enum() {
        let t = make_update_v0();
        let st: StateTransition = t.into();
        assert!(matches!(st, StateTransition::IdentityKeyLimitsUpdate(_)));
    }

    /// Every field but the signature and its key id must change the signable bytes: a relay must
    /// not be able to alter what the identity approved.
    #[test]
    fn should_sign_over_every_limit_field() {
        let base = make_update_v0();
        let base_bytes = base.signable_bytes().expect("signable bytes");

        let variants: Vec<(&str, IdentityKeyLimitsUpdateTransitionV0)> = vec![
            ("identity_id", {
                let mut t = base.clone();
                t.identity_id = Identifier::random();
                t
            }),
            ("nonce", {
                let mut t = base.clone();
                t.nonce += 1;
                t
            }),
            ("key_id", {
                let mut t = base.clone();
                t.key_id += 1;
                t
            }),
            ("total_budget", {
                let mut t = base.clone();
                t.total_budget = Some(200_000_001);
                t
            }),
            ("total_budget unset", {
                let mut t = base.clone();
                t.total_budget = None;
                t
            }),
            ("expires_at", {
                let mut t = base.clone();
                t.expires_at = Some(1_800_000_000_001);
                t
            }),
            ("expires_at unset", {
                let mut t = base.clone();
                t.expires_at = None;
                t
            }),
            ("user_fee_increase", {
                let mut t = base.clone();
                t.user_fee_increase += 1;
                t
            }),
        ];
        for (field, variant) in variants {
            assert_ne!(
                variant.signable_bytes().expect("signable bytes"),
                base_bytes,
                "{field} must be covered by the signature"
            );
        }

        let mut signed = base.clone();
        signed.signature_public_key_id = 9;
        signed.signature = BinaryData::new(vec![7; 65]);
        assert_eq!(
            signed.signable_bytes().expect("signable bytes"),
            base_bytes,
            "the signature and its key id are excluded from the signable bytes"
        );
    }
}
