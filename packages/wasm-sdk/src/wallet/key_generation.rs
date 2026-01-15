//! Key generation functionality for wallets
//!
//! Provides key generation and address derivation without full HD wallet support

use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::hashes::{sha256, Hash};
use dash_sdk::dpp::dashcore::secp256k1::{Secp256k1, SecretKey};
use dash_sdk::dpp::dashcore::{Address, Network, PrivateKey, PublicKey};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wasm_dpp2::NetworkWasm;

/// Key pair information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPair {
    /// Private key in WIF format
    pub private_key_wif: String,
    /// Private key in hex format
    pub private_key_hex: String,
    /// Public key in hex format
    pub public_key: String,
    /// Address for the key
    pub address: String,
    /// Network (mainnet/testnet)
    pub network: String,
}

#[wasm_bindgen(js_name = "KeyPair")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPairWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "privateKeyWif")]
    pub private_key_wif: String,
    #[wasm_bindgen(getter_with_clone, js_name = "privateKeyHex")]
    pub private_key_hex: String,
    #[wasm_bindgen(getter_with_clone, js_name = "publicKey")]
    pub public_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub address: String,
    #[wasm_bindgen(getter_with_clone)]
    pub network: String,
}

impl From<KeyPair> for KeyPairWasm {
    fn from(value: KeyPair) -> Self {
        Self {
            private_key_wif: value.private_key_wif,
            private_key_hex: value.private_key_hex,
            public_key: value.public_key,
            address: value.address,
            network: value.network,
        }
    }
}

impl_wasm_serde_conversions!(KeyPairWasm);

#[wasm_bindgen]
impl WasmSdk {
    fn build_key_pair(
        private_key: &PrivateKey,
        network: Network,
        network_label: &str,
    ) -> Result<KeyPair, WasmSdkError> {
        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        let public_key_bytes = public_key.inner.serialize();
        let address = Address::p2pkh(&public_key, network);

        Ok(KeyPair {
            private_key_wif: private_key.to_wif(),
            private_key_hex: hex::encode(private_key.inner.secret_bytes()),
            public_key: hex::encode(public_key_bytes),
            address: address.to_string(),
            network: network_label.to_string(),
        })
    }

    /// Generate a new random key pair
    #[wasm_bindgen(js_name = "generateKeyPair")]
    pub fn generate_key_pair(
        #[wasm_bindgen(unchecked_param_type = "NetworkLike")] network: JsValue,
    ) -> Result<KeyPairWasm, WasmSdkError> {
        let network_wasm = NetworkWasm::try_from(&network)?;
        let net: Network = network_wasm.into();

        // Generate random 32 bytes
        let mut key_bytes = [0u8; 32];
        getrandom::getrandom(&mut key_bytes).map_err(|e| {
            WasmSdkError::generic(format!("Failed to generate random bytes: {}", e))
        })?;

        // Create private key
        let private_key = PrivateKey::from_byte_array(&key_bytes, net)
            .map_err(|e| WasmSdkError::generic(format!("Failed to create private key: {}", e)))?;

        // Ensure secret key is valid before building info
        SecretKey::from_slice(&key_bytes)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;

        let key_pair = Self::build_key_pair(&private_key, net, network_wasm.as_str())?;
        Ok(KeyPairWasm::from(key_pair))
    }

    /// Generate multiple key pairs
    #[wasm_bindgen(js_name = "generateKeyPairs")]
    pub fn generate_key_pairs(
        #[wasm_bindgen(unchecked_param_type = "NetworkLike")] network: JsValue,
        count: u32,
    ) -> Result<Vec<KeyPairWasm>, WasmSdkError> {
        if count == 0 || count > 100 {
            return Err(WasmSdkError::invalid_argument(
                "Count must be between 1 and 100",
            ));
        }

        let mut pairs = Vec::new();
        for _ in 0..count {
            pairs.push(Self::generate_key_pair(network.clone())?);
        }
        Ok(pairs)
    }

    /// Create key pair from private key WIF
    #[wasm_bindgen(js_name = "keyPairFromWif")]
    pub fn key_pair_from_wif(
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: &str,
    ) -> Result<KeyPairWasm, WasmSdkError> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid WIF: {}", e)))?;

        let network_wasm: NetworkWasm = private_key.network.into();
        let key_pair =
            Self::build_key_pair(&private_key, private_key.network, network_wasm.as_str())?;
        Ok(KeyPairWasm::from(key_pair))
    }

    /// Create key pair from private key hex
    #[wasm_bindgen(js_name = "keyPairFromHex")]
    pub fn key_pair_from_hex(
        #[wasm_bindgen(js_name = "privateKeyHex")] private_key_hex: &str,
        #[wasm_bindgen(unchecked_param_type = "NetworkLike")] network: JsValue,
    ) -> Result<KeyPairWasm, WasmSdkError> {
        if private_key_hex.len() != 64 {
            return Err(WasmSdkError::invalid_argument(
                "Private key hex must be exactly 64 characters",
            ));
        }

        let network_wasm = NetworkWasm::try_from(&network)?;
        let net: Network = network_wasm.into();

        let key_bytes = hex::decode(private_key_hex)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid hex: {}", e)))?;

        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| WasmSdkError::invalid_argument("Private key bytes must be 32 bytes"))?;
        let private_key = PrivateKey::from_byte_array(&key_array, net)
            .map_err(|e| WasmSdkError::generic(format!("Failed to create private key: {}", e)))?;

        let key_pair = Self::build_key_pair(&private_key, net, network_wasm.as_str())?;
        Ok(KeyPairWasm::from(key_pair))
    }

    /// Get address from public key
    #[wasm_bindgen(js_name = "pubkeyToAddress")]
    pub fn pubkey_to_address(
        #[wasm_bindgen(js_name = "pubkeyHex")] pubkey_hex: &str,
        #[wasm_bindgen(unchecked_param_type = "NetworkLike")] network: JsValue,
    ) -> Result<String, WasmSdkError> {
        let network_wasm = NetworkWasm::try_from(&network)?;
        let net: Network = network_wasm.into();

        let pubkey_bytes = hex::decode(pubkey_hex)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid hex: {}", e)))?;

        let public_key = PublicKey::from_slice(&pubkey_bytes)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid public key: {}", e)))?;

        let address = Address::p2pkh(&public_key, net);
        Ok(address.to_string())
    }

    /// Validate a Dash address
    #[wasm_bindgen(js_name = "validateAddress")]
    pub fn validate_address(
        address: &str,
        #[wasm_bindgen(unchecked_param_type = "NetworkLike")] network: JsValue,
    ) -> bool {
        let Ok(network_wasm) = NetworkWasm::try_from(&network) else {
            return false;
        };
        let net: Network = network_wasm.into();

        Address::from_str(address)
            .map(|addr| *addr.network() == net)
            .unwrap_or(false)
    }

    /// Sign a message with a private key
    #[wasm_bindgen(js_name = "signMessage")]
    pub fn sign_message(
        message: &str,
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: &str,
    ) -> Result<String, WasmSdkError> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid WIF: {}", e)))?;

        // Create message hash
        let message_bytes = message.as_bytes();
        let hash = sha256::Hash::hash(message_bytes);

        // Sign the hash
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&private_key.inner.secret_bytes())
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;

        let message_hash =
            dash_sdk::dpp::dashcore::secp256k1::Message::from_digest(hash.to_byte_array());
        let signature = secp.sign_ecdsa(&message_hash, &secret_key);

        Ok(hex::encode(signature.serialize_compact()))
    }

    /// Generate deterministic test identity keys for SDK functional tests.
    ///
    /// This generates the same keys that are created in the genesis state when
    /// SDK_TEST_DATA=true is set. The seed should match the first byte of the
    /// identity ID (1, 2, or 3 for the test identities).
    ///
    /// Returns an array of objects containing:
    /// - keyId: The identity key ID
    /// - privateKeyHex: The 32-byte private key in hex format
    /// - publicKeyData: The public key data in hex (33 bytes for ECDSA_SECP256K1, 20 bytes for ECDSA_HASH160)
    /// - keyType: The key type (e.g., "ECDSA_SECP256K1", "ECDSA_HASH160")
    /// - purpose: The key purpose (e.g., "AUTHENTICATION", "TRANSFER")
    /// - securityLevel: The security level (e.g., "MASTER", "CRITICAL", "HIGH")
    ///
    /// Key indices:
    /// - 0: MASTER level AUTHENTICATION key (ECDSA_SECP256K1)
    /// - 1: CRITICAL level AUTHENTICATION key (ECDSA_SECP256K1)
    /// - 2: HIGH level AUTHENTICATION key (ECDSA_SECP256K1)
    /// - 3: CRITICAL level TRANSFER key (ECDSA_HASH160) - for credit transfers
    #[wasm_bindgen(js_name = "generateTestIdentityKeys")]
    pub fn generate_test_identity_keys(seed: u64) -> Result<JsValue, WasmSdkError> {
        use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dash_sdk::dpp::identity::IdentityPublicKey;
        use dash_sdk::dpp::version::LATEST_PLATFORM_VERSION;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut rng = StdRng::seed_from_u64(seed);
        let platform_version = LATEST_PLATFORM_VERSION;

        // Generate 3 authentication keys (master, critical, high)
        let mut keys = IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
            3,
            &mut rng,
            platform_version,
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to generate keys: {}", e)))?;

        // Add a TRANSFER purpose key (key id 3) for identity credit transfers
        // This matches what's created in the genesis state
        let transfer_key = IdentityPublicKey::random_masternode_transfer_key_with_rng(
            3, // key id 3 (after master=0, critical=1, high=2)
            &mut rng,
            platform_version,
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to generate transfer key: {}", e)))?;
        keys.push(transfer_key);

        let result: Vec<serde_json::Value> = keys
            .into_iter()
            .map(|(key, private_key_bytes)| {
                serde_json::json!({
                    "keyId": key.id(),
                    "privateKeyHex": hex::encode(private_key_bytes),
                    "publicKeyData": hex::encode(key.data().as_slice()),
                    "keyType": format!("{:?}", key.key_type()),
                    "purpose": format!("{:?}", key.purpose()),
                    "securityLevel": format!("{:?}", key.security_level()),
                })
            })
            .collect();

        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| WasmSdkError::generic(format!("Serialization error: {}", e)))
    }
}
