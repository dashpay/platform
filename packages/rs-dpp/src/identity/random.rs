use crate::identity::v0::IdentityV0;
use crate::identity::{Identity, IdentityPublicKey, KeyCount};
use crate::version::PlatformVersion;
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
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityV0::random_identity_with_rng(key_count, rng, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::random_identity_with_rng".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, I), ProtocolError>
    where
        I: Default
            + IntoIterator<Item = (IdentityPublicKey, [u8; 32])>
            + Extend<(IdentityPublicKey, [u8; 32])>,
    {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => IdentityV0::random_identity_with_main_keys_with_private_key(
                key_count,
                rng,
                platform_version,
            )
            .map(|(a, b)| (a.into(), b)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::random_identity_with_main_keys_with_private_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
    pub fn random_identity(
        key_count: KeyCount,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityV0::random_identity(key_count, seed, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::random_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
        count: u16,
        key_count: KeyCount,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Self>, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(
                IdentityV0::random_identities(count, key_count, seed, platform_version)?
                    .into_iter()
                    .map(|identity| identity.into())
                    .collect(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::random_identities".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Self>, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityV0::random_identities_with_rng(
                count,
                key_count,
                rng,
                platform_version,
            )?
            .into_iter()
            .map(|identity| identity.into())
            .collect()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::random_identities_with_rng".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Self>, I), ProtocolError>
    where
        I: Default
            + FromIterator<(IdentityPublicKey, [u8; 32])>
            + Extend<(IdentityPublicKey, [u8; 32])>,
    {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => IdentityV0::random_identities_with_private_keys_with_rng(
                count,
                key_count,
                rng,
                platform_version,
            )
            .map(|(identities, keys)| {
                (
                    identities
                        .into_iter()
                        .map(|identity| identity.into())
                        .collect(),
                    keys,
                )
            }),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::random_identities_with_private_keys_with_rng".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
