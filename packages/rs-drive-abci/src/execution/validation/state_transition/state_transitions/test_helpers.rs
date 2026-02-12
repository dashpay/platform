//! Shared test utilities for address-based state transition tests.
//!
//! This module provides common test infrastructure for testing state transitions
//! that involve platform addresses, including signers, address creation helpers,
//! balance setup utilities, and shielded pool state helpers.

use crate::config::{PlatformConfig, PlatformTestConfig};
use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::blockdata::script::ScriptBuf;
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::dashcore::secp256k1::{PublicKey as RawPublicKey, Secp256k1, SecretKey as RawSecretKey};
use dpp::dashcore::PublicKey;
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::prelude::AddressNonce;
use dpp::serialization::PlatformSerializable;
use dpp::shielded::SerializedAction;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use drive::drive::shielded::paths::{
    shielded_anchors_credit_pool_path, shielded_credit_pool_encrypted_notes_path,
    shielded_credit_pool_nullifiers_path, shielded_credit_pool_path, SHIELDED_TOTAL_BALANCE_KEY,
};
use drive::grovedb::Element;
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

// Re-export commonly used types for convenience
pub use dpp::dashcore::blockdata::opcodes::all::{
    OP_CHECKMULTISIG, OP_CHECKSIG, OP_DROP, OP_PUSHNUM_1, OP_RETURN,
};
pub use dpp::dashcore::blockdata::script::ScriptBuf as TestScriptBuf;
pub use dpp::dashcore::hashes::Hash as TestHash;
#[allow(unused_imports)]
pub use dpp::dashcore::secp256k1::Secp256k1 as TestSecp256k1;
#[allow(unused_imports)]
pub use dpp::dashcore::PublicKey as TestPublicKey;
pub use dpp::ProtocolError as TestProtocolError;

// ==========================================
// Test Infrastructure - Signer for Addresses
// ==========================================

/// A P2PKH key entry containing the secret key only
#[derive(Debug, Clone)]
pub struct P2pkhKeyEntry {
    pub secret_key: RawSecretKey,
}

/// A P2SH multisig entry containing multiple secret keys and the redeem script
#[derive(Debug, Clone)]
pub struct P2shMultisigEntry {
    pub threshold: u8,
    pub secret_keys: Vec<RawSecretKey>,
    pub redeem_script: Vec<u8>,
}

/// A test signer that can sign for P2PKH and P2SH multisig addresses
#[derive(Debug, Default)]
pub struct TestAddressSigner {
    pub p2pkh_keys: HashMap<[u8; 20], P2pkhKeyEntry>,
    pub p2sh_entries: HashMap<[u8; 20], P2shMultisigEntry>,
}

impl TestAddressSigner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_keypair(seed: [u8; 32]) -> (RawSecretKey, PublicKey) {
        let secp = Secp256k1::new();
        // Hash the seed to ensure it's always a valid secret key
        // (non-zero, less than curve order). Raw seeds like [0u8; 32] are invalid.
        let hashed_seed = sha256::Hash::hash(&seed);
        let secret_key =
            RawSecretKey::from_byte_array(hashed_seed.as_byte_array()).expect("valid secret key");
        let raw_public_key = RawPublicKey::from_secret_key(&secp, &secret_key);
        let public_key = PublicKey::new(raw_public_key);
        (secret_key, public_key)
    }

    pub fn sign_data(data: &[u8], secret_key: &RawSecretKey) -> Vec<u8> {
        dpp::dashcore::signer::sign(data, secret_key.as_ref())
            .expect("signing should succeed")
            .to_vec()
    }

    pub fn create_multisig_script(threshold: u8, pubkeys: &[PublicKey]) -> Vec<u8> {
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

    pub fn add_p2pkh(&mut self, seed: [u8; 32]) -> PlatformAddress {
        let (secret_key, public_key) = Self::create_keypair(seed);
        let pubkey_hash = *public_key.pubkey_hash().as_byte_array();
        self.p2pkh_keys
            .insert(pubkey_hash, P2pkhKeyEntry { secret_key });
        PlatformAddress::P2pkh(pubkey_hash)
    }

    pub fn add_p2sh_multisig(&mut self, threshold: u8, seeds: &[[u8; 32]]) -> PlatformAddress {
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
                redeem_script,
            },
        );

        PlatformAddress::P2sh(script_hash)
    }

    /// Get the private key bytes for a P2PKH address (for signing the transition)
    #[allow(dead_code)]
    pub fn get_p2pkh_private_key(&self, address: &PlatformAddress) -> Option<[u8; 32]> {
        match address {
            PlatformAddress::P2pkh(hash) => self
                .p2pkh_keys
                .get(hash)
                .map(|entry| entry.secret_key.secret_bytes()),
            _ => None,
        }
    }

    /// Convenience: Adds a 2-of-3 P2SH multisig address
    pub fn add_p2sh_2of3(
        &mut self,
        seed1: [u8; 32],
        seed2: [u8; 32],
        seed3: [u8; 32],
    ) -> PlatformAddress {
        self.add_p2sh_multisig(2, &[seed1, seed2, seed3])
    }

    /// Convenience: Adds a 1-of-1 P2SH multisig address
    pub fn add_p2sh_1of1(&mut self, seed: [u8; 32]) -> PlatformAddress {
        self.add_p2sh_multisig(1, &[seed])
    }

    /// Convenience: Adds a 3-of-3 P2SH multisig address
    pub fn add_p2sh_3of3(
        &mut self,
        seed1: [u8; 32],
        seed2: [u8; 32],
        seed3: [u8; 32],
    ) -> PlatformAddress {
        self.add_p2sh_multisig(3, &[seed1, seed2, seed3])
    }

    /// Convenience: Adds an n-of-n P2SH multisig address
    pub fn add_p2sh_n_of_n(&mut self, seeds: &[[u8; 32]]) -> PlatformAddress {
        self.add_p2sh_multisig(seeds.len() as u8, seeds)
    }

    /// Sign P2PKH and create witness
    pub fn sign_p2pkh(
        &self,
        address: PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.sign_create_witness(&address, data)
    }

    /// Sign P2SH and create witness
    #[allow(dead_code)]
    pub fn sign_p2sh(
        &self,
        address: PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.sign_create_witness(&address, data)
    }

    /// Sign P2SH with ALL keys (not just threshold)
    pub fn sign_p2sh_all_keys(
        &self,
        address: PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        match address {
            PlatformAddress::P2sh(hash) => {
                let entry = self.p2sh_entries.get(&hash).ok_or_else(|| {
                    ProtocolError::Generic(format!(
                        "No P2SH entry found for script hash {}",
                        hex::encode(hash)
                    ))
                })?;
                // Sign with ALL keys, not just threshold
                let signatures: Vec<BinaryData> = entry
                    .secret_keys
                    .iter()
                    .map(|sk| BinaryData::new(Self::sign_data(data, sk)))
                    .collect();

                Ok(AddressWitness::P2sh {
                    signatures,
                    redeem_script: BinaryData::new(entry.redeem_script.clone()),
                })
            }
            _ => Err(ProtocolError::Generic("Expected P2SH address".to_string())),
        }
    }

    /// Gets the P2SH entry for an address (for test manipulation)
    pub fn get_p2sh_entry(&self, hash: &[u8; 20]) -> Option<&P2shMultisigEntry> {
        self.p2sh_entries.get(hash)
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

// ==========================================
// Helper Functions
// ==========================================

/// Helper function to create a platform address from a seed (for addresses that don't need signing)
pub fn create_platform_address(seed: u8) -> PlatformAddress {
    let mut hash = [0u8; 20];
    hash[0] = seed;
    hash[19] = seed;
    PlatformAddress::P2pkh(hash)
}

/// Helper function to create a dummy P2PKH witness for testing structure validation
pub fn create_dummy_witness() -> AddressWitness {
    AddressWitness::P2pkh {
        signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
    }
}

/// Helper function to set up an address with balance and nonce in the drive
pub fn setup_address_with_balance(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    address: PlatformAddress,
    nonce: AddressNonce,
    balance: u64,
) {
    let platform_version = PlatformVersion::latest();
    let mut drive_operations = Vec::new();

    platform
        .drive
        .set_balance_to_address(
            address,
            nonce,
            balance,
            &mut None,
            &mut drive_operations,
            platform_version,
        )
        .expect("expected to set balance to address");

    platform
        .drive
        .apply_batch_low_level_drive_operations(
            None,
            None,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to apply drive operations");
}

/// Helper function to set up an address with balance and nonce in the drive,
/// and also add to system credits (needed for withdrawal tests which remove from system credits)
pub fn setup_address_with_balance_and_system_credits(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    address: PlatformAddress,
    nonce: AddressNonce,
    balance: u64,
) {
    let platform_version = PlatformVersion::latest();
    let mut drive_operations = Vec::new();

    // Add to system credits first (withdrawals remove from system credits)
    platform
        .drive
        .add_to_system_credits(balance, None, platform_version)
        .expect("expected to add to system credits");

    platform
        .drive
        .set_balance_to_address(
            address,
            nonce,
            balance,
            &mut None,
            &mut drive_operations,
            platform_version,
        )
        .expect("expected to set balance to address");

    platform
        .drive
        .apply_batch_low_level_drive_operations(
            None,
            None,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to apply drive operations");
}

// ==========================================
// Shielded Test Helpers
// ==========================================

/// Create a `SerializedAction` with syntactically valid sizes but meaningless crypto data.
/// Passes structure validation (correct field sizes) but will fail ZK proof verification.
pub fn create_dummy_serialized_action() -> SerializedAction {
    SerializedAction {
        nullifier: [1u8; 32],
        rk: [2u8; 32],
        cmx: [3u8; 32],
        encrypted_note: vec![4u8; 692], // epk(32) + enc(580) + out(80)
        cv_net: [5u8; 32],
        spend_auth_sig: [6u8; 64],
    }
}

/// Standard platform setup for shielded tests with instant lock signature verification disabled.
pub fn setup_platform() -> TempPlatform<MockCoreRPCLike> {
    let platform_config = PlatformConfig {
        testing_configs: PlatformTestConfig {
            disable_instant_lock_signature_verification: true,
            ..Default::default()
        },
        ..Default::default()
    };

    TestPlatformBuilder::new()
        .with_config(platform_config)
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state()
}

/// Execute a state transition through the full processing pipeline and return the result.
pub fn process_transition(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: StateTransition,
    platform_version: &PlatformVersion,
) -> StateTransitionsProcessingResult {
    let transition_bytes = transition
        .serialize_to_bytes()
        .expect("should serialize transition");
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();

    platform
        .platform
        .process_raw_state_transitions(
            &vec![transition_bytes],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process state transition")
}

/// Insert a fake anchor into the shielded anchors tree via GroveDB.
pub fn insert_anchor_into_state(platform: &TempPlatform<MockCoreRPCLike>, anchor: &[u8; 32]) {
    let platform_version = PlatformVersion::latest();
    let grove_version = &platform_version.drive.grove_version;
    let transaction = platform.drive.grove.start_transaction();
    let anchors_path = shielded_anchors_credit_pool_path();

    platform
        .drive
        .grove
        .insert(
            &anchors_path,
            anchor,
            Element::Item(vec![], None),
            None,
            Some(&transaction),
            grove_version,
        )
        .unwrap()
        .expect("should insert anchor");

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("should commit transaction");
}

/// Insert a nullifier into the nullifiers tree via GroveDB.
pub fn insert_nullifier_into_state(platform: &TempPlatform<MockCoreRPCLike>, nullifier: &[u8; 32]) {
    let platform_version = PlatformVersion::latest();
    let grove_version = &platform_version.drive.grove_version;
    let transaction = platform.drive.grove.start_transaction();
    let nullifiers_path = shielded_credit_pool_nullifiers_path();

    platform
        .drive
        .grove
        .insert(
            &nullifiers_path,
            nullifier,
            Element::Item(vec![], None),
            None,
            Some(&transaction),
            grove_version,
        )
        .unwrap()
        .expect("should insert nullifier");

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("should commit transaction");
}

/// Set the shielded pool total balance in GroveDB.
pub fn set_pool_total_balance(platform: &TempPlatform<MockCoreRPCLike>, balance: u64) {
    let platform_version = PlatformVersion::latest();
    let grove_version = &platform_version.drive.grove_version;
    let transaction = platform.drive.grove.start_transaction();
    let pool_path = shielded_credit_pool_path();

    platform
        .drive
        .grove
        .insert(
            &pool_path,
            &[SHIELDED_TOTAL_BALANCE_KEY],
            Element::new_sum_item(balance as i64),
            None,
            Some(&transaction),
            grove_version,
        )
        .unwrap()
        .expect("should set total balance");

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("should commit transaction");
}

/// Insert dummy encrypted notes into the shielded pool to meet the minimum
/// notes threshold for outgoing transitions.
pub fn insert_dummy_encrypted_notes(platform: &TempPlatform<MockCoreRPCLike>, count: u64) {
    let platform_version = PlatformVersion::latest();
    let grove_version = &platform_version.drive.grove_version;
    let transaction = platform.drive.grove.start_transaction();
    let notes_path = shielded_credit_pool_encrypted_notes_path();

    for i in 0..count {
        platform
            .drive
            .grove
            .insert(
                &notes_path,
                &i.to_be_bytes(),
                Element::Item(vec![0], None),
                None,
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .expect("should insert dummy note");
    }

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("should commit transaction");
}
