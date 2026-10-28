//! The key set a new identity is created with.
//!
//! [`IdentityCreationKeyLayout`] is the single source of the key layout
//! this crate's hosts (the FFI, the JNI bridge, and the Swift and Kotlin
//! SDKs) register a new identity with. Hosts derive one key per slot and
//! stamp the slot's role on it; they never decide the layout themselves.
//!
//! [`IdentityCreationKeys`] is the write-path pre-flight. Every
//! identity-creating entry point of this crate takes it instead of a raw
//! key map, so a key set that breaks DIP-11 ("identities must be created
//! with at least one authentication key in each security level") is refused
//! before anything is signed or broadcast. Consensus does not enforce that
//! rule, so this client check does not duplicate a `rs-dpp` validation.
//!
//! # The one exemption
//!
//! Compatibility beats compliance in exactly one case: resuming a tracked
//! registration asset lock
//! (`FromExistingAssetLock { consume_invitation_voucher: false, .. }`) whose
//! amount cannot pay for the full DIP-11 key set at the current fee
//! schedule. Such a lock was funded for the four keys every registration
//! used before the MEDIUM key existed, so it resumes with exactly those
//! ([`IdentityCreationKeyLayout::AssetLockResume`]) and the pre-DIP-11 check.
//! Rust decides this from the tracked lock's persisted amount
//! ([`IdentityWallet::resume_key_layout`],
//! [`IdentityWallet::creation_keys_for_funding`]); hosts never compare
//! amounts. Every other creation, including a lock built first and
//! registered later with enough funds, gets the DIP-11 check.

use std::collections::BTreeMap;

use dashcore::OutPoint;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::data_contracts::SystemDataContract;
use dpp::fee::Credits;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::StateTransitionEstimatedFeeValidation;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

use super::IdentityWallet;
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::AssetLockFunding;

/// DashPay document type the ENCRYPTION / DECRYPTION keys are bound to. It is
/// the only DashPay document type that requires an identity-encryption bound
/// key.
const DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE: &str = "contactRequest";

/// Authentication security levels DIP-11 requires a new identity to hold a
/// key AT (not merely at or above), in the order the error reports them.
const REQUIRED_AUTHENTICATION_LEVELS: [SecurityLevel; 4] = [
    SecurityLevel::MASTER,
    SecurityLevel::CRITICAL,
    SecurityLevel::HIGH,
    SecurityLevel::MEDIUM,
];

/// Which key layout an identity-creation flow registers.
///
/// Every slot is an `ECDSA_SECP256K1` key whose DIP-9 key index equals its
/// key id (`m/9'/coin'/5'/0'/0'/identity_index'/key_id'`). Discovery and the
/// derivation breadcrumbs rely on that equality. Key ids 0-5 keep the roles
/// they had before the AUTHENTICATION/MEDIUM key was added; that key takes
/// the next free id.
///
/// | key id | purpose        | level    | layouts                      |
/// |--------|----------------|----------|------------------------------|
/// | 0      | AUTHENTICATION | MASTER   | all                          |
/// | 1      | AUTHENTICATION | CRITICAL | all                          |
/// | 2      | AUTHENTICATION | HIGH     | all                          |
/// | 3      | TRANSFER       | CRITICAL | all                          |
/// | 4      | ENCRYPTION     | MEDIUM   | `NewIdentityWithDashPayKeys` |
/// | 5      | DECRYPTION     | MEDIUM   | `NewIdentityWithDashPayKeys` |
/// | 4 / 6  | AUTHENTICATION | MEDIUM   | `NewIdentity` / `…WithDashPayKeys` |
///
/// The ENCRYPTION / DECRYPTION keys are bound to DashPay's `contactRequest`
/// document type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentityCreationKeyLayout {
    /// A new identity without DashPay keys.
    NewIdentity,
    /// A new identity that can send and receive DashPay contact requests.
    NewIdentityWithDashPayKeys,
    /// The four keys every resume was re-derived with before the
    /// AUTHENTICATION/MEDIUM key existed (key ids 0-3). Used only to resume
    /// a tracked registration lock whose amount cannot pay for the DIP-11
    /// key set, as decided by [`IdentityWallet::resume_key_layout`]: such a
    /// lock must not grow a key, or a fee, it was not funded for.
    AssetLockResume,
}

/// One key of an [`IdentityCreationKeyLayout`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCreationKeySlot {
    /// The key id the key is registered under.
    pub key_id: KeyID,
    /// The key type.
    pub key_type: KeyType,
    /// The key purpose.
    pub purpose: Purpose,
    /// The key security level.
    pub security_level: SecurityLevel,
    /// The contract bounds the key is registered with, if any.
    pub contract_bounds: Option<ContractBounds>,
}

impl IdentityCreationKeySlot {
    /// The DIP-9 key index (`m/9'/coin'/5'/0'/0'/identity_index'/key_index'`)
    /// the slot's key is derived at. Always the key id.
    pub fn derivation_key_index(&self) -> u32 {
        self.key_id
    }

    /// The identity public key this slot registers, carrying the derived
    /// public key `data`.
    pub fn public_key(&self, data: BinaryData) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: self.key_id,
            purpose: self.purpose,
            security_level: self.security_level,
            contract_bounds: self.contract_bounds.clone(),
            key_type: self.key_type,
            read_only: false,
            data,
            disabled_at: None,
        })
    }
}

impl IdentityCreationKeyLayout {
    /// The layout's slots in key-id order. Key ids are contiguous from 0.
    pub fn slots(self) -> Vec<IdentityCreationKeySlot> {
        let slot = |key_id, purpose, security_level, contract_bounds| IdentityCreationKeySlot {
            key_id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose,
            security_level,
            contract_bounds,
        };
        let mut slots = vec![
            // Signs IdentityCreate / IdentityUpdate.
            slot(0, Purpose::AUTHENTICATION, SecurityLevel::MASTER, None),
            // Signs token transitions, which require CRITICAL.
            slot(1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, None),
            // Signs general document operations.
            slot(2, Purpose::AUTHENTICATION, SecurityLevel::HIGH, None),
            // Signs credit transfers and withdrawals.
            slot(3, Purpose::TRANSFER, SecurityLevel::CRITICAL, None),
        ];
        if self == Self::NewIdentityWithDashPayKeys {
            let bounds = ContractBounds::SingleContractDocumentType {
                id: SystemDataContract::Dashpay.id(),
                document_type_name: DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE.to_string(),
            };
            slots.push(slot(
                4,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
                Some(bounds.clone()),
            ));
            slots.push(slot(
                5,
                Purpose::DECRYPTION,
                SecurityLevel::MEDIUM,
                Some(bounds),
            ));
        }
        if self != Self::AssetLockResume {
            // DIP-11: one authentication key at every security level. Appended
            // at the next free id so ids 0-5 keep their roles.
            let next_free = slots.len() as KeyID;
            slots.push(slot(
                next_free,
                Purpose::AUTHENTICATION,
                SecurityLevel::MEDIUM,
                None,
            ));
        }
        slots
    }

    /// Number of keys the layout registers, for sizing the creation fee.
    pub fn key_count(self) -> usize {
        self.slots().len()
    }
}

/// Why a key set cannot create an identity.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityCreationKeysError {
    /// The key set is empty.
    #[error("an identity must be created with at least one public key")]
    Empty,
    /// A key is stored under a map entry other than its own key id.
    #[error("key map entry {map_key} holds a key with id {key_id}")]
    KeyIdMismatch {
        /// The map entry.
        map_key: KeyID,
        /// The id the key itself carries.
        key_id: KeyID,
    },
    /// Key id 0 is missing or is not an AUTHENTICATION key at MASTER level.
    /// It signs the identity-creation transition.
    #[error("key id 0 must be an AUTHENTICATION key at MASTER security level")]
    KeyZeroNotMasterAuthentication,
    /// DIP-11: the key set has no enabled AUTHENTICATION key at this
    /// security level.
    #[error(
        "a new identity must have an enabled AUTHENTICATION key at {security_level:?} \
         security level (DIP-11)"
    )]
    MissingAuthenticationKey {
        /// The security level without an authentication key.
        security_level: SecurityLevel,
    },
    /// A key set validated with the pre-DIP-11 check was used for a creation
    /// that is not exempt: only resuming a tracked registration lock that
    /// cannot fund the DIP-11 key set may skip the MEDIUM key.
    #[error(
        "keys without the DIP-11 check are only accepted to resume a tracked registration \
         asset lock that cannot fund the full DIP-11 key set"
    )]
    LegacyKeysNotExempt,
}

/// Which pre-flight a key set passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreationKeysCheck {
    /// The DIP-11 check for a new creation.
    NewIdentity,
    /// The check a resumed registration was started under.
    AssetLockResume,
}

/// A key set that passed the pre-flight for creating an identity.
///
/// Build it with [`Self::new_identity`] (address-funded, shielded and
/// invitation creations) or [`IdentityWallet::creation_keys_for_funding`]
/// (asset-lock funded registrations). Levels and structure are checked; key types are not, so
/// a caller-built set (for example one with `ECDSA_HASH160` keys) passes as
/// long as it has the required authentication levels.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityCreationKeys {
    keys: BTreeMap<KeyID, IdentityPublicKey>,
    check: CreationKeysCheck,
}

impl IdentityCreationKeys {
    /// Validate `keys` for a new identity: non-empty, each key under its own
    /// id, key id 0 AUTHENTICATION/MASTER, and an enabled AUTHENTICATION key
    /// at each of MASTER, CRITICAL, HIGH and MEDIUM. Security levels are
    /// minimums for signing, but DIP-11 needs one key AT each level here.
    pub fn new_identity(
        keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Self, IdentityCreationKeysError> {
        check_key_zero(&keys)?;
        if let Some((map_key, key)) = keys.iter().find(|(map_key, key)| **map_key != key.id()) {
            return Err(IdentityCreationKeysError::KeyIdMismatch {
                map_key: *map_key,
                key_id: key.id(),
            });
        }
        for security_level in REQUIRED_AUTHENTICATION_LEVELS {
            let present = keys.values().any(|key| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == security_level
                    && key.disabled_at().is_none()
            });
            if !present {
                return Err(IdentityCreationKeysError::MissingAuthenticationKey { security_level });
            }
        }
        Ok(Self {
            keys,
            check: CreationKeysCheck::NewIdentity,
        })
    }

    /// The pre-DIP-11 check alone (non-empty, key id 0
    /// AUTHENTICATION/MASTER), for the exempt resume only. Crate-private:
    /// callers reach it through [`IdentityWallet::creation_keys_for_funding`],
    /// which decides the exemption from the tracked lock.
    fn legacy_resume(
        keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Self, IdentityCreationKeysError> {
        check_key_zero(&keys)?;
        Ok(Self {
            keys,
            check: CreationKeysCheck::AssetLockResume,
        })
    }

    /// Refuse a resume-grade key set on a path that only creates new
    /// identities.
    pub(crate) fn ensure_new_identity(&self) -> Result<(), IdentityCreationKeysError> {
        match self.check {
            CreationKeysCheck::NewIdentity => Ok(()),
            CreationKeysCheck::AssetLockResume => {
                Err(IdentityCreationKeysError::LegacyKeysNotExempt)
            }
        }
    }

    /// The validated keys.
    pub fn keys(&self) -> &BTreeMap<KeyID, IdentityPublicKey> {
        &self.keys
    }

    /// The validated keys, by value.
    pub fn into_keys(self) -> BTreeMap<KeyID, IdentityPublicKey> {
        self.keys
    }
}

/// The tracked outpoint `funding` resumes, when it resumes a tracked
/// registration lock rather than funding a new registration or consuming an
/// invitation voucher.
// TODO: record the creation layout on the tracked asset lock so the
// exemption follows provenance rather than amount: an interrupted
// registration started after DIP-11 enforcement and funded at the exact
// 4-key minimum is grandfathered today too. Needs a field in every
// persister (SQLite, SwiftData, Room).
fn resumed_registration_out_point(funding: &AssetLockFunding) -> Option<&OutPoint> {
    match funding {
        AssetLockFunding::FromExistingAssetLock {
            out_point,
            consume_invitation_voucher: false,
        } => Some(out_point),
        _ => None,
    }
}

/// The minimum credits an identity-create transition with the DIP-11
/// [`IdentityCreationKeyLayout::NewIdentity`] key set needs, from the
/// versioned `rs-dpp` fee computation (no fee number lives here).
fn dip11_key_set_min_fee(platform_version: &PlatformVersion) -> Result<Credits, ProtocolError> {
    let public_keys = IdentityCreationKeyLayout::NewIdentity
        .slots()
        .iter()
        .map(|slot| {
            IdentityPublicKeyInCreation::from(&slot.public_key(BinaryData::new(vec![0; 33])))
        })
        .collect();
    IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
        public_keys,
        ..Default::default()
    })
    .calculate_min_required_fee(platform_version)
}

/// Whether a tracked lock of `amount_duffs` cannot pay for the DIP-11 key
/// set, and so resumes with the pre-DIP-11 keys. Returns the required
/// credits alongside the verdict for logging.
fn underfunded_for_dip11(
    amount_duffs: u64,
    platform_version: &PlatformVersion,
) -> Result<(bool, Credits), ProtocolError> {
    let required = dip11_key_set_min_fee(platform_version)?;
    Ok((
        amount_duffs.saturating_mul(CREDITS_PER_DUFF) < required,
        required,
    ))
}

impl IdentityWallet {
    /// Whether resuming the tracked lock at `out_point` is exempt from
    /// DIP-11, with the lock amount and the DIP-11 requirement.
    async fn resume_exemption(
        &self,
        out_point: &OutPoint,
    ) -> Result<(bool, u64, Credits), PlatformWalletError> {
        let amount_duffs = self
            .asset_locks
            .tracked_amount_duffs(out_point)
            .await
            .ok_or(PlatformWalletError::AssetLockNotTracked(*out_point))?;
        let (exempt, required) = underfunded_for_dip11(amount_duffs, self.sdk.version())
            .map_err(|e| PlatformWalletError::Sdk(dash_sdk::Error::Protocol(e)))?;
        Ok((exempt, amount_duffs, required))
    }

    /// The key layout a host must derive to resume the tracked registration
    /// lock at `out_point`: [`IdentityCreationKeyLayout::AssetLockResume`]
    /// when the lock's amount cannot pay for the DIP-11 key set (a lock
    /// funded for the four pre-DIP-11 keys), else
    /// [`IdentityCreationKeyLayout::NewIdentity`].
    ///
    /// # Errors
    /// [`PlatformWalletError::AssetLockNotTracked`] when no lock is tracked
    /// at `out_point`.
    pub async fn resume_key_layout(
        &self,
        out_point: &OutPoint,
    ) -> Result<IdentityCreationKeyLayout, PlatformWalletError> {
        let (exempt, _, _) = self.resume_exemption(out_point).await?;
        Ok(if exempt {
            IdentityCreationKeyLayout::AssetLockResume
        } else {
            IdentityCreationKeyLayout::NewIdentity
        })
    }

    /// Validate `keys` for an asset-lock funded registration.
    ///
    /// Gives the pre-DIP-11 check alone only to an exempt resume: `funding`
    /// resumes a tracked registration lock
    /// (`FromExistingAssetLock { consume_invitation_voucher: false, .. }`)
    /// whose persisted amount cannot pay for the DIP-11 key set. That case
    /// is logged at `info`. Every other funding, including an
    /// invitation-voucher reclaim and a lock built first and funded for the
    /// full set, gets [`IdentityCreationKeys::new_identity`].
    ///
    /// # Errors
    /// [`PlatformWalletError::IdentityCreationKeys`] when the keys fail the
    /// check that applies; [`PlatformWalletError::AssetLockNotTracked`] when
    /// a resume names an untracked lock.
    pub async fn creation_keys_for_funding(
        &self,
        keys: BTreeMap<KeyID, IdentityPublicKey>,
        funding: &AssetLockFunding,
    ) -> Result<IdentityCreationKeys, PlatformWalletError> {
        let Some(out_point) = resumed_registration_out_point(funding) else {
            return Ok(IdentityCreationKeys::new_identity(keys)?);
        };
        let (exempt, amount_duffs, required_credits) = self.resume_exemption(out_point).await?;
        if !exempt {
            return Ok(IdentityCreationKeys::new_identity(keys)?);
        }
        tracing::info!(
            outpoint = %out_point,
            amount_duffs,
            required_credits,
            "resuming a tracked identity registration lock that cannot fund the DIP-11 key \
             set; applying the pre-DIP-11 key check (no MEDIUM authentication key required)"
        );
        Ok(IdentityCreationKeys::legacy_resume(keys)?)
    }

    /// Refuse pre-DIP-11 keys unless `funding` is an exempt resume.
    pub(crate) async fn ensure_creation_keys_permitted(
        &self,
        keys: &IdentityCreationKeys,
        funding: &AssetLockFunding,
    ) -> Result<(), PlatformWalletError> {
        if keys.check == CreationKeysCheck::NewIdentity {
            return Ok(());
        }
        let exempt = match resumed_registration_out_point(funding) {
            Some(out_point) => self.resume_exemption(out_point).await?.0,
            None => false,
        };
        if exempt {
            Ok(())
        } else {
            Err(IdentityCreationKeysError::LegacyKeysNotExempt.into())
        }
    }
}

/// The pre-DIP-11 registration check: non-empty, and key id 0 is an
/// AUTHENTICATION key at MASTER level.
fn check_key_zero(
    keys: &BTreeMap<KeyID, IdentityPublicKey>,
) -> Result<(), IdentityCreationKeysError> {
    if keys.is_empty() {
        return Err(IdentityCreationKeysError::Empty);
    }
    match keys.get(&0) {
        Some(key)
            if key.purpose() == Purpose::AUTHENTICATION
                && key.security_level() == SecurityLevel::MASTER =>
        {
            Ok(())
        }
        _ => Err(IdentityCreationKeysError::KeyZeroNotMasterAuthentication),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;

    fn key(
        id: KeyID,
        key_type: KeyType,
        purpose: Purpose,
        level: SecurityLevel,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: level,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at: None,
        })
    }

    fn keys_from_layout(layout: IdentityCreationKeyLayout) -> BTreeMap<KeyID, IdentityPublicKey> {
        layout
            .slots()
            .iter()
            .map(|slot| {
                let data = BinaryData::new(vec![slot.key_id as u8; 33]);
                (slot.key_id, slot.public_key(data))
            })
            .collect()
    }

    /// The key set Dash Evo Tool registers today: an ECDSA_HASH160 master,
    /// ECDSA_HASH160 CRITICAL / HIGH / TRANSFER keys, the DashPay pair, and no
    /// AUTHENTICATION/MEDIUM key.
    fn det_style_keys() -> BTreeMap<KeyID, IdentityPublicKey> {
        [
            key(
                0,
                KeyType::ECDSA_HASH160,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
            ),
            key(
                1,
                KeyType::ECDSA_HASH160,
                Purpose::AUTHENTICATION,
                SecurityLevel::CRITICAL,
            ),
            key(
                2,
                KeyType::ECDSA_HASH160,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
            ),
            key(
                3,
                KeyType::ECDSA_HASH160,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
            ),
            key(
                4,
                KeyType::ECDSA_SECP256K1,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
            ),
            key(
                5,
                KeyType::ECDSA_SECP256K1,
                Purpose::DECRYPTION,
                SecurityLevel::MEDIUM,
            ),
        ]
        .into_iter()
        .map(|k| (k.id(), k))
        .collect()
    }

    fn roles(layout: IdentityCreationKeyLayout) -> Vec<(KeyID, Purpose, SecurityLevel, bool)> {
        layout
            .slots()
            .into_iter()
            .map(|s| {
                (
                    s.key_id,
                    s.purpose,
                    s.security_level,
                    s.contract_bounds.is_some(),
                )
            })
            .collect()
    }

    #[test]
    fn should_append_medium_authentication_key_after_the_existing_new_identity_keys() {
        assert_eq!(
            roles(IdentityCreationKeyLayout::NewIdentity),
            vec![
                (0, Purpose::AUTHENTICATION, SecurityLevel::MASTER, false),
                (1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, false),
                (2, Purpose::AUTHENTICATION, SecurityLevel::HIGH, false),
                (3, Purpose::TRANSFER, SecurityLevel::CRITICAL, false),
                (4, Purpose::AUTHENTICATION, SecurityLevel::MEDIUM, false),
            ]
        );
    }

    #[test]
    fn should_keep_dashpay_key_ids_and_append_medium_authentication_key_at_id_six() {
        assert_eq!(
            roles(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys),
            vec![
                (0, Purpose::AUTHENTICATION, SecurityLevel::MASTER, false),
                (1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, false),
                (2, Purpose::AUTHENTICATION, SecurityLevel::HIGH, false),
                (3, Purpose::TRANSFER, SecurityLevel::CRITICAL, false),
                (4, Purpose::ENCRYPTION, SecurityLevel::MEDIUM, true),
                (5, Purpose::DECRYPTION, SecurityLevel::MEDIUM, true),
                (6, Purpose::AUTHENTICATION, SecurityLevel::MEDIUM, false),
            ]
        );
    }

    #[test]
    fn should_keep_the_four_legacy_keys_for_an_asset_lock_resume() {
        assert_eq!(
            roles(IdentityCreationKeyLayout::AssetLockResume),
            vec![
                (0, Purpose::AUTHENTICATION, SecurityLevel::MASTER, false),
                (1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, false),
                (2, Purpose::AUTHENTICATION, SecurityLevel::HIGH, false),
                (3, Purpose::TRANSFER, SecurityLevel::CRITICAL, false),
            ]
        );
    }

    #[test]
    fn should_bind_dashpay_keys_to_the_contact_request_document_type() {
        let slots = IdentityCreationKeyLayout::NewIdentityWithDashPayKeys.slots();
        let expected = Some(ContractBounds::SingleContractDocumentType {
            id: SystemDataContract::Dashpay.id(),
            document_type_name: "contactRequest".to_string(),
        });
        assert_eq!(slots[4].contract_bounds, expected);
        assert_eq!(slots[5].contract_bounds, expected);
    }

    #[test]
    fn should_derive_every_slot_as_secp256k1_at_its_key_id() {
        for layout in [
            IdentityCreationKeyLayout::NewIdentity,
            IdentityCreationKeyLayout::NewIdentityWithDashPayKeys,
            IdentityCreationKeyLayout::AssetLockResume,
        ] {
            let slots = layout.slots();
            assert_eq!(layout.key_count(), slots.len());
            for (position, slot) in slots.iter().enumerate() {
                assert_eq!(slot.key_id as usize, position, "{layout:?}");
                assert_eq!(slot.derivation_key_index(), slot.key_id, "{layout:?}");
                assert_eq!(slot.key_type, KeyType::ECDSA_SECP256K1, "{layout:?}");
            }
        }
    }

    #[test]
    fn should_accept_both_new_identity_layouts() {
        for layout in [
            IdentityCreationKeyLayout::NewIdentity,
            IdentityCreationKeyLayout::NewIdentityWithDashPayKeys,
        ] {
            let keys = keys_from_layout(layout);
            let validated = IdentityCreationKeys::new_identity(keys.clone())
                .unwrap_or_else(|e| panic!("{layout:?}: {e}"));
            assert_eq!(validated.keys(), &keys);
            assert_eq!(validated.into_keys(), keys);
        }
    }

    #[test]
    fn should_refuse_a_new_identity_without_a_medium_authentication_key() {
        let keys = keys_from_layout(IdentityCreationKeyLayout::AssetLockResume);
        assert_eq!(
            IdentityCreationKeys::new_identity(keys),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::MEDIUM,
            })
        );
    }

    #[test]
    fn should_refuse_a_new_identity_missing_any_authentication_level() {
        for (key_id, security_level) in [
            (1, SecurityLevel::CRITICAL),
            (2, SecurityLevel::HIGH),
            (4, SecurityLevel::MEDIUM),
        ] {
            let mut keys = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
            keys.remove(&key_id);
            assert_eq!(
                IdentityCreationKeys::new_identity(keys),
                Err(IdentityCreationKeysError::MissingAuthenticationKey { security_level })
            );
        }
    }

    #[test]
    fn should_not_count_a_stronger_key_as_the_missing_level() {
        // A second CRITICAL key does not stand in for the MEDIUM one.
        let mut keys = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        keys.insert(
            4,
            key(
                4,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::CRITICAL,
            ),
        );
        assert_eq!(
            IdentityCreationKeys::new_identity(keys),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::MEDIUM,
            })
        );
    }

    #[test]
    fn should_not_count_a_disabled_key_or_a_non_authentication_key() {
        let mut disabled = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        disabled.get_mut(&4).expect("medium key").set_disabled_at(1);
        assert_eq!(
            IdentityCreationKeys::new_identity(disabled),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::MEDIUM,
            })
        );

        // The CRITICAL TRANSFER key does not count as a CRITICAL auth key.
        let mut transfer_only = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        transfer_only.remove(&1);
        assert_eq!(
            IdentityCreationKeys::new_identity(transfer_only),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::CRITICAL,
            })
        );
    }

    #[test]
    fn should_refuse_an_empty_set_a_non_master_key_zero_and_a_misfiled_key() {
        assert_eq!(
            IdentityCreationKeys::new_identity(BTreeMap::new()),
            Err(IdentityCreationKeysError::Empty)
        );

        let mut no_master = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        no_master.remove(&0);
        assert_eq!(
            IdentityCreationKeys::new_identity(no_master),
            Err(IdentityCreationKeysError::KeyZeroNotMasterAuthentication)
        );

        let mut misfiled = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        let medium = misfiled.remove(&4).expect("medium key");
        misfiled.insert(9, medium);
        assert_eq!(
            IdentityCreationKeys::new_identity(misfiled),
            Err(IdentityCreationKeysError::KeyIdMismatch {
                map_key: 9,
                key_id: 4,
            })
        );
    }

    #[test]
    fn should_check_levels_not_key_types() {
        // A caller-built ECDSA_HASH160 set passes once it has a MEDIUM key.
        let mut keys = det_style_keys();
        keys.insert(
            6,
            key(
                6,
                KeyType::ECDSA_HASH160,
                Purpose::AUTHENTICATION,
                SecurityLevel::MEDIUM,
            ),
        );
        assert!(IdentityCreationKeys::new_identity(keys).is_ok());

        assert_eq!(
            IdentityCreationKeys::new_identity(det_style_keys()),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::MEDIUM,
            })
        );
    }

    /// Credits an identity-create needs for `layout`'s keys, via the same
    /// versioned `rs-dpp` computation the exemption uses.
    fn min_fee(layout: IdentityCreationKeyLayout) -> Credits {
        let public_keys = keys_from_layout(layout)
            .values()
            .map(IdentityPublicKeyInCreation::from)
            .collect();
        IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
            public_keys,
            ..Default::default()
        })
        .calculate_min_required_fee(PlatformVersion::latest())
        .expect("min fee")
    }

    /// Smallest whole-duff amount that pays `credits`.
    fn duffs_covering(credits: Credits) -> u64 {
        credits.div_ceil(CREDITS_PER_DUFF)
    }

    #[test]
    fn should_exempt_only_a_lock_that_cannot_fund_the_dip11_key_set() {
        let platform_version = PlatformVersion::latest();
        let dip11 = min_fee(IdentityCreationKeyLayout::NewIdentity);
        let legacy = min_fee(IdentityCreationKeyLayout::AssetLockResume);
        assert_eq!(dip11_key_set_min_fee(platform_version).expect("fee"), dip11);
        assert!(dip11 > legacy, "the MEDIUM key must cost something");

        let exact = duffs_covering(dip11);
        assert_eq!(
            underfunded_for_dip11(exact, platform_version).expect("fee"),
            (false, dip11)
        );
        assert!(
            underfunded_for_dip11(exact - 1, platform_version)
                .expect("fee")
                .0
        );
        // A lock funded at the old four-key minimum is exempt.
        assert!(
            underfunded_for_dip11(duffs_covering(legacy), platform_version)
                .expect("fee")
                .0
        );
    }

    /// A wallet with a ChainLocked registration lock of `amount_duffs`
    /// tracked at the returned outpoint.
    async fn wallet_with_tracked_lock(
        amount_duffs: u64,
    ) -> (
        std::sync::Arc<crate::PlatformWallet>,
        OutPoint,
        std::sync::Arc<crate::PlatformWalletManager<crate::test_support::NoopTestPersister>>,
    ) {
        use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (manager, wallet_id) = crate::test_support::test_platform_wallet_manager().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
        let out_point = OutPoint {
            txid: dashcore::Txid::from_byte_array([7; 32]),
            vout: 0,
        };
        wallet
            .identity()
            .asset_locks
            .track_asset_lock(TrackedAssetLock {
                operation_id: None,
                out_point,
                transaction: dashcore::Transaction {
                    version: 3,
                    lock_time: 0,
                    input: vec![],
                    output: vec![],
                    special_transaction_payload: None,
                },
                account_index: 0,
                funding_type: AssetLockFundingType::IdentityRegistration,
                identity_index: 0,
                amount: amount_duffs,
                status: AssetLockStatus::ChainLocked,
                proof: None,
            })
            .await;
        (wallet, out_point, manager)
    }

    fn resume_of(out_point: OutPoint) -> AssetLockFunding {
        AssetLockFunding::FromExistingAssetLock {
            out_point,
            consume_invitation_voucher: false,
        }
    }

    fn missing_medium() -> IdentityCreationKeysError {
        IdentityCreationKeysError::MissingAuthenticationKey {
            security_level: SecurityLevel::MEDIUM,
        }
    }

    fn creation_keys_error(err: PlatformWalletError) -> IdentityCreationKeysError {
        match err {
            PlatformWalletError::IdentityCreationKeys(e) => e,
            other => panic!("expected an IdentityCreationKeys error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_resume_an_underfunded_legacy_lock_with_the_four_pre_dip11_keys() {
        let legacy_minimum = duffs_covering(min_fee(IdentityCreationKeyLayout::AssetLockResume));
        let (wallet, out_point, _manager) = wallet_with_tracked_lock(legacy_minimum).await;
        let iw = wallet.identity();
        let funding = resume_of(out_point);

        assert_eq!(
            iw.resume_key_layout(&out_point).await.expect("tracked"),
            IdentityCreationKeyLayout::AssetLockResume
        );
        // The four keys hosts re-derive, and a caller-built DET-style set,
        // both resume with the pre-DIP-11 check and nothing added.
        for original in [
            keys_from_layout(IdentityCreationKeyLayout::AssetLockResume),
            det_style_keys(),
        ] {
            let keys = iw
                .creation_keys_for_funding(original.clone(), &funding)
                .await
                .expect("an underfunded resume keeps the pre-DIP-11 check");
            iw.ensure_creation_keys_permitted(&keys, &funding)
                .await
                .expect("the exempt resume accepts its keys");
            assert_eq!(keys.into_keys(), original);
        }

        // The pre-DIP-11 check itself still applies.
        let mut no_master = keys_from_layout(IdentityCreationKeyLayout::AssetLockResume);
        no_master.remove(&0);
        let err = iw
            .creation_keys_for_funding(no_master, &funding)
            .await
            .expect_err("key 0 must stay MASTER");
        assert_eq!(
            creation_keys_error(err),
            IdentityCreationKeysError::KeyZeroNotMasterAuthentication
        );
    }

    #[tokio::test]
    async fn should_require_every_level_to_resume_an_amply_funded_lock() {
        // Covers a legacy lock funded above the minimum and a DET-style lock
        // built first and registered later: both can pay for the MEDIUM key.
        let exact = duffs_covering(min_fee(IdentityCreationKeyLayout::NewIdentity));
        for amount in [exact, 10_000_000] {
            let (wallet, out_point, _manager) = wallet_with_tracked_lock(amount).await;
            let iw = wallet.identity();
            let funding = resume_of(out_point);

            assert_eq!(
                iw.resume_key_layout(&out_point).await.expect("tracked"),
                IdentityCreationKeyLayout::NewIdentity
            );
            for keys in [
                keys_from_layout(IdentityCreationKeyLayout::AssetLockResume),
                det_style_keys(),
            ] {
                let err = iw
                    .creation_keys_for_funding(keys, &funding)
                    .await
                    .expect_err("an amply funded resume needs all four levels");
                assert_eq!(creation_keys_error(err), missing_medium());
            }
            let keys = iw
                .creation_keys_for_funding(
                    keys_from_layout(IdentityCreationKeyLayout::NewIdentity),
                    &funding,
                )
                .await
                .expect("the DIP-11 layout resumes");
            iw.ensure_creation_keys_permitted(&keys, &funding)
                .await
                .expect("DIP-11 keys are always permitted");
        }
    }

    #[tokio::test]
    async fn should_never_exempt_fresh_or_invitation_funding() {
        // Even with an underfunded tracked lock in the wallet, only its
        // plain resume is exempt.
        let legacy_minimum = duffs_covering(min_fee(IdentityCreationKeyLayout::AssetLockResume));
        let (wallet, out_point, _manager) = wallet_with_tracked_lock(legacy_minimum).await;
        let iw = wallet.identity();
        let fresh = AssetLockFunding::FromWalletBalance {
            amount_duffs: legacy_minimum,
            account_index: 0,
        };
        let invitation_reclaim = AssetLockFunding::FromExistingAssetLock {
            out_point,
            consume_invitation_voucher: true,
        };
        for funding in [&fresh, &invitation_reclaim] {
            let err = iw
                .creation_keys_for_funding(
                    keys_from_layout(IdentityCreationKeyLayout::AssetLockResume),
                    funding,
                )
                .await
                .expect_err("not exempt");
            assert_eq!(creation_keys_error(err), missing_medium());
        }

        // Pre-DIP-11 keys obtained for the exempt resume cannot be reused for
        // any other creation.
        let legacy_keys = iw
            .creation_keys_for_funding(
                keys_from_layout(IdentityCreationKeyLayout::AssetLockResume),
                &resume_of(out_point),
            )
            .await
            .expect("exempt resume");
        for funding in [&fresh, &invitation_reclaim] {
            let err = iw
                .ensure_creation_keys_permitted(&legacy_keys, funding)
                .await
                .expect_err("not exempt");
            assert_eq!(
                creation_keys_error(err),
                IdentityCreationKeysError::LegacyKeysNotExempt
            );
        }
        assert_eq!(
            legacy_keys.ensure_new_identity(),
            Err(IdentityCreationKeysError::LegacyKeysNotExempt)
        );
    }

    #[tokio::test]
    async fn should_refuse_to_decide_a_resume_of_an_untracked_lock() {
        let (wallet, _, _manager) = wallet_with_tracked_lock(1).await;
        let untracked = OutPoint {
            txid: dashcore::Txid::from_byte_array([9; 32]),
            vout: 1,
        };
        assert!(matches!(
            wallet.identity().resume_key_layout(&untracked).await,
            Err(PlatformWalletError::AssetLockNotTracked(o)) if o == untracked
        ));
        assert!(matches!(
            wallet
                .identity()
                .creation_keys_for_funding(det_style_keys(), &resume_of(untracked))
                .await,
            Err(PlatformWalletError::AssetLockNotTracked(_))
        ));
    }
}
