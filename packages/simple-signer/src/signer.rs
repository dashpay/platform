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

/// A simple signer implementation for signing operations with identity keys.
///
/// This signer stores private keys in memory and can be used for both testing
/// and production scenarios where convenience methods are preferred.
#[derive(Default, Clone, PartialEq, Encode, Decode)]
pub struct SimpleSigner {
    /// Private keys is a map from the public key to the Private key bytes
    pub private_keys: BTreeMap<IdentityPublicKey, [u8; 32]>,
    /// Private keys to be added at the end of a block
    pub private_keys_in_creation: BTreeMap<IdentityPublicKey, [u8; 32]>,

    /// Maps address hash (20 bytes) to private key (32 bytes)
    pub address_private_keys: BTreeMap<[u8; 20], [u8; 32]>,
    /// Addres private keys to be added at the end of a block
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

    /// Add a key from a WIF-encoded private key string.
    ///
    /// This method parses the WIF string and adds the key to the signer.
    ///
    /// # Arguments
    ///
    /// * `identity_public_key` - The identity public key associated with this private key
    /// * `wif` - The WIF-encoded private key string
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the key was added successfully, or an error if WIF parsing fails.
    pub fn add_key_from_wif(
        &mut self,
        identity_public_key: IdentityPublicKey,
        wif: &str,
    ) -> Result<(), ProtocolError> {
        let private_key = dashcore::PrivateKey::from_wif(wif)
            .map_err(|e| ProtocolError::Generic(format!("Invalid WIF private key: {}", e)))?;
        self.add_identity_public_key(identity_public_key, private_key.inner.secret_bytes());
        Ok(())
    }

    /// Create a new SimpleSigner with a single key loaded from WIF.
    ///
    /// This is a convenience method for creating a signer with a single key.
    ///
    /// # Arguments
    ///
    /// * `identity_public_key` - The identity public key associated with this private key
    /// * `wif` - The WIF-encoded private key string
    ///
    /// # Returns
    ///
    /// Returns a new `SimpleSigner` with the key loaded, or an error if WIF parsing fails.
    pub fn from_wif(
        identity_public_key: IdentityPublicKey,
        wif: &str,
    ) -> Result<Self, ProtocolError> {
        let mut signer = Self::default();
        signer.add_key_from_wif(identity_public_key, wif)?;
        Ok(signer)
    }

    /// Create a new SimpleSigner with a single key from raw bytes.
    ///
    /// This is a convenience method for creating a signer with a single key
    /// when you already have the private key as raw bytes.
    ///
    /// # Arguments
    ///
    /// * `identity_public_key` - The identity public key associated with this private key
    /// * `private_key` - The 32-byte private key
    ///
    /// # Returns
    ///
    /// Returns a new `SimpleSigner` with the key loaded.
    pub fn from_private_key(
        identity_public_key: IdentityPublicKey,
        private_key: [u8; 32],
    ) -> Self {
        let mut signer = Self::default();
        signer.add_identity_public_key(identity_public_key, private_key);
        signer
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
}

impl Signer<IdentityPublicKey> for SimpleSigner {
    fn sign(
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

    fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        // First, sign the data to get the signature
        let signature = self.sign(key, data)?;

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

impl Signer<PlatformAddress> for SimpleSigner {
    fn sign(&self, address: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
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

    fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        // First, sign the data to get the signature
        let signature = self.sign(key, data)?;
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
