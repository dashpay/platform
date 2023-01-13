use crate::identity::key_type::KEY_TYPE_MAX_SIZE_TYPE;
use crate::identity::KeyType::ECDSA_SECP256K1;
use crate::identity::Purpose::AUTHENTICATION;
use crate::identity::SecurityLevel::MASTER;
use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use crate::ProtocolError;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::convert::TryFrom;
use std::ops::{Div, Rem};

pub type KeyCount = KeyID;

pub type UsedKeyMatrix = Vec<bool>;

impl IdentityPublicKey {
    pub fn random_key(id: KeyID, seed: Option<u64>) -> Self {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_key_with_rng(id, &mut rng, None).unwrap()
    }

    pub fn random_keys(first_id: KeyID, count: KeyCount, seed: Option<u64>) -> Vec<Self> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let end_id = first_id + count;
        (first_id..end_id)
            .into_iter()
            .map(|key_id| Self::random_key_with_rng(key_id, &mut rng, None).unwrap())
            .collect()
    }

    pub fn random_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
    ) -> Result<Self, ProtocolError> {
        // we have 16 different permutations possible
        let mut binding = [false; 16].to_vec();
        let (key_count, key_matrix) = used_key_matrix.unwrap_or((0, &mut binding));
        if key_count > 16 {
            return Err(ProtocolError::PublicKeyGenerationError(
                "too many keys already created".to_string(),
            ));
        }
        let key_number = rng.gen_range(0..(16 - key_count as u8));
        // now we need to find the first bool that isn't set to true
        let mut needed_pos = None;
        let mut counter = 0;
        key_matrix
            .into_iter()
            .enumerate()
            .for_each(|(pos, is_set)| {
                if !*is_set {
                    if counter == key_number {
                        needed_pos = Some(pos as u8);
                        *is_set = true;
                    }
                    counter += 1;
                }
            });
        let needed_pos = needed_pos.ok_or(ProtocolError::PublicKeyGenerationError(
            "too many keys already created".to_string(),
        ))?;
        let key_type = needed_pos.div(&4);
        let security_level = needed_pos.rem(&4);
        let security_level = SecurityLevel::try_from(security_level).unwrap();
        let key_type = KeyType::try_from(key_type).unwrap();
        let read_only = false;
        let data = key_type.random_public_key_data(rng);
        Ok(IdentityPublicKey {
            id,
            key_type,
            purpose: AUTHENTICATION,
            security_level,
            read_only,
            disabled_at: None,
            data,
        })
    }

    pub fn random_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
    ) -> Result<Self, ProtocolError> {
        // we have 64 different permutations possible
        let mut binding = [false; 64].to_vec();
        let (key_count, key_matrix) = used_key_matrix.unwrap_or((0, &mut binding));
        if key_count > 64 {
            return Err(ProtocolError::PublicKeyGenerationError(
                "too many keys already created".to_string(),
            ));
        }
        let key_number = rng.gen_range(0..(64 - key_count as u8));
        // now we need to find the first bool that isn't set to true
        let mut needed_pos = None;
        let mut counter = 0;
        key_matrix
            .into_iter()
            .enumerate()
            .for_each(|(pos, is_set)| {
                if !*is_set {
                    if counter == key_number {
                        needed_pos = Some(pos as u8);
                        *is_set = true;
                    }
                    counter += 1;
                }
            });
        let needed_pos = needed_pos.ok_or(ProtocolError::PublicKeyGenerationError(
            "too many keys already created".to_string(),
        ))?;
        let security_level = needed_pos.div(&16);
        let left = needed_pos.rem(&16);
        let security_level = SecurityLevel::try_from(security_level).unwrap();
        let key_type = left.div(&4);
        let purpose = left.rem(&4);
        let purpose = Purpose::try_from(purpose).unwrap();
        let key_type = KeyType::try_from(key_type).unwrap();
        let read_only = false;
        let data = key_type.random_public_key_data(rng);
        Ok(IdentityPublicKey {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
        })
    }

    pub fn max_possible_size_key(id: KeyID) -> Self {
        let key_type = *KEY_TYPE_MAX_SIZE_TYPE;
        let purpose = AUTHENTICATION;
        let security_level = MASTER;
        let read_only = false;
        let data = vec![255; key_type.default_size()];

        IdentityPublicKey {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
        }
    }

    pub fn random_ecdsa_master_authentication_key_with_rng(id: KeyID, rng: &mut StdRng) -> Self {
        let key_type = ECDSA_SECP256K1;
        let purpose = AUTHENTICATION;
        let security_level = MASTER;
        let read_only = false;
        let data = key_type.random_public_key_data(rng);
        IdentityPublicKey {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
        }
    }

    pub fn random_authentication_keys_with_rng(key_count: KeyCount, rng: &mut StdRng) -> Vec<Self> {
        let mut used_key_matrix = [false; 16].to_vec();
        (0..key_count)
            .map(|i| Self::random_authentication_key_with_rng(i, rng, Some((i, &mut used_key_matrix))).unwrap())
            .collect()
    }

    pub fn random_keys_with_rng(key_count: KeyCount, rng: &mut StdRng) -> Vec<Self> {
        (0..key_count)
            .map(|i| Self::random_key_with_rng(i, rng, None).unwrap())
            .collect()
    }

    pub fn random_unique_keys_with_rng(
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Result<Vec<Self>, ProtocolError> {
        let mut keys = [false; 64].to_vec();
        (0..key_count)
            .map(|i| Self::random_key_with_rng(i, rng, Some((i, &mut keys))))
            .collect()
    }
}
