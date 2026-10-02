use async_trait::async_trait;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::bincode::{Decode, Encode};
use dpp::bls_signatures::{Bls12381G2Impl, SignatureSchemes};
use dpp::dashcore::secp256k1::rand::{RngCore, SeedableRng};
use dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dpp::dashcore::signer;
use dpp::ed25519_dalek::Signer as BlsSigner;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::platform_value::BinaryData;
use dpp::state_transition::errors::InvalidIdentityPublicKeyTypeError;
use dpp::util::hash::ripemd160_sha256;
use dpp::{bls_signatures, dashcore, ed25519_dalek, ProtocolError};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

/// This simple signer is only to be used in tests
#[derive(Default, Clone, PartialEq, Encode, Decode)]
pub struct SimpleSigner {
    /// Private keys is a map from the public key to the Private key bytes
    pub private_keys: BTreeMap<IdentityPublicKey, [u8; 32]>,
    /// Private keys to be added at the end of a block
    pub private_keys_in_creation: BTreeMap<IdentityPublicKey, [u8; 32]>,

    /// Maps address hash (20 bytes) to private key (32 bytes)
    pub address_private_keys: BTreeMap<[u8; 20], [u8; 32]>,
    /// Address private keys to be added at the end of a block
    pub address_private_keys_in_creation: BTreeMap<[u8; 20], [u8; 32]>,
}

impl Debug for SimpleSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleSigner")
            .field(
                "private_keys",
                &self
                    .private_keys
                    .iter()
                    .map(|(k, v)| (k, format!("sk: {}", BASE64_STANDARD.encode(v))))
                    .collect::<BTreeMap<_, _>>(),
            )
            .field(
                "private_keys_in_creation",
                &self
                    .private_keys_in_creation
                    .iter()
                    .map(|(k, v)| (k, format!("sk: {}", BASE64_STANDARD.encode(v))))
                    .collect::<BTreeMap<_, _>>(),
            )
            .finish()
    }
}

/// Errors returned by the seed-based eager-derivation constructors.
#[cfg(feature = "derive")]
#[derive(Debug, thiserror::Error)]
pub enum SimpleSignerError {
    /// The seed produced an invalid root extended private key.
    #[error("invalid seed for root xpriv: {0}")]
    InvalidSeed(String),
    /// The DIP-17 / DIP-9 derivation path failed to construct.
    #[error("derivation path: {0}")]
    DerivationPath(String),
    /// `derive_priv` failed at the given leaf index.
    #[error("derive_priv at index {index}: {message}")]
    DerivePriv {
        /// Leaf index that failed.
        index: u32,
        /// Underlying key-wallet error message.
        message: String,
    },
    /// A leaf [`ChildNumber`] could not be constructed from the requested index.
    #[error("invalid leaf index {index}: {message}")]
    InvalidIndex {
        /// Offending leaf index.
        index: u32,
        /// Underlying key-wallet error message.
        message: String,
    },
}

impl SimpleSigner {
    /// Add a key to the signer
    pub fn add_identity_public_key(
        &mut self,
        public_key: IdentityPublicKey,
        private_key: [u8; 32],
    ) {
        self.private_keys.insert(public_key, private_key);
    }

    /// Add keys to the signer
    pub fn add_identity_public_keys<I: IntoIterator<Item = (IdentityPublicKey, [u8; 32])>>(
        &mut self,
        keys: I,
    ) {
        self.private_keys.extend(keys)
    }

    /// Add a key to the signer
    pub fn add_address_key(&mut self, address: [u8; 20], private_key: [u8; 32]) {
        self.address_private_keys.insert(address, private_key);
    }

    /// Add keys to the signer
    pub fn add_address_keys<I: IntoIterator<Item = ([u8; 20], [u8; 32])>>(&mut self, keys: I) {
        self.address_private_keys_in_creation.extend(keys)
    }

    /// Commit keys in creation
    pub fn commit_block_keys(&mut self) {
        self.private_keys.append(&mut self.private_keys_in_creation);
        self.address_private_keys
            .append(&mut self.address_private_keys_in_creation);
    }

    /// Generate a new random ECDSA keypair and corresponding P2PKH address hash,
    /// store the private key so this signer can use it, and return the PlatformAddress.
    ///
    /// This is only for tests.
    pub fn add_random_address_key<R: RngCore + ?Sized>(&mut self, rng: &mut R) -> PlatformAddress {
        let secp = Secp256k1::new();

        // Generate a valid secp256k1 secret key from random bytes
        let mut ecdsa_rng = dashcore::secp256k1::rand::rngs::StdRng::from_rng(rng).unwrap();
        let secret_key = SecretKey::new(&mut ecdsa_rng);

        // Derive compressed public key
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let pubkey_ser = public_key.serialize(); // 33-byte compressed

        let address_hash = ripemd160_sha256(&pubkey_ser);

        // Store private key so this signer can later sign for this address
        // (use *_in_creation to mirror your identity key behavior)
        self.address_private_keys_in_creation
            .insert(address_hash, secret_key.secret_bytes());

        PlatformAddress::P2pkh(address_hash)
    }

    /// Build a [`SimpleSigner`] populated with the DIP-17 platform-payment
    /// gap window for `(account, key_class)`. Each leaf
    /// `m/9'/coin_type'/17'/account'/key_class'/index` derives a
    /// secp256k1 keypair; the 20-byte RIPEMD160(SHA256(pubkey)) hash is
    /// inserted into [`Self::address_private_keys`].
    #[cfg(feature = "derive")]
    pub fn from_seed_for_platform_address_account(
        seed: &[u8; 64],
        network: key_wallet::Network,
        account: u32,
        key_class: u32,
        gap_limit: u32,
    ) -> Result<Self, SimpleSignerError> {
        Self::from_seed_for_platform_addresses(seed, network, account, key_class, 0..gap_limit)
    }

    /// Build a [`SimpleSigner`] populated with the DIP-17 platform-payment
    /// keys for an explicit set of derivation `indices` under
    /// `(account, key_class)`. Each `index` derives
    /// `m/9'/coin_type'/17'/account'/key_class'/index`; the 20-byte
    /// RIPEMD160(SHA256(pubkey)) hash is inserted into
    /// [`Self::address_private_keys`].
    ///
    /// Use this over [`Self::from_seed_for_platform_address_account`] when
    /// the address set is not the contiguous `0..gap_limit` window — e.g.
    /// a long-lived wallet whose synced funded pool has cycled past the
    /// first gap window. Duplicate indices are deduplicated by the
    /// underlying key map. Returns the first derivation error encountered.
    #[cfg(feature = "derive")]
    pub fn from_seed_for_platform_addresses<I: IntoIterator<Item = u32>>(
        seed: &[u8; 64],
        network: key_wallet::Network,
        account: u32,
        key_class: u32,
        indices: I,
    ) -> Result<Self, SimpleSignerError> {
        use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
        use key_wallet::{AccountType, ChildNumber};

        let root_priv = RootExtendedPrivKey::new_master(seed)
            .map_err(|err| SimpleSignerError::InvalidSeed(err.to_string()))?;
        let root_xpriv = root_priv.to_extended_priv_key(network);

        let account_path = AccountType::PlatformPayment { account, key_class }
            .derivation_path(network)
            .map_err(|err| SimpleSignerError::DerivationPath(err.to_string()))?;

        let secp = Secp256k1::new();
        let mut signer = Self::default();
        for index in indices {
            let leaf = ChildNumber::from_normal_idx(index).map_err(|err| {
                SimpleSignerError::InvalidIndex {
                    index,
                    message: err.to_string(),
                }
            })?;
            // `extend` returns a fresh path; account_path is reused.
            let leaf_path = account_path.extend([leaf]);
            let xpriv = root_xpriv.derive_priv(&secp, &leaf_path).map_err(|err| {
                SimpleSignerError::DerivePriv {
                    index,
                    message: err.to_string(),
                }
            })?;
            let secret: SecretKey = xpriv.private_key;
            let pubkey: PublicKey = PublicKey::from_secret_key(&secp, &secret);
            let pkh = ripemd160_sha256(&pubkey.serialize());
            signer
                .address_private_keys
                .insert(pkh, secret.secret_bytes());
        }
        Ok(signer)
    }

    /// Build a [`SimpleSigner`] populated with the DIP-9 identity-authentication
    /// (ECDSA) gap window for `identity_index`. The returned signer holds raw
    /// secp256k1 secrets keyed on `(pubkey-hash, secret)` via
    /// [`Self::address_private_keys`] — callers that need a `Signer<IdentityPublicKey>`
    /// view must additionally register `IdentityPublicKey` records via
    /// [`Self::add_identity_public_key`] using the matching pubkey bytes.
    #[cfg(feature = "derive")]
    pub fn from_seed_for_identity(
        seed: &[u8; 64],
        network: key_wallet::Network,
        identity_index: u32,
        gap_limit: u32,
    ) -> Result<Self, SimpleSignerError> {
        use key_wallet::bip32::KeyDerivationType;
        use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
        use key_wallet::DerivationPath;

        let root_priv = RootExtendedPrivKey::new_master(seed)
            .map_err(|err| SimpleSignerError::InvalidSeed(err.to_string()))?;
        let root_xpriv = root_priv.to_extended_priv_key(network);

        let secp = Secp256k1::new();
        let mut signer = Self::default();
        for key_index in 0..gap_limit {
            let leaf_path = DerivationPath::identity_authentication_path(
                network,
                KeyDerivationType::ECDSA,
                identity_index,
                key_index,
            );
            let xpriv = root_xpriv.derive_priv(&secp, &leaf_path).map_err(|err| {
                SimpleSignerError::DerivePriv {
                    index: key_index,
                    message: err.to_string(),
                }
            })?;
            let secret: SecretKey = xpriv.private_key;
            let pubkey: PublicKey = PublicKey::from_secret_key(&secp, &secret);
            let pkh = ripemd160_sha256(&pubkey.serialize());
            signer
                .address_private_keys
                .insert(pkh, secret.secret_bytes());
        }
        Ok(signer)
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for SimpleSigner {
    async fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key = self
            .private_keys
            .get(identity_public_key)
            .or_else(|| self.private_keys_in_creation.get(identity_public_key))
            .ok_or(ProtocolError::Generic(format!(
                "{:?} not found in {:?}",
                identity_public_key, self
            )))?;
        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(data, private_key)?;
                Ok(signature.to_vec().into())
            }
            KeyType::BLS12_381 => {
                let pk = bls_signatures::SecretKey::<Bls12381G2Impl>::from_be_bytes(private_key)
                    .into_option()
                    .ok_or(ProtocolError::Generic(
                        "bls private key from bytes isn't correct".to_string(),
                    ))?;
                let signature = pk
                    .sign(SignatureSchemes::Basic, data)
                    .map_err(|e| ProtocolError::Generic(format!("BLS signing failed {}", e)))?;
                Ok(signature.as_raw_value().to_compressed().to_vec().into())
            }
            KeyType::EDDSA_25519_HASH160 => {
                #[allow(clippy::unnecessary_fallible_conversions)]
                let pk = ed25519_dalek::SigningKey::try_from(private_key).map_err(|_e| {
                    ProtocolError::Generic(
                        "eddsa 25519 private key from bytes isn't correct".to_string(),
                    )
                })?;
                Ok(pk.sign(data).to_vec().into())
            }
            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH => Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(identity_public_key.key_type()),
            )),
        }
    }

    async fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        // First, sign the data to get the signature
        let signature = self.sign(key, data).await?;

        // Create the appropriate AddressWitness based on the key type
        match key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                // P2PKH witness only needs the signature - the public key is recovered
                // during verification, saving 33 bytes per witness
                Ok(AddressWitness::P2pkh { signature })
            }
            KeyType::EDDSA_25519_HASH160 => {
                // Ed25519 keys are not supported for address witnesses (P2PKH requires ECDSA)
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(key.key_type()),
                ))
            }
            KeyType::BIP13_SCRIPT_HASH => {
                // For script hash, we would need the redeem script which isn't available from just the key
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(key.key_type()),
                ))
            }
            KeyType::BLS12_381 => {
                // BLS keys are not supported for address witnesses
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(key.key_type()),
                ))
            }
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        self.private_keys
            .get(identity_public_key)
            .or_else(|| self.private_keys_in_creation.get(identity_public_key))
            .is_some()
    }
}

#[async_trait]
impl Signer<PlatformAddress> for SimpleSigner {
    async fn sign(
        &self,
        address: &PlatformAddress,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let hash = match address {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(_) => {
                return Err(ProtocolError::Generic(
                    "P2SH addresses not supported".to_string(),
                ));
            }
        };
        let private_key = self
            .address_private_keys
            .get(hash)
            .or_else(|| self.address_private_keys_in_creation.get(hash))
            .ok_or(format!("No private key found for address {:?}", address))?;

        let signature = signer::sign(data, private_key)?;
        Ok(signature.to_vec().into())
    }

    async fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        // First, sign the data to get the signature
        let signature = self.sign(key, data).await?;
        match key {
            PlatformAddress::P2pkh(_) => Ok(AddressWitness::P2pkh { signature }),
            PlatformAddress::P2sh(_) => Err(ProtocolError::Generic(
                "P2SH addresses not supported".to_string(),
            )),
        }
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        match key {
            PlatformAddress::P2pkh(hash) => self
                .address_private_keys
                .get(hash)
                .or_else(|| self.address_private_keys_in_creation.get(hash))
                .is_some(),
            PlatformAddress::P2sh(_) => false,
        }
    }
}
