use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;

use crate::identity::{IdentityPublicKey, KeyCount, KeyID, KeyType, Purpose, SecurityLevel};
use crate::version::PlatformVersion;
use crate::ProtocolError;

use rand::rngs::StdRng;
use rand::SeedableRng;

pub type UsedKeyMatrix = Vec<bool>;

impl IdentityPublicKey {
    pub fn random_key(id: KeyID, seed: Option<u64>, platform_version: &PlatformVersion) -> Self {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_key_with_rng(id, &mut rng, None, platform_version).unwrap()
    }

    pub fn random_keys(
        first_id: KeyID,
        count: KeyCount,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Vec<Self> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let end_id = first_id + count;
        (first_id..end_id)
            .map(|key_id| {
                Self::random_key_with_rng(key_id, &mut rng, None, platform_version).unwrap()
            })
            .collect()
    }

    pub fn random_authentication_key(
        key_id: KeyID,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Self {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_authentication_key_with_rng(key_id, &mut rng, None, platform_version).unwrap()
    }

    pub fn random_authentication_keys(
        first_id: KeyID,
        count: KeyCount,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Vec<Self> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let end_id = first_id + count;
        (first_id..end_id)
            .map(|key_id| {
                Self::random_authentication_key_with_rng(key_id, &mut rng, None, platform_version)
                    .unwrap()
            })
            .collect()
    }

    /// Generates a random authentication key based on the platform version.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to a random number generator of type `StdRng`.
    /// * `used_key_matrix`: An optional tuple that contains the count of keys that have already been used
    ///                      and a mutable reference to a matrix (or vector) that tracks which keys have been used.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>`: If successful, returns an instance of `Self`.
    ///                                  In case of an error, it returns a `ProtocolError`.
    ///
    /// # Errors
    ///
    /// * `ProtocolError::PublicKeyGenerationError`: This error is returned if too many keys have already been created.
    /// * `ProtocolError::UnknownVersionMismatch`: This error is returned if the provided platform version is not recognized.
    ///
    pub fn random_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyV0::random_authentication_key_with_rng(
                id,
                rng,
                used_key_matrix,
                platform_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::random_authentication_key_with_rng".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Generates a random authentication key and its corresponding private key based on the platform version.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `seed`: A seed that will create a random number generator `StdRng`.
    /// * `used_key_matrix`: An optional tuple that contains the count of keys that have already been used
    ///                      and a mutable reference to a matrix (or vector) that tracks which keys have been used.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Result<(Self, Vec<u8>), ProtocolError>`: If successful, returns an instance of `Self` and the private key as `Vec<u8>`.
    ///                                  In case of an error, it returns a `ProtocolError`.
    ///
    /// # Errors
    ///
    /// * `ProtocolError::PublicKeyGenerationError`: This error is returned if too many keys have already been created.
    /// * `ProtocolError::UnknownVersionMismatch`: This error is returned if the provided platform version is not recognized.
    ///
    pub fn random_authentication_key_with_private_key(
        id: KeyID,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_authentication_key_with_private_key_with_rng(
            id,
            &mut rng,
            None,
            platform_version,
        )
    }

    /// Generates a random authentication key and its corresponding private key based on the platform version.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to a random number generator of type `StdRng`.
    /// * `used_key_matrix`: An optional tuple that contains the count of keys that have already been used
    ///                      and a mutable reference to a matrix (or vector) that tracks which keys have been used.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Result<(Self, Vec<u8>), ProtocolError>`: If successful, returns an instance of `Self` and the private key as `Vec<u8>`.
    ///                                  In case of an error, it returns a `ProtocolError`.
    ///
    /// # Errors
    ///
    /// * `ProtocolError::PublicKeyGenerationError`: This error is returned if too many keys have already been created.
    /// * `ProtocolError::UnknownVersionMismatch`: This error is returned if the provided platform version is not recognized.
    ///
    pub fn random_authentication_key_with_private_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => IdentityPublicKeyV0::random_authentication_key_with_private_key_with_rng(
                id,
                rng,
                used_key_matrix,
                platform_version,
            )
            .map(|(key, private_key)| (key.into(), private_key)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::random_authentication_key_with_private_key_with_rng"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Generates a random key based on the platform version.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to a random number generator of type `StdRng`.
    /// * `used_key_matrix`: An optional tuple that contains the count of keys that have already been used
    ///                      and a mutable reference to a matrix (or vector) that tracks which keys have been used.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>`: If successful, returns an instance of `Self`.
    ///                                  In case of an error, it returns a `ProtocolError`.
    ///
    /// # Errors
    ///
    /// * `ProtocolError::PublicKeyGenerationError`: This error is returned if too many keys have already been created.
    /// * `ProtocolError::UnknownVersionMismatch`: This error is returned if the provided platform version is not recognized.
    ///
    pub fn random_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyV0::random_key_with_rng(
                id,
                rng,
                used_key_matrix,
                platform_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::random_key_with_rng".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Generates a random key based on the platform version.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to a random number generator of type `StdRng`.
    /// * `used_key_matrix`: An optional tuple that contains the count of keys that have already been used
    ///                      and a mutable reference to a matrix (or vector) that tracks which keys have been used.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>`: If successful, returns an instance of `Self`.
    ///                                  In case of an error, it returns a `ProtocolError`.
    ///
    /// # Errors
    ///
    /// * `ProtocolError::PublicKeyGenerationError`: This error is returned if too many keys have already been created.
    /// * `ProtocolError::UnknownVersionMismatch`: This error is returned if the provided platform version is not recognized.
    ///
    pub fn random_key_with_known_attributes(
        id: KeyID,
        rng: &mut StdRng,
        purpose: Purpose,
        security_level: SecurityLevel,
        key_type: KeyType,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => {
                let (key, private_key) = IdentityPublicKeyV0::random_key_with_known_attributes(
                    id,
                    rng,
                    purpose,
                    security_level,
                    key_type,
                    platform_version,
                )?;
                Ok((key.into(), private_key))
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::random_key_with_known_attributes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Generates a random ECDSA master authentication public key along with its corresponding private key.
    ///
    /// This method constructs a random ECDSA (using the secp256k1 curve) master authentication public key
    /// and returns both the public key and its corresponding private key.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to the random number generator.
    ///
    /// # Returns
    ///
    /// * `(Self, Vec<u8>)`: A tuple where the first element is an instance of the `IdentityPublicKey` struct,
    ///                      and the second element is the corresponding private key.
    ///

    pub fn random_ecdsa_master_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => {
                let (key, private_key) =
                    IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
                        id,
                        rng,
                        platform_version,
                    )?;
                Ok((key.into(), private_key))
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Generates a random ECDSA master-level authentication public key along with its corresponding private key.
    ///
    /// This method constructs a random ECDSA (using the secp256k1 curve) high-level authentication public key
    /// and returns both the public key and its corresponding private key.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `seed`: A seed that will create a random number generator `StdRng`.
    ///
    /// # Returns
    ///
    /// * `(Self, Vec<u8>)`: A tuple where the first element is an instance of the `IdentityPublicKey` struct,
    ///                      and the second element is the corresponding private key.
    ///
    pub fn random_ecdsa_master_authentication_key(
        id: KeyID,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_ecdsa_master_authentication_key_with_rng(id, &mut rng, platform_version)
    }

    /// Generates a random ECDSA critical-level authentication public key along with its corresponding private key.
    ///
    /// This method constructs a random ECDSA (using the secp256k1 curve) high-level authentication public key
    /// and returns both the public key and its corresponding private key.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `seed`: A seed that will create a random number generator `StdRng`.
    ///
    /// # Returns
    ///
    /// * `(Self, Vec<u8>)`: A tuple where the first element is an instance of the `IdentityPublicKey` struct,
    ///                      and the second element is the corresponding private key.
    ///
    pub fn random_ecdsa_critical_level_authentication_key(
        id: KeyID,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_ecdsa_critical_level_authentication_key_with_rng(
            id,
            &mut rng,
            platform_version,
        )
    }

    /// Generates a random ECDSA high-level authentication public key along with its corresponding private key.
    ///
    /// This method constructs a random ECDSA (using the secp256k1 curve) high-level authentication public key
    /// and returns both the public key and its corresponding private key.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to the random number generator.
    ///
    /// # Returns
    ///
    /// * `(Self, Vec<u8>)`: A tuple where the first element is an instance of the `IdentityPublicKey` struct,
    ///                      and the second element is the corresponding private key.
    ///
    pub fn random_ecdsa_critical_level_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => {
                let (key, private_key) =
                    IdentityPublicKeyV0::random_ecdsa_critical_level_authentication_key_with_rng(
                        id,
                        rng,
                        platform_version,
                    )?;
                Ok((key.into(), private_key))
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Generates a random ECDSA high-level authentication public key along with its corresponding private key.
    ///
    /// This method constructs a random ECDSA (using the secp256k1 curve) high-level authentication public key
    /// and returns both the public key and its corresponding private key.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `seed`: A seed that will create a random number generator `StdRng`.
    ///
    /// # Returns
    ///
    /// * `(Self, Vec<u8>)`: A tuple where the first element is an instance of the `IdentityPublicKey` struct,
    ///                      and the second element is the corresponding private key.
    ///
    pub fn random_ecdsa_high_level_authentication_key(
        id: KeyID,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_ecdsa_high_level_authentication_key_with_rng(id, &mut rng, platform_version)
    }

    /// Generates a random ECDSA high-level authentication public key along with its corresponding private key.
    ///
    /// This method constructs a random ECDSA (using the secp256k1 curve) high-level authentication public key
    /// and returns both the public key and its corresponding private key.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `rng`: A mutable reference to the random number generator.
    ///
    /// # Returns
    ///
    /// * `(Self, Vec<u8>)`: A tuple where the first element is an instance of the `IdentityPublicKey` struct,
    ///                      and the second element is the corresponding private key.
    ///
    pub fn random_ecdsa_high_level_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => {
                let (key, private_key) =
                    IdentityPublicKeyV0::random_ecdsa_high_level_authentication_key_with_rng(
                        id,
                        rng,
                        platform_version,
                    )?;
                Ok((key.into(), private_key))
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::random_ecdsa_high_level_authentication_key_with_rng"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn random_authentication_keys_with_rng(
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Self>, ProtocolError> {
        let mut used_key_matrix = [false; 16].to_vec();
        (0..key_count)
            .map(|i| {
                Self::random_authentication_key_with_rng(
                    i,
                    rng,
                    Some((i, &mut used_key_matrix)),
                    platform_version,
                )
            })
            .collect()
    }

    pub fn random_authentication_keys_with_private_keys_with_rng(
        start_id: KeyID,
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Self, Vec<u8>)>, ProtocolError> {
        (start_id..(start_id + key_count))
            .map(|i| {
                Self::random_authentication_key_with_private_key_with_rng(
                    i,
                    rng,
                    None,
                    platform_version,
                )
            })
            .collect()
    }

    pub fn main_keys_with_random_authentication_keys_with_private_keys_with_rng(
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Self, Vec<u8>)>, ProtocolError> {
        if key_count < 2 {
            return Err(ProtocolError::PublicKeyGenerationError(
                "at least 2 keys must be created".to_string(),
            ));
        }
        //create a master and a high level key
        let mut main_keys = if key_count == 2 {
            vec![
                Self::random_ecdsa_master_authentication_key_with_rng(0, rng, platform_version)?,
                Self::random_ecdsa_high_level_authentication_key_with_rng(
                    1,
                    rng,
                    platform_version,
                )?,
            ]
        } else {
            vec![
                Self::random_ecdsa_master_authentication_key_with_rng(0, rng, platform_version)?,
                Self::random_ecdsa_critical_level_authentication_key_with_rng(
                    1,
                    rng,
                    platform_version,
                )?,
                Self::random_ecdsa_high_level_authentication_key_with_rng(
                    2,
                    rng,
                    platform_version,
                )?,
            ]
        };
        let mut used_key_matrix = [false; 16].to_vec();
        used_key_matrix[0] = true;
        used_key_matrix[1] = true;
        used_key_matrix[2] = true;
        used_key_matrix[4] = true; //also a master key
        used_key_matrix[8] = true; //also a master key
        used_key_matrix[12] = true; //also a master key
        main_keys.extend((3..key_count).map(|i| {
            Self::random_authentication_key_with_private_key_with_rng(
                i,
                rng,
                Some((i, &mut used_key_matrix)),
                platform_version,
            )
            .unwrap()
        }));
        Ok(main_keys)
    }

    pub fn random_unique_keys_with_rng(
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Self>, ProtocolError> {
        let mut keys = [false; 64].to_vec();
        (0..key_count)
            .map(|i| Self::random_key_with_rng(i, rng, Some((i, &mut keys)), platform_version))
            .collect()
    }

    pub fn random_keys_with_rng(
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<Self> {
        (0..key_count)
            .map(|i| Self::random_key_with_rng(i, rng, None, platform_version).unwrap())
            .collect()
    }
}
