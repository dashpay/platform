//! Which of an identity's keys this wallet lets an identity update disable.
//!
//! Two refusals, both client policy that Platform does not enforce:
//!
//! - **DIP-11** — the last valid authentication key of a security level,
//!   judged by dpp's [`dpp_check_keys_can_be_disabled`].
//! - **Last enabled TRANSFER key able to sign** — wallet policy, not a DIP
//!   rule: without one the identity can no longer withdraw or transfer credits
//!   until a new transfer key is added.
//!
//! [`check_keys_can_be_disabled`] is the single place both are decided. The
//! identity-update entry points refuse with it before fetching a nonce, and
//! hosts ask it through `IdentityWallet::check_keys_can_be_disabled` to offer
//! only the keys it allows.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::key_disable_rule::{
    check_keys_can_be_disabled as dpp_check_keys_can_be_disabled, LastAuthenticationKeyError,
};
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};

/// Disabling `key_id` would leave the identity without an enabled key of
/// `purpose` able to sign. Wallet policy, not a DIP rule or consensus:
/// Platform accepts the update.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "cannot disable {purpose} key {key_id}: no other enabled {purpose} key able to sign \
     withdrawals and transfers would remain"
)]
pub struct LastEnabledKeyError {
    /// The key the update would disable.
    pub key_id: KeyID,
    /// The purpose that would be left without an enabled key able to sign.
    pub purpose: Purpose,
}

/// Why this wallet refuses an identity update's key disables.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyDisableRefusal {
    /// DIP-11: the last valid authentication key of a security level.
    #[error(transparent)]
    LastAuthenticationKey(#[from] LastAuthenticationKeyError),
    /// Wallet policy: the last enabled key of a purpose that must keep one.
    #[error(transparent)]
    LastEnabledKey(#[from] LastEnabledKeyError),
}

/// Purposes an identity must keep an enabled key of. TRANSFER only: it
/// authorizes credit withdrawals and transfers.
const PURPOSES_KEPT_ENABLED: [Purpose; 1] = [Purpose::TRANSFER];

/// Whether identity-signed validation lets `key` sign a credit withdrawal or
/// transfer, both of which require a CRITICAL key. `ECDSA_HASH160` keys, which
/// identities created by older Dash Evo Tool versions carry, can.
fn can_sign_withdrawals_and_transfers(key: &IdentityPublicKey) -> bool {
    key.security_level() == SecurityLevel::CRITICAL && key.key_type().can_sign_state_transitions()
}

/// Whether an identity update that adds `add_public_keys` and disables
/// `disable_public_keys` is one this wallet lets through: DIP-11 first (see
/// [`dpp_check_keys_can_be_disabled`] for what counts as a valid
/// authentication key), then the last enabled TRANSFER key able to sign. A
/// TRANSFER key that cannot sign neither counts as remaining nor is protected:
/// disabling it costs no ability. A key added in the same update counts as
/// remaining for both.
///
/// No clock, exactly like dpp's identity update builder: an authentication
/// key with any expiry does not count as remaining. So for the same keys, the
/// answer hosts are shown and the one the update gets are the same.
pub fn check_keys_can_be_disabled(
    identity: &Identity,
    add_public_keys: &[IdentityPublicKey],
    disable_public_keys: &[KeyID],
) -> Result<(), KeyDisableRefusal> {
    dpp_check_keys_can_be_disabled(identity, add_public_keys, disable_public_keys, None)?;

    for key_id in disable_public_keys {
        let Some(key) = identity.public_keys().get(key_id) else {
            continue;
        };
        let purpose = key.purpose();
        if !PURPOSES_KEPT_ENABLED.contains(&purpose)
            || key.disabled_at().is_some()
            || !can_sign_withdrawals_and_transfers(key)
        {
            continue;
        }
        let another_remains = identity
            .public_keys()
            .values()
            .filter(|remaining| !disable_public_keys.contains(&remaining.id()))
            .chain(add_public_keys)
            .any(|remaining| {
                remaining.purpose() == purpose
                    && remaining.disabled_at().is_none()
                    && can_sign_withdrawals_and_transfers(remaining)
            });
        if !another_remains {
            return Err(LastEnabledKeyError {
                key_id: *key_id,
                purpose,
            }
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{KeyType, SecurityLevel};
    use dpp::platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;

    fn key(id: KeyID, purpose: Purpose, security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8 + 2; 33]),
            disabled_at: None,
        })
    }

    /// MASTER 0, CRITICAL 1 authentication keys and TRANSFER 3.
    fn identity_with(extra: Vec<IdentityPublicKey>) -> Identity {
        let keys = [
            key(0, Purpose::AUTHENTICATION, SecurityLevel::MASTER),
            key(1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL),
            key(3, Purpose::TRANSFER, SecurityLevel::CRITICAL),
        ]
        .into_iter()
        .chain(extra);
        IdentityV0 {
            id: Identifier::from([1; 32]),
            public_keys: keys.map(|key| (key.id(), key)).collect::<BTreeMap<_, _>>(),
            balance: 0,
            revision: 0,
        }
        .into()
    }

    #[test]
    fn should_refuse_disabling_the_last_enabled_transfer_key() {
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![]), &[], &[3]),
            Err(KeyDisableRefusal::LastEnabledKey(LastEnabledKeyError {
                key_id: 3,
                purpose: Purpose::TRANSFER,
            }))
        );
    }

    #[test]
    fn should_allow_disabling_a_transfer_key_another_enabled_one_replaces() {
        let second = key(4, Purpose::TRANSFER, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![second.clone()]), &[], &[3]),
            Ok(())
        );
        // A transfer key added in the same update counts.
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![]), &[second], &[3]),
            Ok(())
        );
    }

    #[test]
    fn should_not_count_a_disabled_transfer_key_as_remaining() {
        let mut disabled = key(4, Purpose::TRANSFER, SecurityLevel::CRITICAL);
        disabled.set_disabled_at(1_700_000_000_000);
        assert!(matches!(
            check_keys_can_be_disabled(&identity_with(vec![disabled]), &[], &[3]),
            Err(KeyDisableRefusal::LastEnabledKey(_))
        ));
        // Disabling both transfer keys at once leaves none.
        let second = key(4, Purpose::TRANSFER, SecurityLevel::CRITICAL);
        assert!(matches!(
            check_keys_can_be_disabled(&identity_with(vec![second]), &[], &[3, 4]),
            Err(KeyDisableRefusal::LastEnabledKey(_))
        ));
    }

    fn transfer_key(id: KeyID, key_type: KeyType, level: SecurityLevel) -> IdentityPublicKey {
        let data = match key_type {
            KeyType::ECDSA_SECP256K1 => vec![id as u8 + 2; 33],
            KeyType::BLS12_381 => vec![id as u8 + 2; 48],
            _ => vec![id as u8 + 2; 20],
        };
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::TRANSFER,
            security_level: level,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(data),
            disabled_at: None,
        })
    }

    #[test]
    fn should_not_count_a_transfer_key_that_cannot_sign_as_remaining() {
        for key_type in [KeyType::BIP13_SCRIPT_HASH, KeyType::EDDSA_25519_HASH160] {
            let unusable = transfer_key(4, key_type, SecurityLevel::CRITICAL);
            let refused = Err(KeyDisableRefusal::LastEnabledKey(LastEnabledKeyError {
                key_id: 3,
                purpose: Purpose::TRANSFER,
            }));
            assert_eq!(
                check_keys_can_be_disabled(&identity_with(vec![unusable.clone()]), &[], &[3]),
                refused,
                "{key_type:?} held by the identity"
            );
            assert_eq!(
                check_keys_can_be_disabled(&identity_with(vec![]), &[unusable], &[3]),
                refused,
                "{key_type:?} added in the same update"
            );
        }
    }

    #[test]
    fn should_not_count_a_transfer_key_below_critical_as_remaining() {
        let high = transfer_key(4, KeyType::ECDSA_SECP256K1, SecurityLevel::HIGH);
        assert!(matches!(
            check_keys_can_be_disabled(&identity_with(vec![high]), &[], &[3]),
            Err(KeyDisableRefusal::LastEnabledKey(_))
        ));
    }

    #[test]
    fn should_count_a_hash160_transfer_key_as_remaining() {
        let hash160 = transfer_key(4, KeyType::ECDSA_HASH160, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![hash160.clone()]), &[], &[3]),
            Ok(())
        );
        // And the last one is protected like any other that can sign.
        let only_hash160 = transfer_key(3, KeyType::ECDSA_HASH160, SecurityLevel::CRITICAL);
        assert!(matches!(
            check_keys_can_be_disabled(&identity_with(vec![only_hash160]), &[], &[3]),
            Err(KeyDisableRefusal::LastEnabledKey(_))
        ));
    }

    #[test]
    fn should_allow_disabling_a_transfer_key_that_cannot_sign() {
        // Key id 3 replaces the signing TRANSFER key of `identity_with`, so this
        // is the identity's only TRANSFER key and disabling it costs no ability.
        let only_unusable = transfer_key(3, KeyType::BIP13_SCRIPT_HASH, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![only_unusable]), &[], &[3]),
            Ok(())
        );
    }

    #[test]
    fn should_say_the_remaining_transfer_key_must_be_able_to_sign() {
        let unusable = transfer_key(4, KeyType::BIP13_SCRIPT_HASH, SecurityLevel::CRITICAL);
        let refusal = check_keys_can_be_disabled(&identity_with(vec![unusable]), &[], &[3])
            .expect_err("an unusable key does not replace the last one that can sign");
        assert_eq!(
            refusal.to_string(),
            "cannot disable TRANSFER key 3: no other enabled TRANSFER key able to sign \
             withdrawals and transfers would remain"
        );
    }

    #[test]
    fn should_report_the_dip11_refusal_for_authentication_keys() {
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![]), &[], &[1]),
            Err(KeyDisableRefusal::LastAuthenticationKey(
                LastAuthenticationKeyError {
                    key_id: 1,
                    security_level: SecurityLevel::CRITICAL,
                }
            ))
        );
    }

    #[test]
    fn should_allow_disabling_keys_of_other_purposes() {
        let encryption = key(5, Purpose::ENCRYPTION, SecurityLevel::MEDIUM);
        assert_eq!(
            check_keys_can_be_disabled(&identity_with(vec![encryption]), &[], &[5]),
            Ok(())
        );
    }
}
