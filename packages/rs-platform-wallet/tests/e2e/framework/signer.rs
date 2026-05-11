//! Seed-backed `Signer<IdentityPublicKey>` for the e2e harness, plus a
//! [`derive_identity_key`] helper for building placeholder identity keys.
//!
//! Identities use DIP-9
//! (`m/9'/coin_type'/5'/0'/ECDSA'/identity_index'/key_index'`).
//!
//! Note: `Signer<PlatformAddress>` is provided directly by `SimpleSigner`
//! (built via `super::make_platform_signer`) and no longer needs a wrapper.

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::dashcore::signer as core_signer;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::util::hash::ripemd160_sha256;
use dpp::ProtocolError;
use key_wallet::Network;
use simple_signer::signer::SimpleSigner;

use super::{FrameworkError, FrameworkResult};

/// Default gap window pre-derived at construction
/// (matches `key-wallet`'s `DIP17_GAP_LIMIT`).
pub const DEFAULT_GAP_LIMIT: u32 = 20;

/// Seed-backed [`Signer<IdentityPublicKey>`] for one DIP-9 identity slot.
///
/// Composes [`SimpleSigner::from_seed_for_identity`], which populates
/// `inner.address_private_keys` with `(ripemd160_sha256(pubkey), secret)`
/// pairs for `key_index ∈ 0..gap_limit`. The trait impl looks up by
/// hashing the [`IdentityPublicKey::data`] field — matching the same
/// hash used at construction.
#[derive(Clone, Debug)]
pub struct SeedBackedIdentitySigner {
    inner: SimpleSigner,
    identity_index: u32,
}

impl SeedBackedIdentitySigner {
    /// Build a signer for the DIP-9 identity at `identity_index`,
    /// pre-deriving `key_index ∈ 0..DEFAULT_GAP_LIMIT` ECDSA auth keys.
    pub fn new(
        seed_bytes: &[u8; 64],
        network: Network,
        identity_index: u32,
    ) -> FrameworkResult<Self> {
        Self::new_with_gap(seed_bytes, network, identity_index, DEFAULT_GAP_LIMIT)
    }

    /// Same as [`Self::new`] with an explicit gap window. The window
    /// counts identity-key indices, not address indices.
    pub fn new_with_gap(
        seed_bytes: &[u8; 64],
        network: Network,
        identity_index: u32,
        gap_limit: u32,
    ) -> FrameworkResult<Self> {
        let inner =
            SimpleSigner::from_seed_for_identity(seed_bytes, network, identity_index, gap_limit)
                .map_err(|err| {
                    FrameworkError::Wallet(format!("SeedBackedIdentitySigner: {err}"))
                })?;
        Ok(Self {
            inner,
            identity_index,
        })
    }

    /// DIP-9 identity index this signer is bound to.
    pub fn identity_index(&self) -> u32 {
        self.identity_index
    }

    /// Number of pre-derived identity keys currently in the cache.
    pub fn cached_key_count(&self) -> usize {
        self.inner.address_private_keys.len()
    }

    /// Insert a freshly-derived identity-key secret into the inner
    /// [`SimpleSigner`]'s `address_private_keys` cache so subsequent
    /// `Signer<IdentityPublicKey>` calls can resolve the matching
    /// [`IdentityPublicKey`].
    ///
    /// Used by the ID-004 key-rotation helper after a new auth key
    /// has been derived via [`derive_identity_key`] outside the
    /// initial gap window. `public_key` must be the 33-byte
    /// compressed `secp256k1::PublicKey` produced alongside `secret`
    /// — the cache is keyed on `ripemd160_sha256(pubkey)`, mirroring
    /// the construction-time pre-population in
    /// [`SimpleSigner::from_seed_for_identity`].
    pub fn inject_identity_key(&mut self, public_key: &[u8; 33], secret: [u8; 32]) {
        let pkh = ripemd160_sha256(public_key.as_slice());
        self.inner.address_private_keys.insert(pkh, secret);
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for SeedBackedIdentitySigner {
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        match key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {}
            other => {
                return Err(ProtocolError::Generic(format!(
                    "SeedBackedIdentitySigner: unsupported key type {other:?}"
                )));
            }
        }
        let secret = lookup_identity_secret(&self.inner, key)?;
        let signature = core_signer::sign(data, &secret)?;
        Ok(signature.to_vec().into())
    }

    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        // Identity-key signers never produce platform-address witnesses —
        // the DPP signer trait forces both methods on a single impl.
        Err(ProtocolError::Generic(
            "SeedBackedIdentitySigner: AddressWitness is not produced by an identity signer".into(),
        ))
    }

    fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
        match identity_key_lookup(key) {
            Some(pkh) => self.inner.address_private_keys.contains_key(&pkh),
            None => false,
        }
    }
}

/// Compute the `address_private_keys` lookup key for an
/// [`IdentityPublicKey`].
///
/// `SimpleSigner::from_seed_for_identity` keys its cache by
/// `ripemd160_sha256(compressed_pubkey)` — so for `ECDSA_SECP256K1` we
/// hash `key.data()` (the raw pubkey), but for `ECDSA_HASH160`
/// `key.data()` is **already** the 20-byte hash and re-hashing would
/// produce `hash160(hash160(pubkey))`, which would never match.
/// Returns `None` for unsupported key types.
fn identity_key_lookup(key: &IdentityPublicKey) -> Option<[u8; 20]> {
    match key.key_type() {
        KeyType::ECDSA_SECP256K1 => Some(ripemd160_sha256(key.data().as_slice())),
        KeyType::ECDSA_HASH160 => key.data().as_slice().try_into().ok(),
        _ => None,
    }
}

/// Resolve an [`IdentityPublicKey`] to its pre-derived 32-byte secret,
/// or surface a [`ProtocolError`] naming the missing fingerprint.
#[allow(clippy::result_large_err)]
fn lookup_identity_secret(
    inner: &SimpleSigner,
    key: &IdentityPublicKey,
) -> Result<[u8; 32], ProtocolError> {
    let pkh = identity_key_lookup(key).ok_or_else(|| {
        ProtocolError::Generic(format!(
            "SeedBackedIdentitySigner: unsupported key type {:?}",
            key.key_type()
        ))
    })?;
    inner
        .address_private_keys
        .get(&pkh)
        .copied()
        .ok_or_else(|| {
            ProtocolError::Generic(format!(
                "SeedBackedIdentitySigner: identity key {} not in pre-derived gap window",
                hex::encode(pkh)
            ))
        })
}

/// Build a fully-formed [`IdentityPublicKey`] for a placeholder
/// identity at the DIP-9 slot
/// `m/9'/coin_type'/5'/0'/ECDSA'/identity_index'/key_index'`.
///
/// Top-level helper — not bound to a [`SeedBackedIdentitySigner`]
/// instance — so call sites can build a placeholder identity from a
/// seed without instantiating the signer first. The returned key has
/// `id = key_index as KeyID` (the canonical convention at
/// registration — DPP assigns key ids sequentially starting at 0),
/// `read_only = false`, `disabled_at = None`, `contract_bounds = None`,
/// `key_type = ECDSA_SECP256K1` (the only DIP-9 derivation type this
/// helper supports).
pub fn derive_identity_key(
    seed: &[u8; 64],
    network: Network,
    identity_index: u32,
    key_index: u32,
    purpose: Purpose,
    security_level: SecurityLevel,
) -> FrameworkResult<IdentityPublicKey> {
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
    use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;

    let root_priv = RootExtendedPrivKey::new_master(seed).map_err(|err| {
        FrameworkError::Wallet(format!(
            "derive_identity_key: invalid seed for root xpriv: {err}"
        ))
    })?;
    let master = root_priv.to_extended_priv_key(network);
    let derived =
        derive_ecdsa_identity_auth_keypair_from_master(&master, network, identity_index, key_index)
            .map_err(|err| {
                FrameworkError::Wallet(format!(
                    "derive_identity_key: derive ({identity_index}, {key_index}): {err}"
                ))
            })?;
    let v0 = IdentityPublicKeyV0 {
        id: key_index as KeyID,
        purpose,
        security_level,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(derived.public_key.to_vec()),
        disabled_at: None,
    };
    Ok(IdentityPublicKey::V0(v0))
}
