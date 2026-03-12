use crate::address_funds::AddressWitness;
use crate::address_funds::AddressWitnessVerificationOperations;
use crate::prelude::AddressNonce;
use crate::ProtocolError;
use bech32::{Bech32m, Hrp};
use bincode::{Decode, Encode};
use dashcore::address::Payload;
use dashcore::blockdata::script::ScriptBuf;
use dashcore::hashes::{sha256d, Hash};
use dashcore::key::Secp256k1;
use dashcore::secp256k1::ecdsa::RecoverableSignature;
use dashcore::secp256k1::Message;
use dashcore::signer::CompactSignature;
use dashcore::{Address, Network, PrivateKey, PubkeyHash, PublicKey, ScriptHash};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::str::FromStr;

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
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub enum PlatformAddress {
    /// Pay to pubkey hash
    /// - bech32m encoding type byte: 0xb0
    /// - storage key type byte: 0x00
    P2pkh([u8; 20]),
    /// Pay to script hash
    /// - bech32m encoding type byte: 0x80
    /// - storage key type byte: 0x01
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

impl From<&PrivateKey> for PlatformAddress {
    /// Derives a P2PKH Platform address from a private key.
    ///
    /// The address is derived as: P2PKH(Hash160(compressed_public_key))
    /// where Hash160 = RIPEMD160(SHA256(x)), which is the standard Bitcoin P2PKH derivation.
    fn from(private_key: &PrivateKey) -> Self {
        let secp = Secp256k1::new();
        let pubkey_hash = private_key.public_key(&secp).pubkey_hash();
        PlatformAddress::P2pkh(*pubkey_hash.as_byte_array())
    }
}

impl Default for PlatformAddress {
    fn default() -> Self {
        PlatformAddress::P2pkh([0u8; 20])
    }
}

/// Human-readable part for Platform addresses on mainnet (DIP-0018)
pub const PLATFORM_HRP_MAINNET: &str = "dash";
/// Human-readable part for Platform addresses on testnet/devnet/regtest (DIP-0018)
pub const PLATFORM_HRP_TESTNET: &str = "tdash";

impl PlatformAddress {
    /// Type byte for P2PKH addresses in bech32m encoding (user-facing)
    pub const P2PKH_TYPE: u8 = 0xb0;
    /// Type byte for P2SH addresses in bech32m encoding (user-facing)
    pub const P2SH_TYPE: u8 = 0x80;

    /// Returns the appropriate HRP (Human-Readable Part) for the given network.
    ///
    /// Per DIP-0018:
    /// - Mainnet: "dash"
    /// - Testnet/Devnet/Regtest: "tdash"
    pub fn hrp_for_network(network: Network) -> &'static str {
        match network {
            Network::Dash => PLATFORM_HRP_MAINNET,
            Network::Testnet | Network::Devnet | Network::Regtest => PLATFORM_HRP_TESTNET,
            // For any other networks, default to testnet HRP
            _ => PLATFORM_HRP_TESTNET,
        }
    }

    /// Encodes the PlatformAddress as a bech32m string for the specified network.
    ///
    /// The encoding follows DIP-0018:
    /// - Format: `<HRP>1<data-part>`
    /// - Data: type_byte (0xb0 for P2PKH, 0x80 for P2SH) || 20-byte hash
    /// - Checksum: bech32m (BIP-350)
    ///
    /// NOTE: This uses bech32m type bytes (0xb0/0x80) for user-facing addresses,
    /// NOT the storage type bytes (0x00/0x01) used in GroveDB keys.
    ///
    /// # Example
    /// ```ignore
    /// let address = PlatformAddress::P2pkh([0xf7, 0xda, ...]);
    /// let encoded = address.to_bech32m_string(Network::Dash);
    /// // Returns something like "dash1k..."
    /// ```
    pub fn to_bech32m_string(&self, network: Network) -> String {
        let hrp_str = Self::hrp_for_network(network);
        let hrp = Hrp::parse(hrp_str).expect("HRP is valid");

        // Build the 21-byte payload: type_byte || hash
        // Using bech32m type bytes (0xb0/0x80), NOT storage type bytes (0x00/0x01)
        let mut payload = Vec::with_capacity(1 + ADDRESS_HASH_SIZE);
        match self {
            PlatformAddress::P2pkh(hash) => {
                payload.push(Self::P2PKH_TYPE);
                payload.extend_from_slice(hash);
            }
            PlatformAddress::P2sh(hash) => {
                payload.push(Self::P2SH_TYPE);
                payload.extend_from_slice(hash);
            }
        }

        // Verified that this can not error
        bech32::encode::<Bech32m>(hrp, &payload).expect("encoding should succeed")
    }

    /// Decodes a bech32m-encoded Platform address string per DIP-0018.
    ///
    /// NOTE: This expects bech32m type bytes (0xb0/0x80) in the encoded string,
    /// NOT the storage type bytes (0x00/0x01) used in GroveDB keys.
    ///
    /// # Returns
    /// - `Ok((PlatformAddress, Network))` - The decoded address and its network
    /// - `Err(ProtocolError)` - If the address is invalid
    pub fn from_bech32m_string(s: &str) -> Result<(Self, Network), ProtocolError> {
        // Decode the bech32m string
        let (hrp, data) =
            bech32::decode(s).map_err(|e| ProtocolError::DecodingError(format!("{}", e)))?;

        // Determine network from HRP (case-insensitive per DIP-0018)
        let hrp_lower = hrp.as_str().to_ascii_lowercase();
        let network = match hrp_lower.as_str() {
            s if s == PLATFORM_HRP_MAINNET => Network::Dash,
            s if s == PLATFORM_HRP_TESTNET => Network::Testnet,
            _ => {
                return Err(ProtocolError::DecodingError(format!(
                    "invalid HRP '{}': expected '{}' or '{}'",
                    hrp, PLATFORM_HRP_MAINNET, PLATFORM_HRP_TESTNET
                )))
            }
        };

        // Validate payload length: 1 type byte + 20 hash bytes = 21 bytes
        if data.len() != 1 + ADDRESS_HASH_SIZE {
            return Err(ProtocolError::DecodingError(format!(
                "invalid Platform address length: expected {} bytes, got {}",
                1 + ADDRESS_HASH_SIZE,
                data.len()
            )));
        }

        // Parse using bech32m type bytes (0xb0/0x80), NOT storage type bytes
        let address_type = data[0];
        let hash: [u8; 20] = data[1..21]
            .try_into()
            .map_err(|_| ProtocolError::DecodingError("invalid hash length".to_string()))?;

        let address = match address_type {
            Self::P2PKH_TYPE => Ok(PlatformAddress::P2pkh(hash)),
            Self::P2SH_TYPE => Ok(PlatformAddress::P2sh(hash)),
            _ => Err(ProtocolError::DecodingError(format!(
                "invalid address type: 0x{:02x}",
                address_type
            ))),
        }?;

        Ok((address, network))
    }

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

    /// Converts the PlatformAddress to bytes for storage keys.
    /// Format: [variant_index (1 byte)] + [hash (20 bytes)]
    ///
    /// Uses bincode serialization which produces: 0x00 for P2pkh, 0x01 for P2sh.
    /// These bytes are used as keys in GroveDB.
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::encode_to_vec(self, bincode::config::standard())
            .expect("PlatformAddress serialization cannot fail")
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

    /// Creates a PlatformAddress from storage bytes.
    /// Format: [variant_index (1 byte)] + [hash (20 bytes)]
    ///
    /// Uses bincode deserialization which expects: 0x00 for P2pkh, 0x01 for P2sh.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let (address, _): (Self, usize) =
            bincode::decode_from_slice(bytes, bincode::config::standard()).map_err(|e| {
                ProtocolError::DecodingError(format!("cannot decode PlatformAddress: {}", e))
            })?;
        Ok(address)
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
    /// * `Ok(AddressWitnessVerificationOperations)` - Operations performed if verification succeeds
    /// * `Err(ProtocolError)` if verification fails
    pub fn verify_bytes_against_witness(
        &self,
        witness: &AddressWitness,
        signable_bytes: &[u8],
    ) -> Result<AddressWitnessVerificationOperations, ProtocolError> {
        match (self, witness) {
            (PlatformAddress::P2pkh(pubkey_hash), AddressWitness::P2pkh { signature }) => {
                // Use verify_hash_signature which:
                // 1. Computes double_sha256(signable_bytes)
                // 2. Recovers the public key from the signature
                // 3. Verifies Hash160(recovered_pubkey) matches pubkey_hash
                //
                // This saves 33 bytes per witness (no need to include pubkey)
                // at a ~4% CPU cost increase (recovery vs verify).
                let data_hash = dashcore::signer::double_sha(signable_bytes);
                dashcore::signer::verify_hash_signature(
                    &data_hash,
                    signature.as_slice(),
                    pubkey_hash,
                )
                .map_err(|e| {
                    ProtocolError::AddressWitnessError(format!(
                        "P2PKH signature verification failed: {}",
                        e
                    ))
                })?;

                Ok(AddressWitnessVerificationOperations::for_p2pkh(
                    signable_bytes.len(),
                ))
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
                    return Err(ProtocolError::AddressWitnessError(format!(
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
                    return Err(ProtocolError::AddressWitnessError(format!(
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
                let mut signature_verifications: u16 = 0;

                let signable_bytes_hash = sha256d::Hash::hash(signable_bytes).to_byte_array();
                let msg = Message::from_digest(signable_bytes_hash);
                let secp = Secp256k1::new();

                while sig_idx < valid_signatures.len() && pubkey_idx < pubkeys.len() {
                    signature_verifications += 1;

                    let sig = RecoverableSignature::from_compact_signature(
                        valid_signatures[sig_idx].as_slice(),
                    )
                    .map_err(|e| {
                        ProtocolError::AddressWitnessError(format!(
                            "Invalid signature format: {}",
                            e
                        ))
                    })?;

                    let pub_key = PublicKey::from_slice(&pubkeys[pubkey_idx]).map_err(|e| {
                        ProtocolError::AddressWitnessError(format!("Invalid public key: {}", e))
                    })?;

                    if secp
                        .verify_ecdsa(&msg, &sig.to_standard(), &pub_key.inner)
                        .is_ok()
                    {
                        matched += 1;
                        sig_idx += 1;
                    }
                    pubkey_idx += 1;
                }

                if matched >= threshold {
                    Ok(AddressWitnessVerificationOperations::for_p2sh_multisig(
                        signature_verifications,
                        signable_bytes.len(),
                    ))
                } else {
                    Err(ProtocolError::AddressWitnessError(format!(
                        "Not enough valid signatures: verified {}, need {}",
                        matched, threshold
                    )))
                }
            }
            (PlatformAddress::P2pkh(_), AddressWitness::P2sh { .. }) => {
                Err(ProtocolError::AddressWitnessError(
                    "P2PKH address requires P2pkh witness, got P2sh".to_string(),
                ))
            }
            (PlatformAddress::P2sh(_), AddressWitness::P2pkh { .. }) => {
                Err(ProtocolError::AddressWitnessError(
                    "P2SH address requires P2sh witness, got P2pkh".to_string(),
                ))
            }
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
                    return Err(ProtocolError::AddressWitnessError(format!(
                        "Unsupported P2SH script type: only standard multisig (OP_M ... OP_N OP_CHECKMULTISIG) is supported. \
                         First opcode was 0x{:02x}, expected OP_1 through OP_16",
                        byte
                    )));
                }
            }
            Some(Ok(dashcore::blockdata::script::Instruction::PushBytes(_))) => {
                return Err(ProtocolError::AddressWitnessError(
                    "Unsupported P2SH script type: only standard multisig is supported. \
                     Script starts with a data push instead of OP_M threshold."
                        .to_string(),
                ))
            }
            Some(Err(e)) => {
                return Err(ProtocolError::AddressWitnessError(format!(
                    "Error parsing P2SH script: {:?}",
                    e
                )))
            }
            None => {
                return Err(ProtocolError::AddressWitnessError(
                    "Empty P2SH redeem script".to_string(),
                ))
            }
        };

        // Read public keys until we hit OP_N
        loop {
            match instructions.next() {
                Some(Ok(dashcore::blockdata::script::Instruction::PushBytes(bytes))) => {
                    // Only compressed public keys (33 bytes) are allowed
                    let len = bytes.len();
                    if len != 33 {
                        return Err(ProtocolError::UncompressedPublicKeyNotAllowedError(
                            crate::consensus::signature::UncompressedPublicKeyNotAllowedError::new(
                                len,
                            ),
                        ));
                    }
                    pubkeys.push(bytes.as_bytes().to_vec());
                }
                Some(Ok(dashcore::blockdata::script::Instruction::Op(op))) => {
                    let byte = op.to_u8();
                    if byte >= OP_PUSHNUM_1.to_u8() && byte <= OP_PUSHNUM_16.to_u8() {
                        // This is OP_N, the total number of keys
                        let n = (byte - OP_PUSHNUM_1.to_u8() + 1) as usize;
                        if pubkeys.len() != n {
                            return Err(ProtocolError::AddressWitnessError(format!(
                                "Multisig script declares {} keys but contains {}",
                                n,
                                pubkeys.len()
                            )));
                        }
                        break;
                    } else if op == OP_CHECKMULTISIG || op == OP_CHECKMULTISIGVERIFY {
                        // Hit CHECKMULTISIG without seeing OP_N - malformed
                        return Err(ProtocolError::AddressWitnessError(
                            "Malformed multisig script: OP_CHECKMULTISIG before OP_N".to_string(),
                        ));
                    } else {
                        return Err(ProtocolError::AddressWitnessError(format!(
                            "Unsupported opcode 0x{:02x} in P2SH script. Only standard multisig is supported.",
                            byte
                        )));
                    }
                }
                Some(Err(e)) => {
                    return Err(ProtocolError::AddressWitnessError(format!(
                        "Error parsing multisig script: {:?}",
                        e
                    )))
                }
                None => {
                    return Err(ProtocolError::AddressWitnessError(
                        "Incomplete multisig script: unexpected end before OP_N".to_string(),
                    ))
                }
            }
        }

        // Validate threshold
        if threshold > pubkeys.len() {
            return Err(ProtocolError::AddressWitnessError(format!(
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
                        return Err(ProtocolError::AddressWitnessError(
                            "Multisig script has extra data after OP_CHECKMULTISIG".to_string(),
                        ));
                    }
                    Ok((threshold, pubkeys))
                } else if op == OP_CHECKMULTISIGVERIFY {
                    Err(ProtocolError::AddressWitnessError(
                        "OP_CHECKMULTISIGVERIFY is not supported, only OP_CHECKMULTISIG"
                            .to_string(),
                    ))
                } else {
                    Err(ProtocolError::AddressWitnessError(format!(
                        "Expected OP_CHECKMULTISIG, got opcode 0x{:02x}",
                        op.to_u8()
                    )))
                }
            }
            _ => Err(ProtocolError::AddressWitnessError(
                "Invalid multisig script: expected OP_CHECKMULTISIG after OP_N".to_string(),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Orchard shielded payment address (requires `shielded-bundle-building`)
// ---------------------------------------------------------------------------

/// Size of the Orchard diversifier (11 bytes).
#[cfg(feature = "shielded-bundle-building")]
pub const ORCHARD_DIVERSIFIER_SIZE: usize = 11;
/// Size of the Orchard diversified transmission key pk_d (32 bytes, Pallas curve point).
#[cfg(feature = "shielded-bundle-building")]
pub const ORCHARD_PKD_SIZE: usize = 32;
/// Total size of a raw Orchard payment address (43 bytes = diversifier + pk_d).
#[cfg(feature = "shielded-bundle-building")]
pub const ORCHARD_ADDRESS_SIZE: usize = ORCHARD_DIVERSIFIER_SIZE + ORCHARD_PKD_SIZE;

/// An Orchard shielded payment address.
///
/// Composed of a diversifier (11 bytes) and a diversified transmission key (32 bytes).
/// The diversifier enables a single spending key to derive an unlimited number of
/// unlinkable payment addresses. Only the holder of the corresponding FullViewingKey
/// (or IncomingViewingKey) can link diversified addresses to the same wallet.
///
/// Bech32m encoding uses type byte `0x10`, producing addresses that start with `z`:
/// - Mainnet: `dash1z...`
/// - Testnet: `tdash1z...`
///
/// The raw Orchard address format matches Zcash Orchard (43 bytes), but the
/// string encoding is Dash-specific (no F4Jumble, no Unified Address wrapper).
///
/// Use [`From<PaymentAddress>`] to convert from the `orchard` crate's native type,
/// or [`to_payment_address()`](OrchardAddress::to_payment_address) to convert back
/// (with pk_d validation).
///
/// Requires the `shielded-bundle-building` feature.
#[cfg(feature = "shielded-bundle-building")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrchardAddress {
    /// 11-byte diversifier derived from the FullViewingKey with an index.
    diversifier: [u8; ORCHARD_DIVERSIFIER_SIZE],
    /// 32-byte diversified transmission key (point on the Pallas curve).
    pk_d: [u8; ORCHARD_PKD_SIZE],
}

#[cfg(feature = "shielded-bundle-building")]
impl OrchardAddress {
    /// Type byte for Orchard addresses in bech32m encoding (user-facing).
    /// Produces 'z' as the first bech32 character.
    pub const ORCHARD_TYPE: u8 = 0x10;

    /// Creates an OrchardAddress from its raw components.
    pub fn from_parts(
        diversifier: [u8; ORCHARD_DIVERSIFIER_SIZE],
        pk_d: [u8; ORCHARD_PKD_SIZE],
    ) -> Self {
        Self { diversifier, pk_d }
    }

    /// Creates an OrchardAddress from a 43-byte raw address.
    ///
    /// The first 11 bytes are the diversifier, the next 32 are pk_d.
    /// No validation is performed on pk_d; use [`From<PaymentAddress>`]
    /// for a pre-validated address.
    pub fn from_raw_bytes(bytes: &[u8; ORCHARD_ADDRESS_SIZE]) -> Self {
        let mut diversifier = [0u8; ORCHARD_DIVERSIFIER_SIZE];
        let mut pk_d = [0u8; ORCHARD_PKD_SIZE];
        diversifier.copy_from_slice(&bytes[..ORCHARD_DIVERSIFIER_SIZE]);
        pk_d.copy_from_slice(&bytes[ORCHARD_DIVERSIFIER_SIZE..]);
        Self { diversifier, pk_d }
    }

    /// Returns the raw 43-byte address (diversifier || pk_d).
    pub fn to_raw_bytes(&self) -> [u8; ORCHARD_ADDRESS_SIZE] {
        let mut bytes = [0u8; ORCHARD_ADDRESS_SIZE];
        bytes[..ORCHARD_DIVERSIFIER_SIZE].copy_from_slice(&self.diversifier);
        bytes[ORCHARD_DIVERSIFIER_SIZE..].copy_from_slice(&self.pk_d);
        bytes
    }

    /// Returns the 11-byte diversifier.
    pub fn diversifier(&self) -> &[u8; ORCHARD_DIVERSIFIER_SIZE] {
        &self.diversifier
    }

    /// Returns the 32-byte diversified transmission key.
    pub fn pk_d(&self) -> &[u8; ORCHARD_PKD_SIZE] {
        &self.pk_d
    }

    /// Encodes the OrchardAddress as a bech32m string for the specified network.
    ///
    /// Format: `<HRP>1<data-part>`
    /// - Data: type_byte (0x10) || diversifier (11 bytes) || pk_d (32 bytes)
    /// - Total payload: 44 bytes
    /// - Checksum: bech32m (BIP-350)
    ///
    /// # Example
    /// ```ignore
    /// let address = OrchardAddress::from_raw_bytes(&raw_bytes);
    /// let encoded = address.to_bech32m_string(Network::Dash);
    /// // Returns something like "dash1z..."
    /// ```
    pub fn to_bech32m_string(&self, network: Network) -> String {
        let hrp_str = PlatformAddress::hrp_for_network(network);
        let hrp = Hrp::parse(hrp_str).expect("HRP is valid");

        let mut payload = Vec::with_capacity(1 + ORCHARD_ADDRESS_SIZE);
        payload.push(Self::ORCHARD_TYPE);
        payload.extend_from_slice(&self.diversifier);
        payload.extend_from_slice(&self.pk_d);

        bech32::encode::<Bech32m>(hrp, &payload).expect("encoding should succeed")
    }

    /// Converts this address to an Orchard [`PaymentAddress`](grovedb_commitment_tree::PaymentAddress).
    ///
    /// Returns an error if `pk_d` is not a valid Pallas curve point.
    pub fn to_payment_address(
        &self,
    ) -> Result<grovedb_commitment_tree::PaymentAddress, ProtocolError> {
        crate::shielded::builder::orchard_address_to_payment_address(self)
    }

    /// Decodes a bech32m-encoded Orchard address string.
    ///
    /// # Returns
    /// - `Ok((OrchardAddress, Network))` - The decoded address and its network
    /// - `Err(ProtocolError)` - If the address is invalid
    pub fn from_bech32m_string(s: &str) -> Result<(Self, Network), ProtocolError> {
        let (hrp, data) =
            bech32::decode(s).map_err(|e| ProtocolError::DecodingError(format!("{}", e)))?;

        let hrp_lower = hrp.as_str().to_ascii_lowercase();
        let network = match hrp_lower.as_str() {
            s if s == PLATFORM_HRP_MAINNET => Network::Dash,
            s if s == PLATFORM_HRP_TESTNET => Network::Testnet,
            _ => {
                return Err(ProtocolError::DecodingError(format!(
                    "invalid HRP '{}': expected '{}' or '{}'",
                    hrp, PLATFORM_HRP_MAINNET, PLATFORM_HRP_TESTNET
                )))
            }
        };

        // Validate payload: 1 type byte + 11 diversifier + 32 pk_d = 44 bytes
        if data.len() != 1 + ORCHARD_ADDRESS_SIZE {
            return Err(ProtocolError::DecodingError(format!(
                "invalid Orchard address length: expected {} bytes, got {}",
                1 + ORCHARD_ADDRESS_SIZE,
                data.len()
            )));
        }

        if data[0] != Self::ORCHARD_TYPE {
            return Err(ProtocolError::DecodingError(format!(
                "invalid Orchard address type byte: expected 0x{:02x}, got 0x{:02x}",
                Self::ORCHARD_TYPE,
                data[0]
            )));
        }

        let mut diversifier = [0u8; ORCHARD_DIVERSIFIER_SIZE];
        let mut pk_d = [0u8; ORCHARD_PKD_SIZE];
        diversifier.copy_from_slice(&data[1..1 + ORCHARD_DIVERSIFIER_SIZE]);
        pk_d.copy_from_slice(&data[1 + ORCHARD_DIVERSIFIER_SIZE..]);

        Ok((Self { diversifier, pk_d }, network))
    }
}

/// Infallible conversion from the orchard crate's `PaymentAddress` to `OrchardAddress`.
///
/// Extracts the raw 43 bytes (diversifier || pk_d) from the validated address.
#[cfg(feature = "shielded-bundle-building")]
impl From<grovedb_commitment_tree::PaymentAddress> for OrchardAddress {
    fn from(addr: grovedb_commitment_tree::PaymentAddress) -> Self {
        Self::from_raw_bytes(&addr.to_raw_address_bytes())
    }
}

/// Infallible conversion from a reference to `PaymentAddress`.
#[cfg(feature = "shielded-bundle-building")]
impl From<&grovedb_commitment_tree::PaymentAddress> for OrchardAddress {
    fn from(addr: &grovedb_commitment_tree::PaymentAddress) -> Self {
        Self::from_raw_bytes(&addr.to_raw_address_bytes())
    }
}

#[cfg(feature = "shielded-bundle-building")]
impl std::fmt::Display for OrchardAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Orchard(d={}, pk_d={})",
            hex::encode(self.diversifier),
            hex::encode(self.pk_d)
        )
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

/// Error type for parsing a bech32m-encoded Platform address
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAddressParseError(pub String);

impl std::fmt::Display for PlatformAddressParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlatformAddressParseError {}

impl FromStr for PlatformAddress {
    type Err = PlatformAddressParseError;

    /// Parses a bech32m-encoded Platform address string.
    ///
    /// This accepts addresses with either mainnet ("dash") or testnet ("tdash") HRP.
    /// The network information is discarded; use `from_bech32m_string` if you need
    /// to preserve the network.
    ///
    /// # Example
    /// ```ignore
    /// let address: PlatformAddress = "dash1k...".parse()?;
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bech32m_string(s)
            .map(|(addr, _network)| addr)
            .map_err(|e| PlatformAddressParseError(e.to_string()))
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
    fn test_platform_address_from_private_key() {
        // Create a keypair
        let seed = [1u8; 32];
        let (secret_key, public_key) = create_keypair(seed);

        // Create PrivateKey from the secret key
        let private_key = PrivateKey::new(secret_key, Network::Testnet);

        // Derive address using From<&PrivateKey>
        let address_from_private = PlatformAddress::from(&private_key);

        // Derive address manually using pubkey_hash() which computes Hash160(pubkey)
        // Hash160 = RIPEMD160(SHA256(x)), the standard Bitcoin P2PKH derivation
        let pubkey_hash = public_key.pubkey_hash();
        let address_from_pubkey = PlatformAddress::P2pkh(*pubkey_hash.as_byte_array());

        // Both addresses should be identical
        assert_eq!(
            address_from_private, address_from_pubkey,
            "Address derived from private key should match Hash160(compressed_pubkey)"
        );

        // Verify it's a P2PKH address
        assert!(address_from_private.is_p2pkh());
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

        // Create witness (only signature needed - public key is recovered)
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
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
        };

        // Verify should fail (recovered pubkey won't match because message differs)
        let result = address.verify_bytes_against_witness(&witness, verify_bytes);
        assert!(
            result.is_err(),
            "P2PKH verification should fail with wrong data"
        );
    }

    #[test]
    fn test_p2pkh_verify_wrong_key_fails() {
        // Create two keypairs
        let seed1 = [1u8; 32];
        let seed2 = [2u8; 32];
        let (_secret_key1, public_key1) = create_keypair(seed1);
        let (secret_key2, _public_key2) = create_keypair(seed2);

        // Create P2PKH address from public key 1's hash
        let pubkey_hash = public_key1.pubkey_hash();
        let address = PlatformAddress::P2pkh(*pubkey_hash.as_byte_array());

        // Sign with key 2 (wrong key)
        let signable_bytes = b"test message";
        let signature = sign_data(signable_bytes, &secret_key2);

        // Create witness (signature is from key 2, but address is for key 1)
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
        };

        // Verify should fail (recovered pubkey hash won't match address)
        let result = address.verify_bytes_against_witness(&witness, signable_bytes);
        assert!(
            result.is_err(),
            "P2PKH verification should fail when signed with wrong key"
        );
    }

    // NOTE: test_uncompressed_public_key_rejected was removed because P2PKH witnesses
    // no longer include the public key - it's recovered from the signature during verification.
    // ECDSA recovery always produces a compressed public key (33 bytes).

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
        };
        let result = p2sh_address.verify_bytes_against_witness(&p2pkh_witness, signable_bytes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("P2SH address requires P2sh witness"));
    }

    // ========================
    // Bech32m encoding tests (DIP-0018)
    // ========================

    #[test]
    fn test_bech32m_p2pkh_mainnet_roundtrip() {
        // Test P2PKH address roundtrip on mainnet
        let hash: [u8; 20] = [
            0xf7, 0xda, 0x0a, 0x2b, 0x5c, 0xbd, 0x4f, 0xf6, 0xbb, 0x2c, 0x4d, 0x89, 0xb6, 0x7d,
            0x2f, 0x3f, 0xfe, 0xec, 0x05, 0x25,
        ];
        let address = PlatformAddress::P2pkh(hash);

        // Encode to bech32m
        let encoded = address.to_bech32m_string(Network::Dash);

        // Verify exact encoding
        assert_eq!(
            encoded, "dash1krma5z3ttj75la4m93xcndna9ullamq9y5e9n5rs",
            "P2PKH mainnet encoding mismatch"
        );

        // Decode and verify roundtrip
        let (decoded, network) =
            PlatformAddress::from_bech32m_string(&encoded).expect("decoding should succeed");
        assert_eq!(decoded, address);
        assert_eq!(network, Network::Dash);
    }

    #[test]
    fn test_bech32m_p2pkh_testnet_roundtrip() {
        // Test P2PKH address roundtrip on testnet
        let hash: [u8; 20] = [
            0xf7, 0xda, 0x0a, 0x2b, 0x5c, 0xbd, 0x4f, 0xf6, 0xbb, 0x2c, 0x4d, 0x89, 0xb6, 0x7d,
            0x2f, 0x3f, 0xfe, 0xec, 0x05, 0x25,
        ];
        let address = PlatformAddress::P2pkh(hash);

        // Encode to bech32m
        let encoded = address.to_bech32m_string(Network::Testnet);

        // Verify exact encoding
        assert_eq!(
            encoded, "tdash1krma5z3ttj75la4m93xcndna9ullamq9y5fzq2j7",
            "P2PKH testnet encoding mismatch"
        );

        // Decode and verify roundtrip
        let (decoded, network) =
            PlatformAddress::from_bech32m_string(&encoded).expect("decoding should succeed");
        assert_eq!(decoded, address);
        assert_eq!(network, Network::Testnet);
    }

    #[test]
    fn test_bech32m_p2sh_mainnet_roundtrip() {
        // Test P2SH address roundtrip on mainnet
        let hash: [u8; 20] = [
            0x43, 0xfa, 0x18, 0x3c, 0xf3, 0xfb, 0x6e, 0x9e, 0x7d, 0xc6, 0x2b, 0x69, 0x2a, 0xeb,
            0x4f, 0xc8, 0xd8, 0x04, 0x56, 0x36,
        ];
        let address = PlatformAddress::P2sh(hash);

        // Encode to bech32m
        let encoded = address.to_bech32m_string(Network::Dash);

        // Verify exact encoding
        assert_eq!(
            encoded, "dash1sppl5xpu70aka8nacc4kj2htflydspzkxch4cad6",
            "P2SH mainnet encoding mismatch"
        );

        // Decode and verify roundtrip
        let (decoded, network) =
            PlatformAddress::from_bech32m_string(&encoded).expect("decoding should succeed");
        assert_eq!(decoded, address);
        assert_eq!(network, Network::Dash);
    }

    #[test]
    fn test_bech32m_p2sh_testnet_roundtrip() {
        // Test P2SH address roundtrip on testnet
        let hash: [u8; 20] = [
            0x43, 0xfa, 0x18, 0x3c, 0xf3, 0xfb, 0x6e, 0x9e, 0x7d, 0xc6, 0x2b, 0x69, 0x2a, 0xeb,
            0x4f, 0xc8, 0xd8, 0x04, 0x56, 0x36,
        ];
        let address = PlatformAddress::P2sh(hash);

        // Encode to bech32m
        let encoded = address.to_bech32m_string(Network::Testnet);

        // Verify exact encoding
        assert_eq!(
            encoded, "tdash1sppl5xpu70aka8nacc4kj2htflydspzkxc8jtru5",
            "P2SH testnet encoding mismatch"
        );

        // Decode and verify roundtrip
        let (decoded, network) =
            PlatformAddress::from_bech32m_string(&encoded).expect("decoding should succeed");
        assert_eq!(decoded, address);
        assert_eq!(network, Network::Testnet);
    }

    #[test]
    fn test_bech32m_devnet_uses_testnet_hrp() {
        let hash: [u8; 20] = [0xAB; 20];
        let address = PlatformAddress::P2pkh(hash);

        // Devnet should use testnet HRP
        let encoded = address.to_bech32m_string(Network::Devnet);
        assert!(
            encoded.starts_with("tdash1"),
            "Devnet address should start with 'tdash1', got: {}",
            encoded
        );
    }

    #[test]
    fn test_bech32m_regtest_uses_testnet_hrp() {
        let hash: [u8; 20] = [0xAB; 20];
        let address = PlatformAddress::P2pkh(hash);

        // Regtest should use testnet HRP
        let encoded = address.to_bech32m_string(Network::Regtest);
        assert!(
            encoded.starts_with("tdash1"),
            "Regtest address should start with 'tdash1', got: {}",
            encoded
        );
    }

    #[test]
    fn test_bech32m_invalid_hrp_fails() {
        // Create a valid bech32m address with wrong HRP using the bech32 crate directly
        let wrong_hrp = Hrp::parse("bitcoin").unwrap();
        let payload: [u8; 21] = [0x00; 21];
        let wrong_hrp_address = bech32::encode::<Bech32m>(wrong_hrp, &payload).unwrap();

        let result = PlatformAddress::from_bech32m_string(&wrong_hrp_address);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid HRP"),
            "Error should mention invalid HRP: {}",
            err
        );
    }

    #[test]
    fn test_bech32m_invalid_checksum_fails() {
        // Create a valid address, then corrupt the checksum
        let hash: [u8; 20] = [0xAB; 20];
        let address = PlatformAddress::P2pkh(hash);
        let mut encoded = address.to_bech32m_string(Network::Dash);

        // Corrupt the last character (part of checksum)
        let last_char = encoded.pop().unwrap();
        let corrupted_char = if last_char == 'q' { 'p' } else { 'q' };
        encoded.push(corrupted_char);

        let result = PlatformAddress::from_bech32m_string(&encoded);
        assert!(result.is_err(), "Should fail with corrupted checksum");
    }

    #[test]
    fn test_bech32m_invalid_type_byte_fails() {
        // Manually construct an address with invalid type byte (0x02)
        // We need to use the bech32 crate directly for this
        let hrp = Hrp::parse("dash").unwrap();
        let invalid_payload: [u8; 21] = [0x02; 21]; // type byte 0x02 is invalid
        let encoded = bech32::encode::<Bech32m>(hrp, &invalid_payload).unwrap();

        let result = PlatformAddress::from_bech32m_string(&encoded);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid address type"),
            "Error should mention invalid type: {}",
            err
        );
    }

    #[test]
    fn test_bech32m_too_short_fails() {
        // Construct an address with too few bytes
        let hrp = Hrp::parse("dash").unwrap();
        let short_payload: [u8; 10] = [0xb0; 10]; // Only 10 bytes instead of 21
        let encoded = bech32::encode::<Bech32m>(hrp, &short_payload).unwrap();

        let result = PlatformAddress::from_bech32m_string(&encoded);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid Platform address length"),
            "Error should mention invalid length: {}",
            err
        );
    }

    #[test]
    fn test_bech32m_from_str_trait() {
        // Test the FromStr trait implementation
        let hash: [u8; 20] = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
        ];
        let original = PlatformAddress::P2pkh(hash);

        // Encode and then parse via FromStr
        let encoded = original.to_bech32m_string(Network::Testnet);
        let parsed: PlatformAddress = encoded.parse().expect("parsing should succeed");

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_bech32m_case_insensitive() {
        // Per DIP-0018, addresses must be lowercase or uppercase (not mixed)
        // The bech32 crate should handle this
        let hash: [u8; 20] = [0xAB; 20];
        let address = PlatformAddress::P2pkh(hash);

        let lowercase = address.to_bech32m_string(Network::Dash);
        let uppercase = lowercase.to_uppercase();

        // Both should decode to the same address
        let (decoded_lower, _) = PlatformAddress::from_bech32m_string(&lowercase).unwrap();
        let (decoded_upper, _) = PlatformAddress::from_bech32m_string(&uppercase).unwrap();

        assert_eq!(decoded_lower, decoded_upper);
        assert_eq!(decoded_lower, address);
    }

    #[test]
    fn test_bech32m_all_zeros_p2pkh() {
        // Edge case: all-zero hash
        let address = PlatformAddress::P2pkh([0u8; 20]);
        let encoded = address.to_bech32m_string(Network::Dash);
        let (decoded, _) = PlatformAddress::from_bech32m_string(&encoded).unwrap();
        assert_eq!(decoded, address);
    }

    #[test]
    fn test_bech32m_all_ones_p2sh() {
        // Edge case: all-ones hash
        let address = PlatformAddress::P2sh([0xFF; 20]);
        let encoded = address.to_bech32m_string(Network::Dash);
        let (decoded, _) = PlatformAddress::from_bech32m_string(&encoded).unwrap();
        assert_eq!(decoded, address);
    }

    #[test]
    fn test_hrp_for_network() {
        assert_eq!(PlatformAddress::hrp_for_network(Network::Dash), "dash");
        assert_eq!(PlatformAddress::hrp_for_network(Network::Testnet), "tdash");
        assert_eq!(PlatformAddress::hrp_for_network(Network::Devnet), "tdash");
        assert_eq!(PlatformAddress::hrp_for_network(Network::Regtest), "tdash");
    }

    #[test]
    fn test_storage_bytes_format() {
        // Verify that to_bytes() (using bincode) produces expected format:
        // [variant_index (1 byte)] + [hash (20 bytes)]
        // P2pkh = variant 0, P2sh = variant 1
        let p2pkh = PlatformAddress::P2pkh([0xAB; 20]);
        let p2sh = PlatformAddress::P2sh([0xCD; 20]);

        let p2pkh_bytes = p2pkh.to_bytes();
        let p2sh_bytes = p2sh.to_bytes();

        // Verify format: 21 bytes total, first byte is variant index
        assert_eq!(p2pkh_bytes.len(), 21);
        assert_eq!(p2sh_bytes.len(), 21);
        assert_eq!(p2pkh_bytes[0], 0x00, "P2pkh variant index must be 0x00");
        assert_eq!(p2sh_bytes[0], 0x01, "P2sh variant index must be 0x01");

        // Verify roundtrip through from_bytes
        let p2pkh_decoded = PlatformAddress::from_bytes(&p2pkh_bytes).unwrap();
        let p2sh_decoded = PlatformAddress::from_bytes(&p2sh_bytes).unwrap();
        assert_eq!(p2pkh_decoded, p2pkh);
        assert_eq!(p2sh_decoded, p2sh);
    }

    #[test]
    fn test_bech32m_uses_different_type_bytes_than_storage() {
        // Verify that bech32m encoding uses type bytes (0xb0/0x80)
        // while storage (bincode) uses variant indices (0x00/0x01)
        let p2pkh = PlatformAddress::P2pkh([0xAB; 20]);
        let p2sh = PlatformAddress::P2sh([0xCD; 20]);

        // Storage bytes (bincode) use variant indices 0x00/0x01
        assert_eq!(p2pkh.to_bytes()[0], 0x00);
        assert_eq!(p2sh.to_bytes()[0], 0x01);

        // Bech32m encoding uses 0xb0/0xb8 (verified by successful roundtrip)
        let p2pkh_encoded = p2pkh.to_bech32m_string(Network::Dash);
        let p2sh_encoded = p2sh.to_bech32m_string(Network::Dash);

        let (p2pkh_decoded, _) = PlatformAddress::from_bech32m_string(&p2pkh_encoded).unwrap();
        let (p2sh_decoded, _) = PlatformAddress::from_bech32m_string(&p2sh_encoded).unwrap();

        assert_eq!(p2pkh_decoded, p2pkh);
        assert_eq!(p2sh_decoded, p2sh);
    }

    // ========================
    // Orchard address tests (require shielded-bundle-building feature)
    // ========================

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_address_from_parts_roundtrip() {
        let diversifier = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ];
        let pk_d = [0xAB; 32];
        let address = OrchardAddress::from_parts(diversifier, pk_d);

        assert_eq!(address.diversifier(), &diversifier);
        assert_eq!(address.pk_d(), &pk_d);

        let raw = address.to_raw_bytes();
        assert_eq!(raw.len(), 43);
        assert_eq!(&raw[..11], &diversifier);
        assert_eq!(&raw[11..], &pk_d[..]);

        let recovered = OrchardAddress::from_raw_bytes(&raw);
        assert_eq!(recovered, address);
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_bech32m_mainnet_roundtrip() {
        let diversifier = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ];
        let pk_d = [0xAB; 32];
        let address = OrchardAddress::from_parts(diversifier, pk_d);

        let encoded = address.to_bech32m_string(Network::Dash);
        assert!(
            encoded.starts_with("dash1z"),
            "Orchard mainnet address should start with 'dash1z', got: {}",
            encoded
        );

        let (decoded, network) =
            OrchardAddress::from_bech32m_string(&encoded).expect("decoding should succeed");
        assert_eq!(decoded, address);
        assert_eq!(network, Network::Dash);
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_bech32m_testnet_roundtrip() {
        let diversifier = [0xFF; 11];
        let pk_d = [0x42; 32];
        let address = OrchardAddress::from_parts(diversifier, pk_d);

        let encoded = address.to_bech32m_string(Network::Testnet);
        assert!(
            encoded.starts_with("tdash1z"),
            "Orchard testnet address should start with 'tdash1z', got: {}",
            encoded
        );

        let (decoded, network) =
            OrchardAddress::from_bech32m_string(&encoded).expect("decoding should succeed");
        assert_eq!(decoded, address);
        assert_eq!(network, Network::Testnet);
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_bech32m_wrong_type_byte_fails() {
        // Manually construct an address with P2PKH type byte (0xb0) but 44-byte payload
        let hrp = Hrp::parse("dash").unwrap();
        let mut payload = vec![PlatformAddress::P2PKH_TYPE]; // Wrong type byte
        payload.extend_from_slice(&[0u8; 43]);
        let encoded = bech32::encode::<Bech32m>(hrp, &payload).unwrap();

        let result = OrchardAddress::from_bech32m_string(&encoded);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid Orchard address type byte"));
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_bech32m_wrong_length_fails() {
        // Too short (only 20 bytes instead of 43)
        let hrp = Hrp::parse("dash").unwrap();
        let mut payload = vec![OrchardAddress::ORCHARD_TYPE];
        payload.extend_from_slice(&[0u8; 20]);
        let encoded = bech32::encode::<Bech32m>(hrp, &payload).unwrap();

        let result = OrchardAddress::from_bech32m_string(&encoded);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid Orchard address length"));
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_and_platform_addresses_are_distinguishable() {
        // Verify that the type bytes produce distinct prefixes
        let p2pkh = PlatformAddress::P2pkh([0xAB; 20]);
        let p2sh = PlatformAddress::P2sh([0xAB; 20]);
        let orchard = OrchardAddress::from_parts([0xAB; 11], [0xAB; 32]);

        let p2pkh_enc = p2pkh.to_bech32m_string(Network::Dash);
        let p2sh_enc = p2sh.to_bech32m_string(Network::Dash);
        let orchard_enc = orchard.to_bech32m_string(Network::Dash);

        // All three start with "dash1" but have different type-byte characters
        assert!(p2pkh_enc.starts_with("dash1k"), "P2PKH: {}", p2pkh_enc);
        assert!(p2sh_enc.starts_with("dash1s"), "P2SH: {}", p2sh_enc);
        assert!(
            orchard_enc.starts_with("dash1z"),
            "Orchard: {}",
            orchard_enc
        );

        // Cross-decoding should fail
        assert!(PlatformAddress::from_bech32m_string(&orchard_enc).is_err());
        assert!(OrchardAddress::from_bech32m_string(&p2pkh_enc).is_err());
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_address_all_zeros() {
        let address = OrchardAddress::from_parts([0u8; 11], [0u8; 32]);
        let encoded = address.to_bech32m_string(Network::Dash);
        let (decoded, _) = OrchardAddress::from_bech32m_string(&encoded).unwrap();
        assert_eq!(decoded, address);
    }

    #[cfg(feature = "shielded-bundle-building")]
    #[test]
    fn test_orchard_address_display() {
        let address = OrchardAddress::from_parts([0x01; 11], [0x02; 32]);
        let display = format!("{}", address);
        assert!(display.starts_with("Orchard(d="));
        assert!(display.contains("pk_d="));
    }
}
