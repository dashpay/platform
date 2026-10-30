//! The key set a new identity is created with.
//!
//! [`IdentityCreationKeyLayout`] is the single source of the key layout
//! this crate's hosts (the FFI, the JNI bridge, and the Swift and Kotlin
//! SDKs) register a new identity with. Hosts derive one key per slot and
//! stamp the slot's role on it; they never decide the layout themselves.
//!
//! [`IdentityCreationKeys`] is the write-path pre-flight. Every
//! identity-creating entry point of this crate takes it instead of a raw
//! key map, so a key set without an AUTHENTICATION key at each of MASTER,
//! CRITICAL and HIGH, or with a disabled key, is refused before anything is
//! signed or broadcast. Consensus does not enforce the level rule, so this
//! client check does not duplicate a `rs-dpp` validation. The structure rules
//! consensus does enforce are `rs-dpp`'s, which the pre-flight calls.
//!
//! Every asset-lock funded registration must also be able to pay for its
//! keys: `register_identity_with_funding` refuses a lock it would build, or
//! a tracked lock it would resume, below the minimum for those keys. Hosts
//! read that minimum, and the address-funded fee, from
//! [`IdentityWallet::identity_creation_min_funding_duffs`] and
//! [`IdentityWallet::identity_creation_from_addresses_min_fees`] instead of
//! keeping a copy of the fee schedule.

use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::consensus::ConsensusError;
use dpp::data_contracts::SystemDataContract;
use dpp::fee::Credits;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::StateTransitionEstimatedFeeValidation;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

use super::IdentityWallet;
use crate::error::PlatformWalletError;
use crate::util::ceil_credits_to_duffs;
use crate::wallet::asset_lock::AssetLockFunding;

/// DashPay document type the ENCRYPTION / DECRYPTION keys are bound to. It is
/// the only DashPay document type that requires an identity-encryption bound
/// key.
const DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE: &str = "contactRequest";

/// Authentication security levels a new identity must hold a key AT (not
/// merely at or above), in the order the error reports them.
///
/// DIP-11 also asks for a MEDIUM key, but a creation may carry at most
/// `max_public_keys_in_creation` (6) keys, which the DashPay layout already
/// fills, and no system contract accepts a MEDIUM signature (they all
/// require HIGH), so a MEDIUM key would have no use.
const REQUIRED_AUTHENTICATION_LEVELS: [SecurityLevel; 3] = [
    SecurityLevel::MASTER,
    SecurityLevel::CRITICAL,
    SecurityLevel::HIGH,
];

/// Which key layout an identity-creation flow registers.
///
/// Every slot is an `ECDSA_SECP256K1` key whose DIP-9 key index equals its
/// key id (`m/9'/coin'/5'/0'/0'/identity_index'/key_id'`). Discovery and the
/// derivation breadcrumbs rely on that equality.
///
/// | key id | purpose        | level    | layouts                      |
/// |--------|----------------|----------|------------------------------|
/// | 0      | AUTHENTICATION | MASTER   | all                          |
/// | 1      | AUTHENTICATION | CRITICAL | all                          |
/// | 2      | AUTHENTICATION | HIGH     | all                          |
/// | 3      | TRANSFER       | CRITICAL | all                          |
/// | 4      | ENCRYPTION     | MEDIUM   | `NewIdentityWithDashPayKeys` |
/// | 5      | DECRYPTION     | MEDIUM   | `NewIdentityWithDashPayKeys` |
///
/// The ENCRYPTION / DECRYPTION keys are bound to DashPay's `contactRequest`
/// document type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentityCreationKeyLayout {
    /// A new identity without DashPay keys. Also the layout a resume of a
    /// tracked registration lock re-derives.
    NewIdentity,
    /// A new identity that can send and receive DashPay contact requests.
    NewIdentityWithDashPayKeys,
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
        slots
    }

    /// Number of keys the layout registers, for sizing the creation fee.
    pub fn key_count(self) -> usize {
        self.slots().len()
    }
}

/// Why a key set cannot create an identity.
#[derive(Debug, thiserror::Error)]
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
    /// A key is disabled. A key in creation carries no disabled state, so it
    /// would be registered enabled.
    #[error("key id {key_id} is disabled; a new identity cannot register a disabled key")]
    DisabledKey {
        /// The disabled key's id.
        key_id: KeyID,
    },
    /// Key id 0 is missing or is not an AUTHENTICATION key at MASTER level.
    /// It signs the identity-creation transition.
    #[error("key id 0 must be an AUTHENTICATION key at MASTER security level")]
    KeyZeroNotMasterAuthentication,
    /// The key set has no AUTHENTICATION key at this security level.
    #[error("a new identity must have an AUTHENTICATION key at {security_level:?} security level")]
    MissingAuthenticationKey {
        /// The security level without an authentication key.
        security_level: SecurityLevel,
    },
    /// Platform's identity-create structure validation refuses the key set.
    #[error(
        "Platform would refuse these identity keys: {}",
        join_consensus_errors(.errors)
    )]
    StructureRefused {
        /// Every consensus error the validation reported.
        errors: Vec<ConsensusError>,
    },
    /// The structure validation could not run at this protocol version.
    #[error("could not validate the identity keys for this protocol version: {0}")]
    ValidationUnavailable(#[source] Box<ProtocolError>),
}

fn join_consensus_errors(errors: &[ConsensusError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// A key set that passed the pre-flight for creating an identity.
///
/// Build it with [`IdentityWallet::identity_creation_keys`], which validates at
/// the wallet's own protocol version. A caller-built set (for example one
/// with `ECDSA_HASH160` keys) passes as long as it has the required
/// authentication levels and Platform's structure validation accepts it.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityCreationKeys {
    keys: BTreeMap<KeyID, IdentityPublicKey>,
}

impl IdentityCreationKeys {
    /// Validate `keys` for a new identity: non-empty, each key under its own
    /// id, no key disabled, key id 0 AUTHENTICATION/MASTER, and an
    /// AUTHENTICATION key at each of MASTER, CRITICAL and HIGH. Security
    /// levels are minimums for signing, but one key AT each level is
    /// required here. Then runs `rs-dpp`'s structure validation for identity
    /// creation at `platform_version`, which leaves secp256k1 key data to the
    /// proof of possession made when signing.
    pub(crate) fn new_identity(
        keys: BTreeMap<KeyID, IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, IdentityCreationKeysError> {
        if keys.is_empty() {
            return Err(IdentityCreationKeysError::Empty);
        }
        if let Some((map_key, key)) = keys.iter().find(|(map_key, key)| **map_key != key.id()) {
            return Err(IdentityCreationKeysError::KeyIdMismatch {
                map_key: *map_key,
                key_id: key.id(),
            });
        }
        if let Some(key) = keys.values().find(|key| key.disabled_at().is_some()) {
            return Err(IdentityCreationKeysError::DisabledKey { key_id: key.id() });
        }
        match keys.get(&0) {
            Some(key)
                if key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER => {}
            _ => return Err(IdentityCreationKeysError::KeyZeroNotMasterAuthentication),
        }
        for security_level in REQUIRED_AUTHENTICATION_LEVELS {
            let present = keys.values().any(|key| {
                key.purpose() == Purpose::AUTHENTICATION && key.security_level() == security_level
            });
            if !present {
                return Err(IdentityCreationKeysError::MissingAuthenticationKey { security_level });
            }
        }
        let public_keys: Vec<IdentityPublicKeyInCreation> = keys
            .values()
            .map(IdentityPublicKeyInCreation::from)
            .collect();
        let validation = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &public_keys,
            true,
            platform_version,
        )
        .map_err(|e| IdentityCreationKeysError::ValidationUnavailable(Box::new(e)))?;
        if !validation.is_valid() {
            return Err(IdentityCreationKeysError::StructureRefused {
                errors: validation.errors,
            });
        }
        Ok(Self { keys })
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

/// `layout`'s keys as they enter an identity-create fee computation. The
/// fee depends on the key count only, so the key data is a placeholder.
fn layout_keys_in_creation(layout: IdentityCreationKeyLayout) -> Vec<IdentityPublicKeyInCreation> {
    layout
        .slots()
        .iter()
        .map(|slot| {
            IdentityPublicKeyInCreation::from(&slot.public_key(BinaryData::new(vec![0; 33])))
        })
        .collect()
}

/// The minimum asset-lock value (duffs) an identity-create transition with
/// `public_keys` needs, from the versioned `rs-dpp` fee computation (no fee
/// number lives here), rounded up to whole duffs.
fn asset_lock_create_min_duffs(
    public_keys: Vec<IdentityPublicKeyInCreation>,
    platform_version: &PlatformVersion,
) -> Result<u64, ProtocolError> {
    IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
        public_keys,
        ..Default::default()
    })
    .calculate_min_required_fee(platform_version)
    .map(ceil_credits_to_duffs)
}

/// The minimum asset-lock value (duffs) registering `layout` needs.
fn layout_min_funding_duffs(
    layout: IdentityCreationKeyLayout,
    platform_version: &PlatformVersion,
) -> Result<u64, ProtocolError> {
    asset_lock_create_min_duffs(layout_keys_in_creation(layout), platform_version)
}

/// The minimum fee (credits) of an address-funded identity create of
/// `layout`, without a refund output, for each input count from 1 to the
/// protocol's `max_address_inputs` (index `n - 1` holds the fee for `n`
/// inputs).
fn layout_from_addresses_min_fees(
    layout: IdentityCreationKeyLayout,
    platform_version: &PlatformVersion,
) -> Result<Vec<Credits>, ProtocolError> {
    let public_keys = layout_keys_in_creation(layout);
    let max_inputs = platform_version.dpp.state_transitions.max_address_inputs;
    let mut inputs = BTreeMap::new();
    let mut fees = Vec::with_capacity(usize::from(max_inputs));
    for input in 0..u32::from(max_inputs) {
        let mut hash = [0u8; 20];
        hash[..4].copy_from_slice(&input.to_be_bytes());
        inputs.insert(PlatformAddress::P2pkh(hash), (0, 0));
        let fee =
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: public_keys.clone(),
                inputs: inputs.clone(),
                ..Default::default()
            })
            .calculate_min_required_fee(platform_version)?;
        fees.push(fee);
    }
    Ok(fees)
}

/// Raise or check `funding` against `required_duffs`, the minimum lock value
/// an identity create with the caller's keys needs:
/// - [`AssetLockFunding::FromWalletBalance`] below the minimum is refused;
/// - [`AssetLockFunding::DrainAccountBalance`] has its floor raised to it;
/// - [`AssetLockFunding::FromExistingAssetLock`] is refused when
///   `existing_lock_amount_duffs` (the tracked lock's persisted amount) is
///   below it. An untracked lock (`None`) is left to the funding resolver.
///
/// [`AssetLockFunding::DrainWalletBalance`] is refused for registration by
/// the funding resolver.
fn apply_identity_create_floor(
    funding: AssetLockFunding,
    required_duffs: u64,
    existing_lock_amount_duffs: Option<u64>,
) -> Result<AssetLockFunding, PlatformWalletError> {
    let insufficient = |available| PlatformWalletError::AssetLockInsufficientFunds {
        available,
        required: required_duffs,
    };
    match funding {
        AssetLockFunding::FromWalletBalance { amount_duffs, .. }
            if amount_duffs < required_duffs =>
        {
            Err(insufficient(amount_duffs))
        }
        AssetLockFunding::DrainAccountBalance {
            account,
            minimum_lock_duffs,
        } => Ok(AssetLockFunding::DrainAccountBalance {
            account,
            minimum_lock_duffs: Some(
                minimum_lock_duffs.map_or(required_duffs, |floor| floor.max(required_duffs)),
            ),
        }),
        AssetLockFunding::FromExistingAssetLock { .. } => match existing_lock_amount_duffs {
            Some(amount_duffs) if amount_duffs < required_duffs => Err(insufficient(amount_duffs)),
            _ => Ok(funding),
        },
        funding => Ok(funding),
    }
}

fn protocol_error(error: ProtocolError) -> PlatformWalletError {
    PlatformWalletError::Sdk(dash_sdk::Error::Protocol(error))
}

impl IdentityWallet {
    /// Validate `keys` for a new identity at the SDK's current platform
    /// version, which on an unpinned SDK can lag the network until its first
    /// proved response. A refusal happens before any lock is built, waited on
    /// or signed for.
    pub fn identity_creation_keys(
        &self,
        keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<IdentityCreationKeys, PlatformWalletError> {
        Ok(IdentityCreationKeys::new_identity(
            keys,
            self.sdk.version(),
        )?)
    }

    /// The minimum asset-lock value (duffs) that registers `layout` at this
    /// wallet's platform version. Hosts show and enforce it instead of
    /// keeping their own copy of the fee schedule.
    pub fn identity_creation_min_funding_duffs(
        &self,
        layout: IdentityCreationKeyLayout,
    ) -> Result<u64, PlatformWalletError> {
        layout_min_funding_duffs(layout, self.sdk.version()).map_err(protocol_error)
    }

    /// The minimum fee (credits) of an address-funded identity create of
    /// `layout` without a refund output, at this wallet's platform version,
    /// for each input count from 1 to the protocol maximum: index `n - 1`
    /// holds the fee for `n` inputs, and the length is the maximum.
    pub fn identity_creation_from_addresses_min_fees(
        &self,
        layout: IdentityCreationKeyLayout,
    ) -> Result<Vec<Credits>, PlatformWalletError> {
        layout_from_addresses_min_fees(layout, self.sdk.version()).map_err(protocol_error)
    }

    /// Make sure `funding` can pay for an identity create with `keys`
    /// before anything is built, waited on, signed or broadcast (see
    /// [`apply_identity_create_floor`]): Platform would reject an
    /// underfunded lock, a tracked one only after the finality wait.
    pub(super) async fn enforce_identity_create_floor(
        &self,
        funding: AssetLockFunding,
        keys: &IdentityCreationKeys,
    ) -> Result<AssetLockFunding, PlatformWalletError> {
        let required_duffs = asset_lock_create_min_duffs(
            keys.keys()
                .values()
                .map(IdentityPublicKeyInCreation::from)
                .collect(),
            self.sdk.version(),
        )
        .map_err(protocol_error)?;
        let existing_lock_amount_duffs = match &funding {
            AssetLockFunding::FromExistingAssetLock { out_point, .. } => {
                self.asset_locks.tracked_amount_duffs(out_point).await
            }
            _ => None,
        };
        apply_identity_create_floor(funding, required_duffs, existing_lock_amount_duffs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Txid};
    use dpp::address_funds::{AddressFundsFeeStrategyStep, AddressWitness};
    use dpp::balances::credits::CREDITS_PER_DUFF;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::signer::Signer as IdentitySigner;
    use dpp::state_transition::StateTransitionStructureValidation;
    use dpp::validation::SimpleConsensusValidationResult;
    use key_wallet::bip32::{DerivationPath, ExtendedPubKey};
    use key_wallet::signer::{ExtendedPubKeySigner, Signer as CoreSigner, SignerMethod};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount;

    use crate::test_support::{
        test_platform_wallet_manager, test_platform_wallet_manager_with_sdk,
        track_registration_asset_lock, NoopTestPersister,
    };
    use crate::{PlatformWallet, PlatformWalletManager};

    const LAYOUTS: [IdentityCreationKeyLayout; 2] = [
        IdentityCreationKeyLayout::NewIdentity,
        IdentityCreationKeyLayout::NewIdentityWithDashPayKeys,
    ];

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
            data: BinaryData::new(vec![id as u8; key_type.default_size()]),
            disabled_at: None,
        })
    }

    /// `layout`'s keys, each with its own key data.
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

    /// The default key set Dash Evo Tool registers (its
    /// `default_identity_key_specs` plus the master key): 20-byte
    /// ECDSA_HASH160 MASTER / CRITICAL / HIGH / TRANSFER keys and the
    /// `contactRequest`-bound ECDSA_SECP256K1 DashPay pair.
    fn det_style_keys() -> BTreeMap<KeyID, IdentityPublicKey> {
        let dashpay_key = |id, purpose| {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id,
                purpose,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                    id: SystemDataContract::Dashpay.id(),
                    document_type_name: DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE.to_string(),
                }),
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![id as u8; 33]),
                disabled_at: None,
            })
        };
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
            dashpay_key(4, Purpose::ENCRYPTION),
            dashpay_key(5, Purpose::DECRYPTION),
        ]
        .into_iter()
        .map(|k| (k.id(), k))
        .collect()
    }

    fn new_identity(
        keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<IdentityCreationKeys, IdentityCreationKeysError> {
        IdentityCreationKeys::new_identity(keys, PlatformVersion::latest())
    }

    /// The consensus errors `keys` are refused with at the latest version.
    fn structure_errors(keys: BTreeMap<KeyID, IdentityPublicKey>) -> Vec<ConsensusError> {
        match new_identity(keys) {
            Err(IdentityCreationKeysError::StructureRefused { errors }) => errors,
            other => panic!("expected a structure refusal, got {other:?}"),
        }
    }

    fn with_key(
        mut keys: BTreeMap<KeyID, IdentityPublicKey>,
        key: IdentityPublicKey,
    ) -> BTreeMap<KeyID, IdentityPublicKey> {
        keys.insert(key.id(), key);
        keys
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
    fn should_register_the_four_base_keys_for_a_new_identity() {
        assert_eq!(
            roles(IdentityCreationKeyLayout::NewIdentity),
            vec![
                (0, Purpose::AUTHENTICATION, SecurityLevel::MASTER, false),
                (1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, false),
                (2, Purpose::AUTHENTICATION, SecurityLevel::HIGH, false),
                (3, Purpose::TRANSFER, SecurityLevel::CRITICAL, false),
            ]
        );
    }

    #[test]
    fn should_add_the_dashpay_pair_at_ids_four_and_five() {
        assert_eq!(
            roles(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys),
            vec![
                (0, Purpose::AUTHENTICATION, SecurityLevel::MASTER, false),
                (1, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, false),
                (2, Purpose::AUTHENTICATION, SecurityLevel::HIGH, false),
                (3, Purpose::TRANSFER, SecurityLevel::CRITICAL, false),
                (4, Purpose::ENCRYPTION, SecurityLevel::MEDIUM, true),
                (5, Purpose::DECRYPTION, SecurityLevel::MEDIUM, true),
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
        for layout in LAYOUTS {
            let slots = layout.slots();
            assert_eq!(layout.key_count(), slots.len());
            for (position, slot) in slots.iter().enumerate() {
                assert_eq!(slot.key_id as usize, position, "{layout:?}");
                assert_eq!(slot.derivation_key_index(), slot.key_id, "{layout:?}");
                assert_eq!(slot.key_type, KeyType::ECDSA_SECP256K1, "{layout:?}");
            }
        }
    }

    /// An address-funded identity create of `public_keys` that is
    /// structurally valid apart from its keys.
    fn from_addresses_create(
        public_keys: Vec<IdentityPublicKeyInCreation>,
        platform_version: &PlatformVersion,
    ) -> IdentityCreateFromAddressesTransitionV0 {
        let funds = &platform_version.dpp.state_transitions.address_funds;
        let amount = funds
            .min_input_amount
            .max(funds.min_identity_funding_amount)
            * 2;
        IdentityCreateFromAddressesTransitionV0 {
            public_keys,
            inputs: BTreeMap::from([(PlatformAddress::P2pkh([1; 20]), (1, amount))]),
            output: None,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0; 65]),
            }],
        }
    }

    /// The consensus structure checks an identity create's keys go through:
    /// the key-structure validation drive-abci runs for asset-lock,
    /// address-funded and shielded creates (key limit, duplicates, one
    /// master, allowed levels), and the address-funded transition's basic
    /// structure.
    fn creation_structure_results(
        keys: &BTreeMap<KeyID, IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> [SimpleConsensusValidationResult; 2] {
        let public_keys: Vec<IdentityPublicKeyInCreation> = keys
            .values()
            .map(IdentityPublicKeyInCreation::from)
            .collect();
        [
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &public_keys,
                true,
                platform_version,
            )
            .expect("known validation version"),
            from_addresses_create(public_keys, platform_version)
                .validate_structure(platform_version),
        ]
    }

    #[test]
    fn should_pass_dpp_creation_structure_validation_for_every_layout() {
        let platform_version = PlatformVersion::latest();
        let max_keys = platform_version
            .dpp
            .state_transitions
            .identities
            .max_public_keys_in_creation as usize;
        for layout in LAYOUTS {
            for result in creation_structure_results(&keys_from_layout(layout), platform_version) {
                assert!(result.is_valid(), "{layout:?}: {:?}", result.errors);
            }
        }

        // Control: the same checks refuse one key over the creation limit,
        // so a layout that outgrows it fails above.
        let mut over_limit =
            keys_from_layout(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys);
        while over_limit.len() <= max_keys {
            let id = over_limit.len() as KeyID;
            over_limit.insert(
                id,
                key(
                    id,
                    KeyType::ECDSA_SECP256K1,
                    Purpose::AUTHENTICATION,
                    SecurityLevel::HIGH,
                ),
            );
        }
        for result in creation_structure_results(&over_limit, platform_version) {
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::StateError(
                        StateError::MaxIdentityPublicKeyLimitReachedError(_)
                    )]
                ),
                "{:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_accept_both_layouts_and_the_det_key_set() {
        for keys in [
            keys_from_layout(IdentityCreationKeyLayout::NewIdentity),
            keys_from_layout(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys),
            // Hash keys pass: only their data size is checked.
            det_style_keys(),
        ] {
            let validated = new_identity(keys.clone()).unwrap_or_else(|e| panic!("{keys:?}: {e}"));
            assert_eq!(validated.keys(), &keys);
            assert_eq!(validated.into_keys(), keys);
        }
    }

    #[test]
    fn should_refuse_more_keys_than_a_creation_may_carry() {
        let max_keys = PlatformVersion::latest()
            .dpp
            .state_transitions
            .identities
            .max_public_keys_in_creation as KeyID;
        let keys = (6..=max_keys).fold(
            keys_from_layout(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys),
            |keys, id| {
                with_key(
                    keys,
                    key(
                        id,
                        KeyType::ECDSA_SECP256K1,
                        Purpose::AUTHENTICATION,
                        SecurityLevel::HIGH,
                    ),
                )
            },
        );
        assert_eq!(keys.len(), max_keys as usize + 1);
        let errors = structure_errors(keys);
        assert!(
            matches!(
                errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::MaxIdentityPublicKeyLimitReachedError(_)
                )]
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn should_refuse_the_same_key_data_under_two_key_ids() {
        let mut keys = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        let data = keys.get(&1).expect("critical key").data().clone();
        keys.get_mut(&2).expect("high key").set_data(data);
        let errors = structure_errors(keys);
        assert!(
            matches!(
                errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DuplicatedIdentityPublicKeyBasicError(_)
                )]
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn should_refuse_a_second_master_key() {
        let errors = structure_errors(with_key(
            keys_from_layout(IdentityCreationKeyLayout::NewIdentity),
            key(
                4,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
            ),
        ));
        assert!(
            matches!(
                errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::TooManyMasterPublicKeyError(_)
                )]
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn should_refuse_a_purpose_at_a_level_it_may_not_have() {
        for (purpose, level) in [
            (Purpose::TRANSFER, SecurityLevel::HIGH),
            (Purpose::ENCRYPTION, SecurityLevel::HIGH),
            (Purpose::DECRYPTION, SecurityLevel::CRITICAL),
            (Purpose::VOTING, SecurityLevel::HIGH),
        ] {
            let errors = structure_errors(with_key(
                keys_from_layout(IdentityCreationKeyLayout::NewIdentity),
                key(4, KeyType::ECDSA_SECP256K1, purpose, level),
            ));
            assert!(
                matches!(
                    errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityPublicKeySecurityLevelError(_)
                    )]
                ),
                "{purpose:?}/{level:?}: {errors:?}"
            );
        }
    }

    #[test]
    fn should_refuse_a_hash_key_whose_data_is_not_a_hash() {
        for len in [0, 19, 21, 33] {
            let mut keys = det_style_keys();
            keys.get_mut(&1)
                .expect("critical key")
                .set_data(BinaryData::new(vec![0xAB; len]));
            let errors = structure_errors(keys);
            assert!(
                matches!(
                    errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityPublicKeyDataError(e)
                    )] if e.public_key_id() == 1
                ),
                "{len} bytes: {errors:?}"
            );
        }
    }

    #[test]
    fn should_leave_secp256k1_key_data_to_its_proof_of_possession() {
        // An uncompressed 65-byte key is valid; a malformed one is refused
        // by the proof of possession, not by the structure rules.
        for len in [20, 33, 65] {
            let mut keys = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
            keys.get_mut(&1)
                .expect("critical key")
                .set_data(BinaryData::new(vec![0xAB; len]));
            new_identity(keys).unwrap_or_else(|e| panic!("{len} bytes: {e}"));
        }
    }

    #[test]
    fn should_apply_the_structure_rules_of_the_given_protocol_version() {
        // Protocol version 13 does not check the size of a hash key's data.
        let mut keys = det_style_keys();
        keys.get_mut(&1)
            .expect("critical key")
            .set_data(BinaryData::new(vec![0xAB; 19]));
        let older = PlatformVersion::get(13).expect("protocol version 13");
        IdentityCreationKeys::new_identity(keys.clone(), older).expect("no size rule at 13");
        assert!(matches!(
            new_identity(keys),
            Err(IdentityCreationKeysError::StructureRefused { .. })
        ));
    }

    #[tokio::test]
    async fn should_validate_a_wallets_keys_at_that_wallets_protocol_version() {
        // Protocol version 13 does not check the size of a hash key's data.
        let mut keys = det_style_keys();
        keys.get_mut(&1)
            .expect("critical key")
            .set_data(BinaryData::new(vec![0xAB; 19]));

        let wallet_at = |version| async move {
            let sdk = dash_sdk::SdkBuilder::new_mock()
                .with_version(version)
                .build()
                .expect("mock sdk");
            let (manager, wallet_id) =
                test_platform_wallet_manager_with_sdk(Arc::new(sdk), Arc::new(NoopTestPersister))
                    .await;
            let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
            (manager, wallet)
        };

        let older = PlatformVersion::get(13).expect("protocol version 13");
        let (_manager, wallet) = wallet_at(older).await;
        wallet
            .identity()
            .identity_creation_keys(keys.clone())
            .expect("no size rule at 13");

        let (_manager, wallet) = wallet_at(PlatformVersion::latest()).await;
        assert!(matches!(
            wallet.identity().identity_creation_keys(keys),
            Err(PlatformWalletError::IdentityCreationKeys(
                IdentityCreationKeysError::StructureRefused { .. }
            ))
        ));
    }

    #[test]
    fn should_name_the_consensus_reason_in_a_refusal() {
        let err = new_identity(with_key(
            keys_from_layout(IdentityCreationKeyLayout::NewIdentity),
            key(
                4,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
            ),
        ))
        .expect_err("two master keys");
        let IdentityCreationKeysError::StructureRefused { errors } = &err else {
            panic!("expected a structure refusal, got {err:?}");
        };
        assert!(err.to_string().contains(&errors[0].to_string()), "{err}");
    }

    #[test]
    fn should_refuse_a_new_identity_missing_any_authentication_level() {
        for (key_id, security_level) in [(1, SecurityLevel::CRITICAL), (2, SecurityLevel::HIGH)] {
            let mut keys = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
            keys.remove(&key_id);
            assert!(matches!(
                new_identity(keys),
                Err(IdentityCreationKeysError::MissingAuthenticationKey { security_level: level })
                    if level == security_level
            ));
        }
    }

    #[test]
    fn should_not_count_a_stronger_key_or_a_transfer_key_as_the_missing_level() {
        // A second CRITICAL key does not stand in for the HIGH one.
        let mut stronger = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        stronger.insert(
            2,
            key(
                2,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::CRITICAL,
            ),
        );
        assert!(matches!(
            new_identity(stronger),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::HIGH,
            })
        ));

        // The CRITICAL TRANSFER key does not count as a CRITICAL auth key.
        let mut transfer_only = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        transfer_only.remove(&1);
        assert!(matches!(
            new_identity(transfer_only),
            Err(IdentityCreationKeysError::MissingAuthenticationKey {
                security_level: SecurityLevel::CRITICAL,
            })
        ));
    }

    #[test]
    fn should_refuse_any_disabled_key() {
        // A disabled key 0 with an enabled MASTER key elsewhere: key 0 would
        // still be registered, and enabled.
        let mut disabled_master = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        disabled_master
            .get_mut(&0)
            .expect("master key")
            .set_disabled_at(1);
        disabled_master.insert(
            4,
            key(
                4,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
            ),
        );
        assert!(matches!(
            new_identity(disabled_master),
            Err(IdentityCreationKeysError::DisabledKey { key_id: 0 })
        ));

        // So is a disabled key at a level no check requires.
        let mut disabled_dashpay =
            keys_from_layout(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys);
        disabled_dashpay
            .get_mut(&5)
            .expect("decryption key")
            .set_disabled_at(1);
        assert!(matches!(
            new_identity(disabled_dashpay),
            Err(IdentityCreationKeysError::DisabledKey { key_id: 5 })
        ));
    }

    #[test]
    fn should_refuse_an_empty_set_a_non_master_key_zero_and_a_misfiled_key() {
        assert!(matches!(
            new_identity(BTreeMap::new()),
            Err(IdentityCreationKeysError::Empty)
        ));

        let mut no_key_zero = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        no_key_zero.remove(&0);
        assert!(matches!(
            new_identity(no_key_zero),
            Err(IdentityCreationKeysError::KeyZeroNotMasterAuthentication)
        ));

        // A MASTER key at another id does not make up for key 0.
        let mut swapped = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        swapped.insert(
            0,
            key(
                0,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
            ),
        );
        swapped.insert(
            2,
            key(
                2,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
            ),
        );
        assert!(matches!(
            new_identity(swapped),
            Err(IdentityCreationKeysError::KeyZeroNotMasterAuthentication)
        ));

        let mut misfiled = keys_from_layout(IdentityCreationKeyLayout::NewIdentity);
        let transfer = misfiled.remove(&3).expect("transfer key");
        misfiled.insert(9, transfer);
        assert!(matches!(
            new_identity(misfiled),
            Err(IdentityCreationKeysError::KeyIdMismatch {
                map_key: 9,
                key_id: 3,
            })
        ));
    }

    /// Credits an identity-create needs for `layout`'s keys, straight from
    /// the versioned `rs-dpp` computation.
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

    /// Smallest whole-duff amount that pays for `layout`.
    fn min_duffs(layout: IdentityCreationKeyLayout) -> u64 {
        min_fee(layout).div_ceil(CREDITS_PER_DUFF)
    }

    #[test]
    fn should_compute_creation_minimums_from_the_versioned_fee_schedule() {
        let platform_version = PlatformVersion::latest();
        for layout in LAYOUTS {
            assert_eq!(
                layout_min_funding_duffs(layout, platform_version).expect("fee"),
                min_duffs(layout)
            );
        }
        assert!(
            min_duffs(IdentityCreationKeyLayout::NewIdentity)
                < min_duffs(IdentityCreationKeyLayout::NewIdentityWithDashPayKeys),
            "the DashPay keys must cost something"
        );

        let base = layout_from_addresses_min_fees(
            IdentityCreationKeyLayout::NewIdentity,
            platform_version,
        )
        .expect("fees");
        let dashpay = layout_from_addresses_min_fees(
            IdentityCreationKeyLayout::NewIdentityWithDashPayKeys,
            platform_version,
        )
        .expect("fees");
        assert_eq!(
            base.len(),
            usize::from(platform_version.dpp.state_transitions.max_address_inputs)
        );
        assert!(!base.is_empty());
        assert!(base.windows(2).all(|w| w[0] < w[1]), "each input costs");
        assert!(base.iter().zip(&dashpay).all(|(b, d)| b < d));
    }

    fn resume_of(out_point: OutPoint) -> AssetLockFunding {
        AssetLockFunding::FromExistingAssetLock {
            out_point,
            consume_invitation_voucher: false,
        }
    }

    #[test]
    fn should_refuse_funding_below_the_minimum_for_its_keys() {
        let required = 1_000;
        let fresh = |amount_duffs| AssetLockFunding::FromWalletBalance {
            amount_duffs,
            account_index: 0,
        };
        assert!(matches!(
            apply_identity_create_floor(fresh(required - 1), required, None),
            Err(PlatformWalletError::AssetLockInsufficientFunds { available, required: r })
                if available == required - 1 && r == required
        ));
        assert!(matches!(
            apply_identity_create_floor(fresh(required), required, None),
            Ok(AssetLockFunding::FromWalletBalance { amount_duffs, .. }) if amount_duffs == required
        ));

        // A drain's floor is raised to the minimum, never lowered.
        let drain = |minimum_lock_duffs| AssetLockFunding::DrainAccountBalance {
            account: AssetLockFundingAccount::Bip44 { account_index: 0 },
            minimum_lock_duffs,
        };
        for (floor, expected) in [
            (None, required),
            (Some(1), required),
            (Some(required + 7), required + 7),
        ] {
            assert!(matches!(
                apply_identity_create_floor(drain(floor), required, None),
                Ok(AssetLockFunding::DrainAccountBalance { minimum_lock_duffs: Some(m), .. })
                    if m == expected
            ));
        }

        // A tracked lock, resumed or reclaimed, must cover the minimum too.
        let reclaim = AssetLockFunding::FromExistingAssetLock {
            out_point: OutPoint::default(),
            consume_invitation_voucher: true,
        };
        for existing in [resume_of(OutPoint::default()), reclaim] {
            assert!(matches!(
                apply_identity_create_floor(existing.clone(), required, Some(required - 1)),
                Err(PlatformWalletError::AssetLockInsufficientFunds { available, required: r })
                    if available == required - 1 && r == required
            ));
            assert!(matches!(
                apply_identity_create_floor(existing.clone(), required, Some(required)),
                Ok(AssetLockFunding::FromExistingAssetLock { .. })
            ));
            // An untracked lock is left to the funding resolver.
            assert!(matches!(
                apply_identity_create_floor(existing, required, None),
                Ok(AssetLockFunding::FromExistingAssetLock { .. })
            ));
        }
    }

    /// Signer that must never be reached: the funding floor fires before
    /// anything is derived, built or signed.
    #[derive(Debug)]
    struct UnreachableSigner;

    #[async_trait::async_trait]
    impl CoreSigner for UnreachableSigner {
        type Error = String;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::crypto::ecdsa::EcdsaSignature,
                dashcore::crypto::ecdsa::EcdsaPublicKey,
            ),
            Self::Error,
        > {
            unreachable!("the funding floor must fire before any signing")
        }

        async fn public_key(
            &self,
            _path: &DerivationPath,
        ) -> Result<dashcore::crypto::ecdsa::EcdsaPublicKey, Self::Error> {
            unreachable!("the funding floor must fire before any key derivation")
        }
    }

    #[async_trait::async_trait]
    impl ExtendedPubKeySigner for UnreachableSigner {
        async fn extended_public_key(
            &self,
            _path: &DerivationPath,
        ) -> Result<ExtendedPubKey, Self::Error> {
            unreachable!("the funding floor must fire before any key derivation")
        }
    }

    #[async_trait::async_trait]
    impl IdentitySigner<IdentityPublicKey> for UnreachableSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            unreachable!("the funding floor must fire before any signing")
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            unreachable!("the funding floor must fire before any signing")
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            false
        }
    }

    /// A wallet with a ChainLocked registration lock of `amount_duffs`
    /// tracked at the returned outpoint.
    async fn wallet_with_tracked_lock(
        amount_duffs: u64,
    ) -> (
        Arc<PlatformWallet>,
        OutPoint,
        Arc<PlatformWalletManager<NoopTestPersister>>,
    ) {
        let (manager, wallet_id) = test_platform_wallet_manager().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
        let out_point = OutPoint {
            txid: Txid::from_byte_array([7; 32]),
            vout: 0,
        };
        track_registration_asset_lock(&wallet, out_point, amount_duffs).await;
        (wallet, out_point, manager)
    }

    /// Register `layout`'s keys with `funding`, expecting a refusal before
    /// either signer is reached.
    async fn refused_registration(
        wallet: &PlatformWallet,
        funding: AssetLockFunding,
        layout: IdentityCreationKeyLayout,
    ) -> PlatformWalletError {
        let keys = new_identity(keys_from_layout(layout)).expect("layout");
        wallet
            .identity()
            .register_identity_with_funding(
                funding,
                0,
                keys,
                &UnreachableSigner,
                &UnreachableSigner,
                None,
            )
            .await
            .expect_err("below the minimum for its keys")
    }

    #[tokio::test]
    async fn should_refuse_an_underfunded_fresh_registration_before_building_a_lock() {
        let (manager, wallet_id) = test_platform_wallet_manager().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
        let layout = IdentityCreationKeyLayout::NewIdentityWithDashPayKeys;
        let available = min_duffs(IdentityCreationKeyLayout::NewIdentity);

        let err = refused_registration(
            &wallet,
            AssetLockFunding::FromWalletBalance {
                amount_duffs: available,
                account_index: 0,
            },
            layout,
        )
        .await;
        assert!(matches!(
            err,
            PlatformWalletError::AssetLockInsufficientFunds { available: a, required }
                if a == available && required == min_duffs(layout)
        ));
        assert_eq!(
            wallet
                .identity()
                .identity_creation_min_funding_duffs(layout)
                .expect("fee"),
            min_duffs(layout)
        );
        assert_eq!(
            wallet
                .identity()
                .identity_creation_from_addresses_min_fees(layout)
                .expect("fees"),
            layout_from_addresses_min_fees(layout, PlatformVersion::latest()).expect("fees")
        );
    }

    #[tokio::test]
    async fn should_refuse_to_resume_a_tracked_lock_that_cannot_pay_for_its_keys() {
        // A lock funded for the four base keys cannot register the DashPay
        // set: refused before the finality wait or any signing.
        let layout = IdentityCreationKeyLayout::NewIdentityWithDashPayKeys;
        let available = min_duffs(IdentityCreationKeyLayout::NewIdentity);
        let (wallet, out_point, _manager) = wallet_with_tracked_lock(available).await;

        let err = refused_registration(&wallet, resume_of(out_point), layout).await;
        assert!(matches!(
            err,
            PlatformWalletError::AssetLockInsufficientFunds { available: a, required }
                if a == available && required == min_duffs(layout)
        ));
    }
}
