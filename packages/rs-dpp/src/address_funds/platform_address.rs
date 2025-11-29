use crate::address_funds::AddressWitness;
use crate::prelude::AddressNonce;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use dashcore::address::Payload;
use dashcore::blockdata::script::ScriptBuf;
use dashcore::hashes::Hash;
use dashcore::{Address, Network, PubkeyHash, ScriptHash};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// The size of the address hash (20 bytes for both P2PKH and P2SH)
pub const ADDRESS_HASH_SIZE: usize = 20;

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub enum PlatformAddress {
    /// Pay to pubkey hash (type byte = 0)
    P2pkh([u8; 20]),
    /// Pay to script hash (type byte = 1)
    P2sh([u8; 20]),
}

impl TryFrom<Address> for PlatformAddress {
    type Error = ProtocolError;

    fn try_from(address: Address) -> Result<Self, Self::Error> {
        match address.payload() {
            Payload::PubkeyHash(hash) => Ok(PlatformAddress::P2pkh(*hash.as_ref())),
            Payload::ScriptHash(hash) => Ok(PlatformAddress::P2sh(*hash.as_ref())),
            _ => Err(ProtocolError::DecodingError(
                "unsupported address type for PlatformAddress: only P2PKH and P2SH are supported"
                    .to_string(),
            )),
        }
    }
}

impl Default for PlatformAddress {
    fn default() -> Self {
        PlatformAddress::P2pkh([0u8; 20])
    }
}

impl PlatformAddress {
    /// Type byte for P2PKH addresses
    pub const P2PKH_TYPE: u8 = 0;
    /// Type byte for P2SH addresses
    pub const P2SH_TYPE: u8 = 1;

    /// Converts the PlatformAddress to a dashcore Address with the specified network.
    pub fn to_address_with_network(&self, network: Network) -> Address {
        match self {
            PlatformAddress::P2pkh(hash) => Address::new(
                network,
                Payload::PubkeyHash(PubkeyHash::from_byte_array(*hash)),
            ),
            PlatformAddress::P2sh(hash) => Address::new(
                network,
                Payload::ScriptHash(ScriptHash::from_byte_array(*hash)),
            ),
        }
    }

    /// Converts the PlatformAddress to bytes.
    /// Format: [address_type (1 byte)] + [hash (20 bytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + ADDRESS_HASH_SIZE);
        match self {
            PlatformAddress::P2pkh(hash) => {
                bytes.push(Self::P2PKH_TYPE);
                bytes.extend_from_slice(hash);
            }
            PlatformAddress::P2sh(hash) => {
                bytes.push(Self::P2SH_TYPE);
                bytes.extend_from_slice(hash);
            }
        }
        bytes
    }

    /// Gets a base64 string of the PlatformAddress concatenated with the nonce.
    /// This creates a unique identifier for address-based state transition inputs.
    pub fn base64_string_with_nonce(&self, nonce: AddressNonce) -> String {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;

        let mut bytes = self.to_bytes();
        bytes.extend_from_slice(&nonce.to_be_bytes());

        STANDARD.encode(bytes)
    }

    /// Creates a PlatformAddress from bytes.
    /// Format: [address_type (1 byte)] + [hash (20 bytes)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < 1 + ADDRESS_HASH_SIZE {
            return Err(ProtocolError::DecodingError(format!(
                "cannot decode PlatformAddress: expected {} bytes, got {}",
                1 + ADDRESS_HASH_SIZE,
                bytes.len()
            )));
        }

        let address_type = bytes[0];
        let hash: [u8; 20] = bytes[1..21]
            .try_into()
            .map_err(|_| ProtocolError::DecodingError("invalid hash length".to_string()))?;

        match address_type {
            Self::P2PKH_TYPE => Ok(PlatformAddress::P2pkh(hash)),
            Self::P2SH_TYPE => Ok(PlatformAddress::P2sh(hash)),
            _ => Err(ProtocolError::DecodingError(format!(
                "invalid address type: {}",
                address_type
            ))),
        }
    }

    /// Returns the hash portion of the address (20 bytes)
    pub fn hash(&self) -> &[u8; 20] {
        match self {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(hash) => hash,
        }
    }

    /// Returns true if this is a P2PKH address
    pub fn is_p2pkh(&self) -> bool {
        matches!(self, PlatformAddress::P2pkh(_))
    }

    /// Returns true if this is a P2SH address
    pub fn is_p2sh(&self) -> bool {
        matches!(self, PlatformAddress::P2sh(_))
    }

    /// Verifies that the provided witness matches this address and that signatures are valid.
    ///
    /// For P2PKH addresses:
    /// - The witness must be `AddressWitness::P2pkh`
    /// - The public key must hash to this address
    /// - The signature must be valid for the signable bytes
    ///
    /// For P2SH addresses:
    /// - The witness must be `AddressWitness::P2sh`
    /// - The redeem script must hash to this address
    /// - For multisig scripts: M valid signatures must be provided for the signable bytes
    ///
    /// # Arguments
    /// * `witness` - The witness containing signature(s) and either a public key (P2PKH) or redeem script (P2SH)
    /// * `signable_bytes` - The data that was signed (will be double-SHA256 hashed internally)
    ///
    /// # Returns
    /// * `Ok(())` if the witness matches this address and all signatures are valid
    /// * `Err(ProtocolError)` if verification fails
    pub fn verify_bytes_against_witness(
        &self,
        witness: &AddressWitness,
        signable_bytes: &[u8],
    ) -> Result<(), ProtocolError> {
        match (self, witness) {
            (
                PlatformAddress::P2pkh(pubkey_hash),
                AddressWitness::P2pkh {
                    signature,
                    public_key,
                },
            ) => {
                // First verify the public key hashes to the address
                let computed_hash = public_key.pubkey_hash();
                if computed_hash.as_byte_array() != pubkey_hash {
                    return Err(ProtocolError::Generic(format!(
                        "Public key hash {} does not match address hash {}",
                        hex::encode(computed_hash.as_byte_array()),
                        hex::encode(pubkey_hash)
                    )));
                }

                // Verify the signature
                dashcore::signer::verify_data_signature(
                    signable_bytes,
                    signature.as_slice(),
                    &public_key.to_bytes(),
                )
                .map_err(|e| {
                    ProtocolError::Generic(format!("P2PKH signature verification failed: {}", e))
                })
            }
            (
                PlatformAddress::P2sh(script_hash),
                AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                },
            ) => {
                // First verify the redeem script hashes to the address
                let script = ScriptBuf::from_bytes(redeem_script.to_vec());
                let computed_hash = script.script_hash();
                if computed_hash.as_byte_array() != script_hash {
                    return Err(ProtocolError::Generic(format!(
                        "Script hash {} does not match address hash {}",
                        hex::encode(computed_hash.as_byte_array()),
                        hex::encode(script_hash)
                    )));
                }

                // Parse the redeem script to extract public keys and threshold
                // Expected format for multisig: OP_M <pubkey1> <pubkey2> ... <pubkeyN> OP_N OP_CHECKMULTISIG
                let (threshold, pubkeys) = Self::parse_multisig_script(&script)?;

                // Filter out empty signatures (OP_0 placeholders for CHECKMULTISIG bug)
                let valid_signatures: Vec<_> = signatures
                    .iter()
                    .filter(|sig| !sig.is_empty() && sig.as_slice() != [0x00])
                    .collect();

                if valid_signatures.len() < threshold {
                    return Err(ProtocolError::Generic(format!(
                        "Not enough signatures: got {}, need {}",
                        valid_signatures.len(),
                        threshold
                    )));
                }

                // Verify signatures against public keys
                // In standard multisig, signatures must match public keys in order
                let mut sig_idx = 0;
                let mut pubkey_idx = 0;
                let mut matched = 0;

                while sig_idx < valid_signatures.len() && pubkey_idx < pubkeys.len() {
                    let result = dashcore::signer::verify_data_signature(
                        signable_bytes,
                        valid_signatures[sig_idx].as_slice(),
                        &pubkeys[pubkey_idx],
                    );

                    if result.is_ok() {
                        matched += 1;
                        sig_idx += 1;
                    }
                    pubkey_idx += 1;
                }

                if matched >= threshold {
                    Ok(())
                } else {
                    Err(ProtocolError::Generic(format!(
                        "Not enough valid signatures: verified {}, need {}",
                        matched, threshold
                    )))
                }
            }
            (PlatformAddress::P2pkh(_), AddressWitness::P2sh { .. }) => {
                Err(ProtocolError::Generic(
                    "P2PKH address requires P2pkh witness, got P2sh".to_string(),
                ))
            }
            (PlatformAddress::P2sh(_), AddressWitness::P2pkh { .. }) => Err(
                ProtocolError::Generic("P2SH address requires P2sh witness, got P2pkh".to_string()),
            ),
        }
    }

    /// Parses a multisig redeem script and extracts the threshold (M) and public keys.
    ///
    /// Expected format: OP_M <pubkey1> <pubkey2> ... <pubkeyN> OP_N OP_CHECKMULTISIG
    ///
    /// # Supported Scripts
    ///
    /// Currently only standard bare multisig scripts are supported. Other P2SH script types
    /// (timelocks, hash puzzles, custom scripts) are not supported and will return an error.
    ///
    /// Full script execution would require either:
    /// - Using the `bitcoinconsensus` library with a synthetic spending transaction
    /// - Implementing a complete script interpreter
    ///
    /// For Platform's authorization use cases, multisig is the primary expected P2SH pattern.
    fn parse_multisig_script(script: &ScriptBuf) -> Result<(usize, Vec<Vec<u8>>), ProtocolError> {
        use dashcore::blockdata::opcodes::all::*;

        let mut instructions = script.instructions();
        let mut pubkeys = Vec::new();

        // First instruction should be OP_M (threshold)
        let threshold = match instructions.next() {
            Some(Ok(dashcore::blockdata::script::Instruction::Op(op))) => {
                let byte = op.to_u8();
                if byte >= OP_PUSHNUM_1.to_u8() && byte <= OP_PUSHNUM_16.to_u8() {
                    (byte - OP_PUSHNUM_1.to_u8() + 1) as usize
                } else {
                    return Err(ProtocolError::Generic(format!(
                        "Unsupported P2SH script type: only standard multisig (OP_M ... OP_N OP_CHECKMULTISIG) is supported. \
                         First opcode was 0x{:02x}, expected OP_1 through OP_16",
                        byte
                    )));
                }
            }
            Some(Ok(dashcore::blockdata::script::Instruction::PushBytes(_))) => {
                return Err(ProtocolError::Generic(
                    "Unsupported P2SH script type: only standard multisig is supported. \
                     Script starts with a data push instead of OP_M threshold."
                        .to_string(),
                ))
            }
            Some(Err(e)) => {
                return Err(ProtocolError::Generic(format!(
                    "Error parsing P2SH script: {:?}",
                    e
                )))
            }
            None => {
                return Err(ProtocolError::Generic(
                    "Empty P2SH redeem script".to_string(),
                ))
            }
        };

        // Read public keys until we hit OP_N
        loop {
            match instructions.next() {
                Some(Ok(dashcore::blockdata::script::Instruction::PushBytes(bytes))) => {
                    // Validate it looks like a public key (33 or 65 bytes)
                    let len = bytes.len();
                    if len != 33 && len != 65 {
                        return Err(ProtocolError::Generic(format!(
                            "Invalid public key in multisig script: expected 33 or 65 bytes, got {}",
                            len
                        )));
                    }
                    pubkeys.push(bytes.as_bytes().to_vec());
                }
                Some(Ok(dashcore::blockdata::script::Instruction::Op(op))) => {
                    let byte = op.to_u8();
                    if byte >= OP_PUSHNUM_1.to_u8() && byte <= OP_PUSHNUM_16.to_u8() {
                        // This is OP_N, the total number of keys
                        let n = (byte - OP_PUSHNUM_1.to_u8() + 1) as usize;
                        if pubkeys.len() != n {
                            return Err(ProtocolError::Generic(format!(
                                "Multisig script declares {} keys but contains {}",
                                n,
                                pubkeys.len()
                            )));
                        }
                        break;
                    } else if op == OP_CHECKMULTISIG || op == OP_CHECKMULTISIGVERIFY {
                        // Hit CHECKMULTISIG without seeing OP_N - malformed
                        return Err(ProtocolError::Generic(
                            "Malformed multisig script: OP_CHECKMULTISIG before OP_N".to_string(),
                        ));
                    } else {
                        return Err(ProtocolError::Generic(format!(
                            "Unsupported opcode 0x{:02x} in P2SH script. Only standard multisig is supported.",
                            byte
                        )));
                    }
                }
                Some(Err(e)) => {
                    return Err(ProtocolError::Generic(format!(
                        "Error parsing multisig script: {:?}",
                        e
                    )))
                }
                None => {
                    return Err(ProtocolError::Generic(
                        "Incomplete multisig script: unexpected end before OP_N".to_string(),
                    ))
                }
            }
        }

        // Validate threshold
        if threshold > pubkeys.len() {
            return Err(ProtocolError::Generic(format!(
                "Invalid multisig: threshold {} exceeds number of keys {}",
                threshold,
                pubkeys.len()
            )));
        }

        // Next should be OP_CHECKMULTISIG
        match instructions.next() {
            Some(Ok(dashcore::blockdata::script::Instruction::Op(op))) => {
                if op == OP_CHECKMULTISIG {
                    // Standard multisig - verify script is complete
                    if instructions.next().is_some() {
                        return Err(ProtocolError::Generic(
                            "Multisig script has extra data after OP_CHECKMULTISIG".to_string(),
                        ));
                    }
                    Ok((threshold, pubkeys))
                } else if op == OP_CHECKMULTISIGVERIFY {
                    Err(ProtocolError::Generic(
                        "OP_CHECKMULTISIGVERIFY is not supported, only OP_CHECKMULTISIG"
                            .to_string(),
                    ))
                } else {
                    Err(ProtocolError::Generic(format!(
                        "Expected OP_CHECKMULTISIG, got opcode 0x{:02x}",
                        op.to_u8()
                    )))
                }
            }
            _ => Err(ProtocolError::Generic(
                "Invalid multisig script: expected OP_CHECKMULTISIG after OP_N".to_string(),
            )),
        }
    }
}

impl std::fmt::Display for PlatformAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformAddress::P2pkh(hash) => write!(f, "P2PKH({})", hex::encode(hash)),
            PlatformAddress::P2sh(hash) => write!(f, "P2SH({})", hex::encode(hash)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::blockdata::opcodes::all::*;
    use dashcore::hashes::Hash;
    use dashcore::secp256k1::{PublicKey as RawPublicKey, Secp256k1, SecretKey as RawSecretKey};
    use dashcore::PublicKey;
    use platform_value::BinaryData;

    /// Helper to create a keypair from a 32-byte seed
    fn create_keypair(seed: [u8; 32]) -> (RawSecretKey, PublicKey) {
        let secp = Secp256k1::new();
        let secret_key = RawSecretKey::from_byte_array(&seed).expect("valid secret key");
        let raw_public_key = RawPublicKey::from_secret_key(&secp, &secret_key);
        let public_key = PublicKey::new(raw_public_key);
        (secret_key, public_key)
    }

    /// Helper to sign data with a secret key
    fn sign_data(data: &[u8], secret_key: &RawSecretKey) -> Vec<u8> {
        dashcore::signer::sign(data, secret_key.as_ref())
            .expect("signing should succeed")
            .to_vec()
    }

    /// Creates a standard multisig redeem script: OP_M <pubkey1> ... <pubkeyN> OP_N OP_CHECKMULTISIG
    fn create_multisig_script(threshold: u8, pubkeys: &[PublicKey]) -> Vec<u8> {
        let mut script = Vec::new();

        // OP_M (threshold)
        script.push(OP_PUSHNUM_1.to_u8() + threshold - 1);

        // Push each public key (33 bytes each for compressed)
        for pubkey in pubkeys {
            let bytes = pubkey.to_bytes();
            script.push(bytes.len() as u8); // push length
            script.extend_from_slice(&bytes);
        }

        // OP_N (total keys)
        script.push(OP_PUSHNUM_1.to_u8() + pubkeys.len() as u8 - 1);

        // OP_CHECKMULTISIG
        script.push(OP_CHECKMULTISIG.to_u8());

        script
    }

    #[test]
    fn test_p2pkh_verify_signature_success() {
        // Create a keypair
        let seed = [1u8; 32];
        let (secret_key, public_key) = create_keypair(seed);

        // Create P2PKH address from public key hash
        let pubkey_hash = public_key.pubkey_hash();
        let address = PlatformAddress::P2pkh(*pubkey_hash.as_byte_array());

        // Data to sign
        let signable_bytes = b"test message for P2PKH verification";

        // Sign the data
        let signature = sign_data(signable_bytes, &secret_key);

        // Create witness
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
            public_key,
        };

        // Verify should succeed
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_ok(),
            "P2PKH verification should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_p2pkh_verify_wrong_signature_fails() {
        // Create a keypair
        let seed = [1u8; 32];
        let (secret_key, public_key) = create_keypair(seed);

        // Create P2PKH address from public key hash
        let pubkey_hash = public_key.pubkey_hash();
        let address = PlatformAddress::P2pkh(*pubkey_hash.as_byte_array());

        // Sign different data than what we verify
        let sign_bytes = b"original message";
        let verify_bytes = b"different message";
        let signature = sign_data(sign_bytes, &secret_key);

        // Create witness with signature for different data
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
            public_key,
        };

        // Verify should fail
        let result = address.verify_bytes_against_witness(&witness, verify_bytes);
        assert!(
            result.is_err(),
            "P2PKH verification should fail with wrong data"
        );
    }

    #[test]
    fn test_p2pkh_verify_wrong_pubkey_fails() {
        // Create two keypairs
        let seed1 = [1u8; 32];
        let seed2 = [2u8; 32];
        let (secret_key1, public_key1) = create_keypair(seed1);
        let (_secret_key2, public_key2) = create_keypair(seed2);

        // Create P2PKH address from public key 1's hash
        let pubkey_hash = public_key1.pubkey_hash();
        let address = PlatformAddress::P2pkh(*pubkey_hash.as_byte_array());

        // Sign with key 1
        let signable_bytes = b"test message";
        let signature = sign_data(signable_bytes, &secret_key1);

        // Create witness with public key 2 (wrong key)
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
            public_key: public_key2,
        };

        // Verify should fail (public key doesn't match address)
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_err(),
            "P2PKH verification should fail with wrong public key"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not match address hash"),
            "Error should mention hash mismatch"
        );
    }

    #[test]
    fn test_p2sh_2_of_3_multisig_verify_success() {
        // Create 3 keypairs for 2-of-3 multisig
        let seeds: [[u8; 32]; 3] = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let keypairs: Vec<_> = seeds.iter().map(|s| create_keypair(*s)).collect();
        let pubkeys: Vec<_> = keypairs.iter().map(|(_, pk)| *pk).collect();

        // Create 2-of-3 multisig redeem script
        let redeem_script = create_multisig_script(2, &pubkeys);

        // Create P2SH address from script hash
        let script_buf = ScriptBuf::from_bytes(redeem_script.clone());
        let script_hash = script_buf.script_hash();
        let address = PlatformAddress::P2sh(*script_hash.as_byte_array());

        // Data to sign
        let signable_bytes = b"test message for P2SH 2-of-3 multisig";

        // Sign with first two keys (keys 0 and 1)
        let sig0 = sign_data(signable_bytes, &keypairs[0].0);
        let sig1 = sign_data(signable_bytes, &keypairs[1].0);

        // Create witness with signatures in order
        // Note: CHECKMULTISIG requires signatures in the same order as pubkeys
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(sig0), BinaryData::new(sig1)],
            redeem_script: BinaryData::new(redeem_script),
        };

        // Verify should succeed
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_ok(),
            "P2SH 2-of-3 multisig verification should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_p2sh_2_of_3_multisig_with_keys_1_and_2_success() {
        // Create 3 keypairs for 2-of-3 multisig
        let seeds: [[u8; 32]; 3] = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let keypairs: Vec<_> = seeds.iter().map(|s| create_keypair(*s)).collect();
        let pubkeys: Vec<_> = keypairs.iter().map(|(_, pk)| *pk).collect();

        // Create 2-of-3 multisig redeem script
        let redeem_script = create_multisig_script(2, &pubkeys);

        // Create P2SH address from script hash
        let script_buf = ScriptBuf::from_bytes(redeem_script.clone());
        let script_hash = script_buf.script_hash();
        let address = PlatformAddress::P2sh(*script_hash.as_byte_array());

        // Data to sign
        let signable_bytes = b"test message for P2SH 2-of-3 multisig";

        // Sign with keys 1 and 2 (different combination)
        let sig1 = sign_data(signable_bytes, &keypairs[1].0);
        let sig2 = sign_data(signable_bytes, &keypairs[2].0);

        // Create witness with signatures in order
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(sig1), BinaryData::new(sig2)],
            redeem_script: BinaryData::new(redeem_script),
        };

        // Verify should succeed
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_ok(),
            "P2SH 2-of-3 multisig with keys 1 and 2 should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_p2sh_not_enough_signatures_fails() {
        // Create 3 keypairs for 2-of-3 multisig
        let seeds: [[u8; 32]; 3] = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let keypairs: Vec<_> = seeds.iter().map(|s| create_keypair(*s)).collect();
        let pubkeys: Vec<_> = keypairs.iter().map(|(_, pk)| *pk).collect();

        // Create 2-of-3 multisig redeem script
        let redeem_script = create_multisig_script(2, &pubkeys);

        // Create P2SH address from script hash
        let script_buf = ScriptBuf::from_bytes(redeem_script.clone());
        let script_hash = script_buf.script_hash();
        let address = PlatformAddress::P2sh(*script_hash.as_byte_array());

        // Data to sign
        let signable_bytes = b"test message";

        // Only sign with one key (need 2)
        let sig0 = sign_data(signable_bytes, &keypairs[0].0);

        // Create witness with only one signature
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(sig0)],
            redeem_script: BinaryData::new(redeem_script),
        };

        // Verify should fail
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_err(),
            "P2SH should fail with only 1 signature when 2 required"
        );
        assert!(
            result.unwrap_err().to_string().contains("Not enough"),
            "Error should mention not enough signatures"
        );
    }

    #[test]
    fn test_p2sh_wrong_script_hash_fails() {
        // Create 3 keypairs
        let seeds: [[u8; 32]; 3] = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let keypairs: Vec<_> = seeds.iter().map(|s| create_keypair(*s)).collect();
        let pubkeys: Vec<_> = keypairs.iter().map(|(_, pk)| *pk).collect();

        // Create a redeem script
        let redeem_script = create_multisig_script(2, &pubkeys);

        // Create P2SH address with DIFFERENT hash (wrong address)
        let wrong_hash = [0xABu8; 20];
        let address = PlatformAddress::P2sh(wrong_hash);

        // Data to sign
        let signable_bytes = b"test message";

        // Sign correctly
        let sig0 = sign_data(signable_bytes, &keypairs[0].0);
        let sig1 = sign_data(signable_bytes, &keypairs[1].0);

        // Create witness
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(sig0), BinaryData::new(sig1)],
            redeem_script: BinaryData::new(redeem_script),
        };

        // Verify should fail (script doesn't hash to address)
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_err(),
            "P2SH should fail when script hash doesn't match address"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not match address hash"),
            "Error should mention hash mismatch"
        );
    }

    #[test]
    fn test_p2pkh_and_p2sh_together() {
        // This test simulates having both a P2PKH and P2SH output and redeeming both

        // === P2PKH Output ===
        let p2pkh_seed = [10u8; 32];
        let (p2pkh_secret, p2pkh_pubkey) = create_keypair(p2pkh_seed);
        let p2pkh_hash = p2pkh_pubkey.pubkey_hash();
        let p2pkh_address = PlatformAddress::P2pkh(*p2pkh_hash.as_byte_array());

        // === P2SH Output (2-of-3 multisig) ===
        let p2sh_seeds: [[u8; 32]; 3] = [[20u8; 32], [21u8; 32], [22u8; 32]];
        let p2sh_keypairs: Vec<_> = p2sh_seeds.iter().map(|s| create_keypair(*s)).collect();
        let p2sh_pubkeys: Vec<_> = p2sh_keypairs.iter().map(|(_, pk)| *pk).collect();
        let redeem_script = create_multisig_script(2, &p2sh_pubkeys);
        let script_buf = ScriptBuf::from_bytes(redeem_script.clone());
        let script_hash = script_buf.script_hash();
        let p2sh_address = PlatformAddress::P2sh(*script_hash.as_byte_array());

        // === Signable bytes (same for both in this test) ===
        let signable_bytes = b"combined transaction data to redeem both outputs";

        // === Redeem P2PKH ===
        let p2pkh_sig = sign_data(signable_bytes, &p2pkh_secret);
        let p2pkh_witness = AddressWitness::P2pkh {
            signature: BinaryData::new(p2pkh_sig),
            public_key: p2pkh_pubkey,
        };
        let p2pkh_result =
            p2pkh_address.verify_bytes_against_witness(&p2pkh_witness, signable_bytes);
        assert!(
            p2pkh_result.is_ok(),
            "P2PKH redemption should succeed: {:?}",
            p2pkh_result
        );

        // === Redeem P2SH (using keys 0 and 2) ===
        let p2sh_sig0 = sign_data(signable_bytes, &p2sh_keypairs[0].0);
        let p2sh_sig2 = sign_data(signable_bytes, &p2sh_keypairs[2].0);
        let p2sh_witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(p2sh_sig0), BinaryData::new(p2sh_sig2)],
            redeem_script: BinaryData::new(redeem_script),
        };
        let p2sh_result = p2sh_address.verify_bytes_against_witness(&p2sh_witness, signable_bytes);
        assert!(
            p2sh_result.is_ok(),
            "P2SH redemption should succeed: {:?}",
            p2sh_result
        );

        // Both outputs successfully redeemed!
    }

    #[test]
    fn test_witness_type_mismatch() {
        // Create P2PKH address
        let seed = [1u8; 32];
        let (_, public_key) = create_keypair(seed);
        let pubkey_hash = public_key.pubkey_hash();
        let p2pkh_address = PlatformAddress::P2pkh(*pubkey_hash.as_byte_array());

        // Create P2SH address
        let p2sh_hash = [0xABu8; 20];
        let p2sh_address = PlatformAddress::P2sh(p2sh_hash);

        let signable_bytes = b"test data";

        // Try P2SH witness on P2PKH address
        let p2sh_witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(vec![0x30, 0x44])],
            redeem_script: BinaryData::new(vec![0x52]),
        };
        let result = p2pkh_address.verify_bytes_against_witness(&p2sh_witness, signable_bytes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("P2PKH address requires P2pkh witness"));

        // Try P2PKH witness on P2SH address
        let p2pkh_witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44]),
            public_key,
        };
        let result = p2sh_address.verify_bytes_against_witness(&p2pkh_witness, signable_bytes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("P2SH address requires P2sh witness"));
    }
}
