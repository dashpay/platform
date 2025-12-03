//! Comprehensive signing tests for AddressFundsTransferTransition
//!
//! Tests cover:
//! - P2PKH single input signing and verification
//! - P2SH multisig input signing and verification
//! - Mixed P2PKH and P2SH inputs
//! - Signature verification failures
//! - Serialization round-trip with signatures
//! - Edge cases and error conditions

use std::collections::{BTreeMap, HashMap};

use dashcore::blockdata::opcodes::all::*;
use dashcore::blockdata::script::ScriptBuf;
use dashcore::hashes::Hash;
use dashcore::secp256k1::{PublicKey as RawPublicKey, Secp256k1, SecretKey as RawSecretKey};
use dashcore::PublicKey;
use platform_value::BinaryData;

use crate::address_funds::{AddressWitness, PlatformAddress};
use crate::identity::signer::Signer;
use crate::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use crate::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// A P2PKH key entry containing secret key and public key
#[derive(Debug, Clone)]
struct P2pkhKeyEntry {
    secret_key: RawSecretKey,
    public_key: PublicKey,
}

/// A P2SH multisig entry containing multiple secret keys and the redeem script
#[derive(Debug, Clone)]
struct P2shMultisigEntry {
    /// The threshold (M in M-of-N)
    threshold: u8,
    /// Secret keys for all participants (may not have all if some participants are external)
    secret_keys: Vec<RawSecretKey>,
    /// Public keys for all participants
    public_keys: Vec<PublicKey>,
    /// The redeem script
    redeem_script: Vec<u8>,
}

/// A test signer that can sign for both P2PKH and P2SH addresses
#[derive(Debug, Default)]
struct TestAddressSigner {
    p2pkh_keys: HashMap<[u8; 20], P2pkhKeyEntry>,
    p2sh_entries: HashMap<[u8; 20], P2shMultisigEntry>,
}

impl TestAddressSigner {
    fn new() -> Self {
        Self::default()
    }

    /// Creates a keypair from a 32-byte seed
    fn create_keypair(seed: [u8; 32]) -> (RawSecretKey, PublicKey) {
        let secp = Secp256k1::new();
        let secret_key = RawSecretKey::from_byte_array(&seed).expect("valid secret key");
        let raw_public_key = RawPublicKey::from_secret_key(&secp, &secret_key);
        let public_key = PublicKey::new(raw_public_key);
        (secret_key, public_key)
    }

    /// Signs data with a secret key
    fn sign_data(data: &[u8], secret_key: &RawSecretKey) -> Vec<u8> {
        dashcore::signer::sign(data, secret_key.as_ref())
            .expect("signing should succeed")
            .to_vec()
    }

    /// Creates a standard multisig redeem script
    fn create_multisig_script(threshold: u8, pubkeys: &[PublicKey]) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_PUSHNUM_1.to_u8() + threshold - 1);
        for pubkey in pubkeys {
            let bytes = pubkey.to_bytes();
            script.push(bytes.len() as u8);
            script.extend_from_slice(&bytes);
        }
        script.push(OP_PUSHNUM_1.to_u8() + pubkeys.len() as u8 - 1);
        script.push(OP_CHECKMULTISIG.to_u8());
        script
    }

    /// Adds a P2PKH address with the given seed, returns the address
    fn add_p2pkh(&mut self, seed: [u8; 32]) -> PlatformAddress {
        let (secret_key, public_key) = Self::create_keypair(seed);
        let pubkey_hash = *public_key.pubkey_hash().as_byte_array();
        self.p2pkh_keys.insert(
            pubkey_hash,
            P2pkhKeyEntry {
                secret_key,
                public_key,
            },
        );
        PlatformAddress::P2pkh(pubkey_hash)
    }

    /// Adds a P2SH multisig address with the given seeds, returns the address
    fn add_p2sh_multisig(&mut self, threshold: u8, seeds: &[[u8; 32]]) -> PlatformAddress {
        let keypairs: Vec<_> = seeds.iter().map(|s| Self::create_keypair(*s)).collect();
        let secret_keys: Vec<_> = keypairs.iter().map(|(sk, _)| *sk).collect();
        let public_keys: Vec<_> = keypairs.iter().map(|(_, pk)| *pk).collect();

        let redeem_script = Self::create_multisig_script(threshold, &public_keys);
        let script_buf = ScriptBuf::from_bytes(redeem_script.clone());
        let script_hash = *script_buf.script_hash().as_byte_array();

        self.p2sh_entries.insert(
            script_hash,
            P2shMultisigEntry {
                threshold,
                secret_keys,
                public_keys,
                redeem_script,
            },
        );

        PlatformAddress::P2sh(script_hash)
    }
}

impl Signer<PlatformAddress> for TestAddressSigner {
    fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        match key {
            PlatformAddress::P2pkh(hash) => {
                let entry = self.p2pkh_keys.get(hash).ok_or_else(|| {
                    ProtocolError::Generic(format!(
                        "No P2PKH key found for address hash {}",
                        hex::encode(hash)
                    ))
                })?;
                let signature = Self::sign_data(data, &entry.secret_key);
                Ok(BinaryData::new(signature))
            }
            PlatformAddress::P2sh(hash) => {
                let entry = self.p2sh_entries.get(hash).ok_or_else(|| {
                    ProtocolError::Generic(format!(
                        "No P2SH entry found for script hash {}",
                        hex::encode(hash)
                    ))
                })?;
                // Return concatenated signatures for multisig
                let mut all_sigs = Vec::new();
                for sk in &entry.secret_keys[..entry.threshold as usize] {
                    all_sigs.extend(Self::sign_data(data, sk));
                }
                Ok(BinaryData::new(all_sigs))
            }
        }
    }

    fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        match key {
            PlatformAddress::P2pkh(hash) => {
                let entry = self.p2pkh_keys.get(hash).ok_or_else(|| {
                    ProtocolError::Generic(format!(
                        "No P2PKH key found for address hash {}",
                        hex::encode(hash)
                    ))
                })?;
                let signature = Self::sign_data(data, &entry.secret_key);
                // P2PKH witness only needs the signature - the public key is recovered
                // during verification, saving 33 bytes per witness
                Ok(AddressWitness::P2pkh {
                    signature: BinaryData::new(signature),
                })
            }
            PlatformAddress::P2sh(hash) => {
                let entry = self.p2sh_entries.get(hash).ok_or_else(|| {
                    ProtocolError::Generic(format!(
                        "No P2SH entry found for script hash {}",
                        hex::encode(hash)
                    ))
                })?;
                // Sign with threshold number of keys (first M keys)
                let signatures: Vec<BinaryData> = entry
                    .secret_keys
                    .iter()
                    .take(entry.threshold as usize)
                    .map(|sk| BinaryData::new(Self::sign_data(data, sk)))
                    .collect();

                Ok(AddressWitness::P2sh {
                    signatures,
                    redeem_script: BinaryData::new(entry.redeem_script.clone()),
                })
            }
        }
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        match key {
            PlatformAddress::P2pkh(hash) => self.p2pkh_keys.contains_key(hash),
            PlatformAddress::P2sh(hash) => self.p2sh_entries.contains_key(hash),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

/// Verifies all input witnesses against the transition's signable bytes
fn verify_transition_signatures(
    transition: &AddressFundsTransferTransitionV0,
) -> Result<(), ProtocolError> {
    let state_transition: StateTransition = transition.clone().into();
    let signable_bytes = state_transition.signable_bytes()?;

    let input_addresses: Vec<_> = transition.inputs.keys().collect();

    if input_addresses.len() != transition.input_witnesses.len() {
        return Err(ProtocolError::Generic(format!(
            "Witness count mismatch: {} inputs but {} witnesses",
            input_addresses.len(),
            transition.input_witnesses.len()
        )));
    }

    for (i, (address, witness)) in input_addresses
        .iter()
        .zip(transition.input_witnesses.iter())
        .enumerate()
    {
        let _operations = address
            .verify_bytes_against_witness(witness, &signable_bytes)
            .map_err(|e| {
                ProtocolError::Generic(format!("Witness {} verification failed: {}", i, e))
            })?;
    }

    Ok(())
}

// ============================================================================
// P2PKH Tests
// ============================================================================

#[test]
fn test_single_p2pkh_input_signing() {
    let mut signer = TestAddressSigner::new();

    // Create input address
    let input_address = signer.add_p2pkh([1u8; 32]);

    // Create output address (doesn't need to be in signer)
    let output_address = PlatformAddress::P2pkh([99u8; 20]);

    // Build inputs and outputs
    let mut inputs = BTreeMap::new();
    inputs.insert(input_address.clone(), (1u32, 1000u64)); // nonce: 1, credits: 1000

    let mut outputs = BTreeMap::new();
    outputs.insert(output_address, 900u64);

    // Create signed transition
    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    // Extract the V0 transition
    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify we have one witness
    assert_eq!(transition.input_witnesses.len(), 1);

    // Verify the witness is P2PKH
    assert!(transition.input_witnesses[0].is_p2pkh());

    // Verify signature is valid
    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_multiple_p2pkh_inputs_signing() {
    let mut signer = TestAddressSigner::new();

    // Create multiple input addresses
    let input1 = signer.add_p2pkh([1u8; 32]);
    let input2 = signer.add_p2pkh([2u8; 32]);
    let input3 = signer.add_p2pkh([3u8; 32]);

    // Create output address
    let output = PlatformAddress::P2pkh([99u8; 20]);

    // Build inputs (multiple inputs)
    let mut inputs = BTreeMap::new();
    inputs.insert(input1.clone(), (1u32, 500u64));
    inputs.insert(input2.clone(), (1u32, 300u64));
    inputs.insert(input3.clone(), (1u32, 200u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    // Create signed transition
    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify we have three witnesses
    assert_eq!(transition.input_witnesses.len(), 3);

    // Verify all witnesses are P2PKH
    for witness in &transition.input_witnesses {
        assert!(witness.is_p2pkh());
    }

    // Verify all signatures are valid
    verify_transition_signatures(&transition).expect("all signatures should be valid");
}

// ============================================================================
// P2SH Multisig Tests
// ============================================================================

#[test]
fn test_single_p2sh_2_of_3_multisig_input_signing() {
    let mut signer = TestAddressSigner::new();

    // Create 2-of-3 multisig input
    let input_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);

    // Create output address
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input_address.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    // Create signed transition
    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify we have one witness
    assert_eq!(transition.input_witnesses.len(), 1);

    // Verify the witness is P2SH
    assert!(transition.input_witnesses[0].is_p2sh());

    // Verify the witness has 2 signatures (threshold)
    if let AddressWitness::P2sh { signatures, .. } = &transition.input_witnesses[0] {
        assert_eq!(signatures.len(), 2);
    } else {
        panic!("Expected P2SH witness");
    }

    // Verify signatures are valid
    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_p2sh_3_of_5_multisig_input_signing() {
    let mut signer = TestAddressSigner::new();

    // Create 3-of-5 multisig input
    let input_address = signer.add_p2sh_multisig(
        3,
        &[[20u8; 32], [21u8; 32], [22u8; 32], [23u8; 32], [24u8; 32]],
    );

    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input_address.clone(), (1u32, 5000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 4500u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify the witness has 3 signatures (threshold)
    if let AddressWitness::P2sh { signatures, .. } = &transition.input_witnesses[0] {
        assert_eq!(signatures.len(), 3);
    } else {
        panic!("Expected P2SH witness");
    }

    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_multiple_p2sh_inputs_signing() {
    let mut signer = TestAddressSigner::new();

    // Create two different multisig addresses
    let input1 = signer.add_p2sh_multisig(2, &[[30u8; 32], [31u8; 32], [32u8; 32]]);
    let input2 = signer.add_p2sh_multisig(1, &[[40u8; 32], [41u8; 32]]); // 1-of-2

    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input1.clone(), (1u32, 1000u64));
    inputs.insert(input2.clone(), (1u32, 500u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 1400u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify both witnesses are P2SH
    assert_eq!(transition.input_witnesses.len(), 2);
    for witness in &transition.input_witnesses {
        assert!(witness.is_p2sh());
    }

    verify_transition_signatures(&transition).expect("signatures should be valid");
}

// ============================================================================
// Mixed P2PKH and P2SH Tests
// ============================================================================

#[test]
fn test_mixed_p2pkh_and_p2sh_inputs() {
    let mut signer = TestAddressSigner::new();

    // Create mixed inputs
    let p2pkh_input = signer.add_p2pkh([50u8; 32]);
    let p2sh_input = signer.add_p2sh_multisig(2, &[[60u8; 32], [61u8; 32], [62u8; 32]]);

    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(p2pkh_input.clone(), (1u32, 1000u64));
    inputs.insert(p2sh_input.clone(), (1u32, 2000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 2800u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify we have two witnesses
    assert_eq!(transition.input_witnesses.len(), 2);

    // Verify we have one of each type (order depends on BTreeMap ordering)
    let p2pkh_count = transition
        .input_witnesses
        .iter()
        .filter(|w| w.is_p2pkh())
        .count();
    let p2sh_count = transition
        .input_witnesses
        .iter()
        .filter(|w| w.is_p2sh())
        .count();
    assert_eq!(p2pkh_count, 1);
    assert_eq!(p2sh_count, 1);

    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_complex_mixed_inputs_multiple_outputs() {
    let mut signer = TestAddressSigner::new();

    // Create various inputs
    let p2pkh1 = signer.add_p2pkh([70u8; 32]);
    let p2pkh2 = signer.add_p2pkh([71u8; 32]);
    let p2sh1 = signer.add_p2sh_multisig(2, &[[80u8; 32], [81u8; 32], [82u8; 32]]);
    let p2sh2 = signer.add_p2sh_multisig(3, &[[90u8; 32], [91u8; 32], [92u8; 32], [93u8; 32]]);

    // Create multiple outputs
    let output1 = PlatformAddress::P2pkh([100u8; 20]);
    let output2 = PlatformAddress::P2sh([101u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(p2pkh1.clone(), (1u32, 1000u64));
    inputs.insert(p2pkh2.clone(), (1u32, 2000u64));
    inputs.insert(p2sh1.clone(), (1u32, 3000u64));
    inputs.insert(p2sh2.clone(), (1u32, 4000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output1, 5000u64);
    outputs.insert(output2, 4500u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Verify we have four witnesses
    assert_eq!(transition.input_witnesses.len(), 4);

    // Verify signature counts
    let p2pkh_count = transition
        .input_witnesses
        .iter()
        .filter(|w| w.is_p2pkh())
        .count();
    let p2sh_count = transition
        .input_witnesses
        .iter()
        .filter(|w| w.is_p2sh())
        .count();
    assert_eq!(p2pkh_count, 2);
    assert_eq!(p2sh_count, 2);

    verify_transition_signatures(&transition).expect("all signatures should be valid");
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_signed_transition_serialization_roundtrip() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2pkh([1u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Serialize
    let serialized = AddressFundsTransferTransitionV0::serialize_to_bytes(&transition)
        .expect("should serialize");

    // Deserialize
    let deserialized = AddressFundsTransferTransitionV0::deserialize_from_bytes(&serialized)
        .expect("should deserialize");

    // Verify equality
    assert_eq!(transition, deserialized);

    // Verify signatures still valid after round-trip
    verify_transition_signatures(&deserialized).expect("signatures should still be valid");
}

#[test]
fn test_multisig_transition_serialization_roundtrip() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    let serialized = AddressFundsTransferTransitionV0::serialize_to_bytes(&transition)
        .expect("should serialize");

    let deserialized = AddressFundsTransferTransitionV0::deserialize_from_bytes(&serialized)
        .expect("should deserialize");

    assert_eq!(transition, deserialized);
    verify_transition_signatures(&deserialized).expect("signatures should still be valid");
}

#[test]
fn test_mixed_transition_serialization_roundtrip() {
    let mut signer = TestAddressSigner::new();

    let p2pkh = signer.add_p2pkh([50u8; 32]);
    let p2sh = signer.add_p2sh_multisig(2, &[[60u8; 32], [61u8; 32], [62u8; 32]]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(p2pkh.clone(), (1u32, 1000u64));
    inputs.insert(p2sh.clone(), (1u32, 2000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 2800u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    let serialized = AddressFundsTransferTransitionV0::serialize_to_bytes(&transition)
        .expect("should serialize");

    let deserialized = AddressFundsTransferTransitionV0::deserialize_from_bytes(&serialized)
        .expect("should deserialize");

    assert_eq!(transition, deserialized);
    verify_transition_signatures(&deserialized).expect("signatures should still be valid");
}

// ============================================================================
// Verification Failure Tests
// ============================================================================

#[test]
fn test_tampered_inputs_verification_fails() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2pkh([1u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output.clone(), 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs.clone(),
        outputs.clone(),
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let mut transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Tamper with the transition by modifying credits
    let original_witnesses = transition.input_witnesses.clone();
    transition.inputs.insert(input.clone(), (1u32, 2000u64)); // Changed credits

    // Re-add original witnesses (they were signed for different data)
    transition.input_witnesses = original_witnesses;

    // Verification should fail
    let result = verify_transition_signatures(&transition);
    assert!(
        result.is_err(),
        "Verification should fail for tampered transition"
    );
}

#[test]
fn test_tampered_outputs_verification_fails() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2pkh([1u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output.clone(), 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let mut transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Tamper with outputs
    transition.outputs.insert(output.clone(), 950u64); // Changed output amount

    // Verification should fail
    let result = verify_transition_signatures(&transition);
    assert!(
        result.is_err(),
        "Verification should fail for tampered outputs"
    );
}

#[test]
fn test_wrong_witness_for_address_fails() {
    let mut signer = TestAddressSigner::new();

    let input1 = signer.add_p2pkh([1u8; 32]);
    let input2 = signer.add_p2pkh([2u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input1.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs.clone(),
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let mut transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Replace input with a different address but keep the same witness
    transition.inputs.clear();
    transition.inputs.insert(input2.clone(), (1u32, 1000u64));

    // Verification should fail (witness public key doesn't match new address)
    let result = verify_transition_signatures(&transition);
    assert!(
        result.is_err(),
        "Verification should fail when witness doesn't match address"
    );
}

#[test]
fn test_missing_witness_fails() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2pkh([1u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let mut transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Remove witness
    transition.input_witnesses.clear();

    // Verification should fail
    let result = verify_transition_signatures(&transition);
    assert!(
        result.is_err(),
        "Verification should fail when witness is missing"
    );
}

#[test]
fn test_p2sh_insufficient_signatures_fails() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let mut transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    // Remove one signature from the P2SH witness
    if let AddressWitness::P2sh {
        signatures,
        redeem_script,
    } = &transition.input_witnesses[0]
    {
        let modified_signatures = vec![signatures[0].clone()]; // Only keep one signature
        transition.input_witnesses[0] = AddressWitness::P2sh {
            signatures: modified_signatures,
            redeem_script: redeem_script.clone(),
        };
    }

    // Verification should fail
    let result = verify_transition_signatures(&transition);
    assert!(
        result.is_err(),
        "Verification should fail with insufficient signatures"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_1_of_1_multisig() {
    let mut signer = TestAddressSigner::new();

    // 1-of-1 multisig is essentially a P2SH wrapped P2PK
    let input = signer.add_p2sh_multisig(1, &[[1u8; 32]]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    if let AddressWitness::P2sh { signatures, .. } = &transition.input_witnesses[0] {
        assert_eq!(signatures.len(), 1);
    }

    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_high_threshold_multisig() {
    let mut signer = TestAddressSigner::new();

    // 5-of-5 requires all signers
    let input =
        signer.add_p2sh_multisig(5, &[[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 10000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 9500u64);

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    if let AddressWitness::P2sh { signatures, .. } = &transition.input_witnesses[0] {
        assert_eq!(signatures.len(), 5);
    }

    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_signer_cannot_sign_unknown_address() {
    let signer = TestAddressSigner::new(); // Empty signer

    let unknown_address = PlatformAddress::P2pkh([1u8; 20]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(unknown_address.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    // Should fail because signer doesn't have the key
    let result = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        0,
        get_platform_version(),
    );

    assert!(
        result.is_err(),
        "Should fail when signer doesn't have key for address"
    );
}

#[test]
fn test_can_sign_with_check() {
    let mut signer = TestAddressSigner::new();

    let known_address = signer.add_p2pkh([1u8; 32]);
    let unknown_address = PlatformAddress::P2pkh([99u8; 20]);

    assert!(signer.can_sign_with(&known_address));
    assert!(!signer.can_sign_with(&unknown_address));
}

#[test]
fn test_user_fee_increase_preserved() {
    let mut signer = TestAddressSigner::new();

    let input = signer.add_p2pkh([1u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    let mut inputs = BTreeMap::new();
    inputs.insert(input.clone(), (1u32, 1000u64));

    let mut outputs = BTreeMap::new();
    outputs.insert(output, 900u64);

    let user_fee_increase = 50u16;

    let state_transition = AddressFundsTransferTransitionV0::try_from_inputs_with_signer(
        inputs,
        outputs,
        vec![],
        &signer,
        user_fee_increase,
        get_platform_version(),
    )
    .expect("should create signed transition");

    let transition = match state_transition {
        StateTransition::AddressFundsTransfer(t) => match t {
            crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0,
        },
        _ => panic!("Expected AddressFundsTransfer transition"),
    };

    assert_eq!(transition.user_fee_increase, user_fee_increase);
    verify_transition_signatures(&transition).expect("signatures should be valid");
}

#[test]
fn test_different_nonces_produce_different_signable_bytes() {
    let mut signer = TestAddressSigner::new();
    let input = signer.add_p2pkh([1u8; 32]);
    let output = PlatformAddress::P2pkh([99u8; 20]);

    // First transition with nonce 1
    let mut inputs1 = BTreeMap::new();
    inputs1.insert(input.clone(), (1u32, 1000u64));
    let mut outputs1 = BTreeMap::new();
    outputs1.insert(output.clone(), 900u64);

    let transition1 = AddressFundsTransferTransitionV0 {
        inputs: inputs1,
        outputs: outputs1,
        fee_strategy: vec![],
        user_fee_increase: 0,
        input_witnesses: vec![],
    };

    // Second transition with nonce 2
    let mut inputs2 = BTreeMap::new();
    inputs2.insert(input.clone(), (2u32, 1000u64)); // Different nonce
    let mut outputs2 = BTreeMap::new();
    outputs2.insert(output.clone(), 900u64);

    let transition2 = AddressFundsTransferTransitionV0 {
        inputs: inputs2,
        outputs: outputs2,
        fee_strategy: vec![],
        user_fee_increase: 0,
        input_witnesses: vec![],
    };

    // Get signable bytes for both
    let st1: StateTransition = transition1.into();
    let st2: StateTransition = transition2.into();

    let bytes1 = st1.signable_bytes().expect("should get signable bytes");
    let bytes2 = st2.signable_bytes().expect("should get signable bytes");

    // They should be different due to different nonces
    assert_ne!(
        bytes1, bytes2,
        "Different nonces should produce different signable bytes"
    );
}
