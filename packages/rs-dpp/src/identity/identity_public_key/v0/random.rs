use crate::identity::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::random::MAX_RANDOM_KEYS;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::KeyType::{ECDSA_HASH160, ECDSA_SECP256K1};
use crate::identity::Purpose::{AUTHENTICATION, VOTING};
use crate::identity::SecurityLevel::{CRITICAL, HIGH, MASTER, MEDIUM};
use crate::identity::{KeyCount, KeyID, KeyType, Purpose, SecurityLevel};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::BinaryData;
use rand::rngs::StdRng;
use rand::Rng;
use std::convert::TryFrom;
use std::ops::{Div, Rem};

pub type UsedKeyMatrix = Vec<bool>;

impl IdentityPublicKeyV0 {
    pub fn random_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
        platform_version: &PlatformVersion,
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
        key_matrix.iter_mut().enumerate().for_each(|(pos, is_set)| {
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
        let data = BinaryData::new(key_type.random_public_key_data(rng, platform_version)?);
        Ok(IdentityPublicKeyV0 {
            id,
            key_type,
            purpose: AUTHENTICATION,
            security_level,
            read_only,
            disabled_at: None,
            data,
            contract_bounds: None,
        })
    }

    pub fn random_authentication_key_with_private_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        // we have 16 different permutations possible

        let mut binding = [false; MAX_RANDOM_KEYS].to_vec();
        let (key_count, key_matrix) = used_key_matrix.unwrap_or((0, &mut binding));

        // input validation
        if key_count != key_matrix.iter().filter(|x| **x).count() as u32 {
            return Err(ProtocolError::PublicKeyGenerationError(
                "invalid argument: key count in used_key_matrix.0 doesn't match actual number of used keys".to_string(),
            ));
        }

        // we need space for at least one additional key
        if key_count > MAX_RANDOM_KEYS as u32 - 1 {
            return Err(ProtocolError::PublicKeyGenerationError(
                "too many keys already created".to_string(),
            ));
        }

        // max_key_number is the number of keys that can be created
        let max_key_number = MAX_RANDOM_KEYS as u32 - key_count;
        let key_number = rng.gen_range(0..max_key_number);
        // now we need to find n'th not used key of this key_matrix (where `n = key_number`),
        // that is: the first bool that isn't set to true
        let mut needed_pos = None;
        let mut counter = 0;
        let mut used = 0;
        key_matrix.iter_mut().enumerate().for_each(|(pos, is_set)| {
            if !*is_set {
                if counter == key_number {
                    needed_pos = Some(pos as u8);
                    *is_set = true;
                }
                counter += 1;
            } else {
                used += 1; // for debugging purposes
            }
        });
        // should never happen, as we have input validation above
        assert_eq!(
            used, key_count,
            "incorrect number of used keys in key_matrix {:?}",
            key_matrix,
        );
        let needed_pos = needed_pos.ok_or(ProtocolError::PublicKeyGenerationError(format!(
            "too many keys already created: , key_count: {}, random key number: {}, unused key counter: {}, used key counter: {}",
            key_count, key_number, counter, used,
        )))?;
        let key_type = needed_pos.div(&4);
        let security_level = needed_pos.rem(&4);
        let security_level = SecurityLevel::try_from(security_level).unwrap();
        let key_type = KeyType::try_from(key_type).unwrap();
        let read_only = false;
        let (public_data, private_data) =
            key_type.random_public_and_private_key_data(rng, platform_version)?;
        let data = BinaryData::new(public_data);
        Ok((
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose: AUTHENTICATION,
                security_level,
                read_only,
                disabled_at: None,
                data,
                contract_bounds: None,
            },
            private_data,
        ))
    }

    pub fn random_key_with_known_attributes(
        id: KeyID,
        rng: &mut StdRng,
        purpose: Purpose,
        security_level: SecurityLevel,
        key_type: KeyType,
        contract_bounds: Option<ContractBounds>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let read_only = false;
        let (public_data, private_data) =
            key_type.random_public_and_private_key_data(rng, platform_version)?;
        let data = BinaryData::new(public_data);
        let identity_public_key = IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
            contract_bounds,
        };
        Ok((identity_public_key, private_data))
    }

    pub fn random_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        used_key_matrix: Option<(KeyCount, &mut UsedKeyMatrix)>,
        platform_version: &PlatformVersion,
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
        key_matrix.iter_mut().enumerate().for_each(|(pos, is_set)| {
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
        let data = BinaryData::new(key_type.random_public_key_data(rng, platform_version)?);
        Ok(IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
            contract_bounds: None,
        })
    }

    pub fn random_ecdsa_master_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let key_type = ECDSA_SECP256K1;
        let purpose = AUTHENTICATION;
        let security_level = MASTER;
        let read_only = false;
        let (data, private_data) =
            key_type.random_public_and_private_key_data(rng, platform_version)?;
        Ok((
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level,
                read_only,
                disabled_at: None,
                data: data.into(),
                contract_bounds: None,
            },
            private_data,
        ))
    }

    pub fn random_voting_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let key_type = ECDSA_HASH160;
        let purpose = VOTING;
        let security_level = MEDIUM;
        let read_only = false;
        let (data, private_data) =
            key_type.random_public_and_private_key_data(rng, platform_version)?;
        Ok((
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level,
                read_only,
                disabled_at: None,
                data: data.into(),
                contract_bounds: None,
            },
            private_data,
        ))
    }

    pub fn random_ecdsa_critical_level_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let key_type = ECDSA_SECP256K1;
        let purpose = AUTHENTICATION;
        let security_level = CRITICAL;
        let read_only = false;
        let (data, private_data) =
            key_type.random_public_and_private_key_data(rng, platform_version)?;
        Ok((
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level,
                read_only,
                disabled_at: None,
                data: data.into(),
                contract_bounds: None,
            },
            private_data,
        ))
    }

    pub fn random_ecdsa_high_level_authentication_key_with_rng(
        id: KeyID,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        let key_type = ECDSA_SECP256K1;
        let purpose = AUTHENTICATION;
        let security_level = HIGH;
        let read_only = false;
        let (data, private_data) =
            key_type.random_public_and_private_key_data(rng, platform_version)?;
        Ok((
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level,
                read_only,
                disabled_at: None,
                data: data.into(),
                contract_bounds: None,
            },
            private_data,
        ))
    }
}
