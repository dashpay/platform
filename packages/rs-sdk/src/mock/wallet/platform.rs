//! Platform wallet using [SimpleSigner].
use std::collections::BTreeMap;

use dashcore_rpc::dashcore::secp256k1::rand::{rngs::StdRng, SeedableRng};
use dpp::{
    identity::{
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer,
        IdentityPublicKey, KeyID, Purpose,
    },
    platform_value::BinaryData,
    version::{PlatformVersion, PlatformVersionCurrentVersion},
    ProtocolError,
};
use simple_signer::signer::SimpleSigner;

use crate::{wallet::PlatformWallet, Error};

/// Mock Platform wallet using [SimpleSigner].
#[derive(Debug)]
pub struct PlatformSignerWallet {
    /// Signer used to sign Platform transactions.
    pub signer: SimpleSigner,

    /// Default keys used to sign Platform transactions for each purpose.
    pub default_keys: BTreeMap<dpp::identity::Purpose, KeyID>,
}

impl PlatformSignerWallet {
    /// Create new mock Platform wallet using [SimpleSigner].
    ///
    /// This method generates new keys and sets first authentication key as default.
    /// Note the key will always be the same, so it's not suitable for production use.
    pub fn new_mock() -> Result<Self, Error> {
        let platform_version = PlatformVersion::get_current()?; // TODO pass as arg
                                                                // it's a mock, so we always want the same key
        const SEED: [u8; 32] = [0u8; 32];
        let mut rng = StdRng::from_seed(SEED);
        // let privkey = PrivateKey::generate_dash(&mut rng).map_err(|e| Error::from(e))?;

        // TODO: also generate other types of keys
        let keys = IdentityPublicKey::random_authentication_keys_with_private_keys_with_rng(
            0 as KeyID,
            6,
            &mut rng,
            platform_version,
        )?;
        // .map_err(|e| Error::Protocol(ProtocolError::PublicKeyGenerationError(e.to_string())))?;

        let mut signer = SimpleSigner::default();
        signer.add_keys(keys);

        Self::try_from(signer)
    }
}

impl TryFrom<SimpleSigner> for PlatformSignerWallet {
    type Error = Error;
    fn try_from(signer: SimpleSigner) -> Result<Self, Self::Error> {
        // get first authentication key and set it as default
        let (first_key, _) = signer
            .private_keys
            .iter()
            .find(|(key, _)| key.purpose() == Purpose::AUTHENTICATION)
            .ok_or(Error::Protocol(ProtocolError::PublicKeyGenerationError(
                "At least one key is required in Signer".to_string(),
            )))?;

        let mut default_keys: BTreeMap<Purpose, KeyID> = BTreeMap::new();
        default_keys.insert(first_key.purpose(), first_key.id());

        Ok(Self {
            signer,
            default_keys,
        })
    }
}

impl Signer for PlatformSignerWallet {
    fn sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.signer.sign(pubkey, message)
    }
}

impl PlatformWallet for PlatformSignerWallet {
    fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey> {
        let id = self.default_keys.get(purpose)?;

        let (key, _) = self
            .signer
            .private_keys
            .iter()
            .find(|(key, _)| key.id() == *id)?;

        Some(key.clone())
    }
}
