use crate::identity::identity_public_key::public_key_factory::KeyCount;
use crate::identity::v0::identity::IdentityV0;
use crate::identity::{Identity, IdentityPublicKey};
use crate::version::FeatureVersion;
use crate::ProtocolError;
use rand::prelude::StdRng;
use std::iter::FromIterator;

impl Identity {
    /// Generates a random identity using the specified version and key count, with a provided RNG.
    ///
    /// # Arguments
    ///
    /// * `version` - An optional feature version for the identity. If not provided, the latest version will be used.
    /// * `key_count` - The number of keys to generate for the identity.
    /// * `rng` - A mutable reference to a random number generator to use for generating the identity.
    ///
    /// # Returns
    ///
    /// A randomly generated identity of the specified version.
    ///
    /// # Panics
    ///
    /// This function will panic if an unsupported version is provided.
    pub fn random_identity_with_rng(
        version: Option<FeatureVersion>,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Self {
        let version = version.unwrap_or(Self::latest_version());
        match version {
            0 => IdentityV0::random_identity_with_rng(key_count, rng).into(),
            _ => panic!(),
        }
    }

    /// Generates a random identity with main keys and their corresponding private keys, using the specified version and key count, with a provided RNG.
    ///
    /// # Arguments
    ///
    /// * `version` - An optional feature version for the identity. If not provided, the latest version will be used.
    /// * `key_count` - The number of keys to generate for the identity.
    /// * `rng` - A mutable reference to a random number generator to use for generating the identity.
    ///
    /// # Returns
    ///
    /// A tuple containing the randomly generated identity and a collection of main keys with their corresponding private keys.
    ///
    /// # Panics
    ///
    /// This function will panic if an unsupported version is provided.
    ///
    /// # Errors
    ///
    /// This function may return a `ProtocolError` if an error occurs during identity generation.
    pub fn random_identity_with_main_keys_with_private_key<I>(
        version: Option<u16>,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Result<(Self, I), ProtocolError>
    where
        I: Default
            + IntoIterator<Item = (IdentityPublicKey, Vec<u8>)>
            + Extend<(IdentityPublicKey, Vec<u8>)>,
    {
        let version = version.unwrap_or(Self::latest_version());
        match version {
            0 => IdentityV0::random_identity_with_main_keys_with_private_key(key_count, rng).into(),
            _ => panic!(),
        }
    }

    /// Generates a random identity using the specified version and key count, with an optional seed for reproducibility.
    ///
    /// # Arguments
    ///
    /// * `version` - An optional feature version for the identity. If not provided, the latest version will be used.
    /// * `key_count` - The number of keys to generate for the identity.
    /// * `seed` - An optional seed for reproducibility. If provided, the RNG will be seeded with this value.
    ///
    /// # Returns
    ///
    /// A randomly generated identity of the specified version.
    ///
    /// # Panics
    ///
    /// This function will panic if an unsupported version is provided.
    pub fn random_identity(version: Option<u16>, key_count: KeyCount, seed: Option<u64>) -> Self {
        let version = version.unwrap_or(Self::latest_version());
        match version {
            0 => IdentityV0::random_identity(key_count, seed).into(),
            _ => panic!(),
        }
    }

    /// Generates a specified number of random identities using the specified version and key count, with an optional seed for reproducibility.
    ///
    /// # Arguments
    ///
    /// * `version` - An optional feature version for the identity. If not provided, the latest version will be used.
    /// * `count` - The number of identities to generate.
    /// * `key_count` - The number of keys to generate for each identity.
    /// * `seed` - An optional seed for reproducibility. If provided, the RNG will be seeded with this value.
    ///
    /// # Returns
    ///
    /// A vector of randomly generated identities of the specified version.
    ///
    /// # Panics
    ///
    /// This function will panic if an unsupported version is provided.
    pub fn random_identities(
        version: Option<u16>,
        count: u16,
        key_count: KeyCount,
        seed: Option<u64>,
    ) -> Vec<Self> {
        let version = version.unwrap_or(Self::latest_version());
        match version {
            0 => IdentityV0::random_identities(count, key_count, seed).into(),
            _ => panic!(),
        }
    }

    /// Generates a specified number of random identities using the specified version and key count, with a provided RNG.
    ///
    /// # Arguments
    /// * `version` - An optional feature version for the identity. If not provided, the latest version will be used.
    /// * `count` - The number of identities to generate.
    /// * `key_count` - The number of keys to generate for each identity.
    /// * `rng` - A mutable reference to a random number generator to use for generating the identities.
    ///
    /// # Returns
    ///
    /// A vector of randomly generated identities of the specified version.
    ///
    /// # Panics
    ///
    /// This function will panic if an unsupported version is provided.
    pub fn random_identities_with_rng(
        version: Option<u16>,
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Vec<Self> {
        let version = version.unwrap_or(Self::latest_version());
        match version {
            0 => IdentityV0::random_identities_with_rng(count, key_count, rng).into(),
            _ => panic!(),
        }
    }

    /// Generates a specified number of random identities with their corresponding private keys, using the specified version, key count, and a provided RNG.
    ///
    /// # Arguments
    ///
    /// * `version` - An optional feature version for the identity. If not provided, the latest version will be used.
    /// * `count` - The number of identities to generate.
    /// * `key_count` - The number of keys to generate for each identity.
    /// * `rng` - A mutable reference to a random number generator to use for generating the identities.
    ///
    /// # Returns
    ///
    /// A tuple containing a vector of randomly generated identities and a collection of main keys with their corresponding private keys.
    ///
    /// # Panics
    ///
    /// This function will panic if an unsupported version is provided.
    ///
    /// # Errors
    ///
    /// This function may return a `ProtocolError` if an error occurs during identity generation.
    pub fn random_identities_with_private_keys_with_rng<I>(
        version: Option<u16>,
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Result<(Vec<Self>, I), ProtocolError>
    where
        I: Default
            + FromIterator<(IdentityPublicKey, Vec<u8>)>
            + Extend<(IdentityPublicKey, Vec<u8>)>,
    {
        let version = version.unwrap_or(Self::latest_version());
        match version {
            0 => IdentityV0::random_identities_with_private_keys_with_rng(count, key_count, rng)
                .into(),
            _ => panic!(),
        }
    }
}
