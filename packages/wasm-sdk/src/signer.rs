//! Signer functionality for WASM SDK
//!
//! This module provides signing capabilities for state transitions in a browser environment.
//! It supports both BLS and ECDSA signatures.

use dpp::identity::{KeyType, Purpose};
use dpp::prelude::Identifier;
use js_sys::{Array, Object, Reflect, Uint8Array};
use web_sys::CryptoKey;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Signer interface for WASM
#[wasm_bindgen]
pub struct WasmSigner {
    /// Private keys by public key ID
    private_keys: HashMap<u32, PrivateKeyInfo>,
    /// Identity ID this signer is associated with
    identity_id: Option<Identifier>,
}

#[derive(Clone)]
struct PrivateKeyInfo {
    private_key: Vec<u8>,
    key_type: KeyType,
    _purpose: Purpose,
}

#[wasm_bindgen]
impl WasmSigner {
    /// Create a new signer
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmSigner {
        WasmSigner {
            private_keys: HashMap::new(),
            identity_id: None,
        }
    }

    /// Set the identity ID for this signer
    #[wasm_bindgen(js_name = setIdentityId)]
    pub fn set_identity_id(&mut self, identity_id: &str) -> Result<(), JsError> {
        let id = Identifier::from_string(
            identity_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
        
        self.identity_id = Some(id);
        Ok(())
    }

    /// Add a private key to the signer
    #[wasm_bindgen(js_name = addPrivateKey)]
    pub fn add_private_key(
        &mut self,
        public_key_id: u32,
        private_key: Vec<u8>,
        key_type: &str,
        purpose: u32,
    ) -> Result<(), JsError> {
        let key_type = match key_type {
            "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
            "BLS12_381" => KeyType::BLS12_381,
            "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
            "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
            "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
            _ => return Err(JsError::new(&format!("Unknown key type: {}", key_type))),
        };

        let purpose = match purpose {
            0 => Purpose::AUTHENTICATION,
            1 => Purpose::ENCRYPTION,
            2 => Purpose::DECRYPTION,
            3 => Purpose::TRANSFER,
            4 => Purpose::SYSTEM,
            5 => Purpose::VOTING,
            _ => return Err(JsError::new(&format!("Unknown purpose: {}", purpose))),
        };

        self.private_keys.insert(
            public_key_id,
            PrivateKeyInfo {
                private_key,
                key_type,
                _purpose: purpose,
            },
        );

        Ok(())
    }

    /// Remove a private key
    #[wasm_bindgen(js_name = removePrivateKey)]
    pub fn remove_private_key(&mut self, public_key_id: u32) -> bool {
        self.private_keys.remove(&public_key_id).is_some()
    }

    /// Sign data with a specific key
    #[wasm_bindgen(js_name = signData)]
    pub async fn sign_data(
        &self,
        data: Vec<u8>,
        public_key_id: u32,
    ) -> Result<Vec<u8>, JsError> {
        let key_info = self
            .private_keys
            .get(&public_key_id)
            .ok_or_else(|| JsError::new(&format!("Private key not found for ID: {}", public_key_id)))?;

        match key_info.key_type {
            KeyType::ECDSA_SECP256K1 => {
                // For ECDSA, we'll use Web Crypto API
                self.sign_ecdsa(&data, &key_info.private_key).await
            }
            KeyType::BLS12_381 => {
                // For BLS, we'll need to use a WASM BLS library
                self.sign_bls(&data, &key_info.private_key).await
            }
            _ => Err(JsError::new(&format!(
                "Signing not supported for key type: {:?}",
                key_info.key_type
            ))),
        }
    }

    /// Sign data using ECDSA
    async fn sign_ecdsa(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, JsError> {
        // Use Web Crypto API for ECDSA signing
        let window = web_sys::window()
            .ok_or_else(|| JsError::new("Window not available"))?;
        
        let crypto = window.crypto()
            .map_err(|_| JsError::new("Crypto not available"))?;
        
        let subtle = crypto.subtle();
        
        // Import the private key
        let key_data = Uint8Array::from(private_key);
        let algorithm = Object::new();
        Reflect::set(&algorithm, &"name".into(), &"ECDSA".into())
            .map_err(|_| JsError::new("Failed to set algorithm name"))?;
        Reflect::set(&algorithm, &"namedCurve".into(), &"P-256".into())
            .map_err(|_| JsError::new("Failed to set named curve"))?;
        
        let key_promise = subtle.import_key_with_object(
            "raw",
            &key_data,
            &algorithm,
            false,
            &Array::of1(&"sign".into()),
        )
        .map_err(|_| JsError::new("Failed to import key"))?;
        
        let key = JsFuture::from(key_promise).await
            .map_err(|e| JsError::new(&format!("Failed to import key: {:?}", e)))?;
        
        // Sign the data
        let sign_algorithm = Object::new();
        Reflect::set(&sign_algorithm, &"name".into(), &"ECDSA".into())
            .map_err(|_| JsError::new("Failed to set sign algorithm"))?;
        Reflect::set(&sign_algorithm, &"hash".into(), &"SHA-256".into())
            .map_err(|_| JsError::new("Failed to set hash algorithm"))?;
        
        let data_array = Uint8Array::from(data);
        let crypto_key = key.dyn_ref::<CryptoKey>()
            .ok_or_else(|| JsError::new("Invalid crypto key"))?;
        
        let signature_promise = subtle.sign_with_object_and_u8_array(
            &sign_algorithm,
            crypto_key,
            &data_array.to_vec(),
        )
        .map_err(|_| JsError::new("Failed to sign data"))?;
        
        let signature = JsFuture::from(signature_promise).await
            .map_err(|e| JsError::new(&format!("Failed to sign: {:?}", e)))?;
        
        // Convert signature to Vec<u8>
        let signature_array = Uint8Array::new(&signature);
        let mut signature_vec = vec![0; signature_array.length() as usize];
        signature_array.copy_to(&mut signature_vec);
        
        Ok(signature_vec)
    }

    /// Sign data using BLS
    async fn sign_bls(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, JsError> {
        // We need to check if BLS is available
        #[cfg(feature = "bls-signatures")]
        {
            // Use our BLS signing implementation
            use crate::bls::bls_sign;
            let sig_array = bls_sign(data, private_key)?;
            Ok(sig_array.to_vec())
        }
        #[cfg(not(feature = "bls-signatures"))]
        {
            // If BLS is not available at compile time, we'll implement a pure WASM solution
            // For now, return an error indicating BLS is not available
            Err(JsError::new("BLS signatures feature not enabled. Please enable the 'bls-signatures' feature in Cargo.toml"))
        }
    }

    /// Get the number of keys in the signer
    #[wasm_bindgen(js_name = getKeyCount)]
    pub fn get_key_count(&self) -> usize {
        self.private_keys.len()
    }

    /// Check if a key exists
    #[wasm_bindgen(js_name = hasKey)]
    pub fn has_key(&self, public_key_id: u32) -> bool {
        self.private_keys.contains_key(&public_key_id)
    }

    /// Get all key IDs
    #[wasm_bindgen(js_name = getKeyIds)]
    pub fn get_key_ids(&self) -> Vec<u32> {
        self.private_keys.keys().copied().collect()
    }
}

/// Browser-based signer that uses Web Crypto API
#[wasm_bindgen]
pub struct BrowserSigner {
    /// Key handles from Web Crypto API
    crypto_keys: HashMap<u32, JsValue>,
}

#[wasm_bindgen]
impl BrowserSigner {
    /// Create a new browser signer
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserSigner {
        BrowserSigner {
            crypto_keys: HashMap::new(),
        }
    }

    /// Generate a new key pair
    #[wasm_bindgen(js_name = generateKeyPair)]
    pub async fn generate_key_pair(
        &mut self,
        key_type: &str,
        public_key_id: u32,
    ) -> Result<JsValue, JsError> {
        let window = web_sys::window()
            .ok_or_else(|| JsError::new("Window not available"))?;
        
        let crypto = window.crypto()
            .map_err(|_| JsError::new("Crypto not available"))?;
        
        let subtle = crypto.subtle();
        
        let algorithm = match key_type {
            "ECDSA_SECP256K1" => {
                let algo = Object::new();
                Reflect::set(&algo, &"name".into(), &"ECDSA".into())
                    .map_err(|_| JsError::new("Failed to set algorithm"))?;
                Reflect::set(&algo, &"namedCurve".into(), &"P-256".into())
                    .map_err(|_| JsError::new("Failed to set curve"))?;
                algo
            }
            _ => return Err(JsError::new(&format!("Unsupported key type: {}", key_type))),
        };
        
        let usages = Array::of2(&"sign".into(), &"verify".into());
        
        let key_pair_promise = subtle.generate_key_with_object(
            &algorithm,
            true, // extractable
            &usages,
        )
        .map_err(|_| JsError::new("Failed to generate key pair"))?;
        
        let key_pair = JsFuture::from(key_pair_promise).await
            .map_err(|e| JsError::new(&format!("Failed to generate key pair: {:?}", e)))?;
        
        // Store the private key
        let private_key = Reflect::get(&key_pair, &"privateKey".into())
            .map_err(|_| JsError::new("Failed to get private key"))?;
        
        self.crypto_keys.insert(public_key_id, private_key);
        
        // Return the public key
        let public_key = Reflect::get(&key_pair, &"publicKey".into())
            .map_err(|_| JsError::new("Failed to get public key"))?;
        
        Ok(public_key)
    }

    /// Sign data with a stored key
    #[wasm_bindgen(js_name = signWithStoredKey)]
    pub async fn sign_with_stored_key(
        &self,
        data: Vec<u8>,
        public_key_id: u32,
    ) -> Result<Vec<u8>, JsError> {
        let key = self
            .crypto_keys
            .get(&public_key_id)
            .ok_or_else(|| JsError::new(&format!("Key not found for ID: {}", public_key_id)))?;
        
        let window = web_sys::window()
            .ok_or_else(|| JsError::new("Window not available"))?;
        
        let crypto = window.crypto()
            .map_err(|_| JsError::new("Crypto not available"))?;
        
        let subtle = crypto.subtle();
        
        let algorithm = Object::new();
        Reflect::set(&algorithm, &"name".into(), &"ECDSA".into())
            .map_err(|_| JsError::new("Failed to set algorithm"))?;
        Reflect::set(&algorithm, &"hash".into(), &"SHA-256".into())
            .map_err(|_| JsError::new("Failed to set hash"))?;
        
        let data_array = Uint8Array::from(&data[..]);
        
        let crypto_key = key.dyn_ref::<CryptoKey>()
            .ok_or_else(|| JsError::new("Invalid crypto key"))?;
        
        let signature_promise = subtle.sign_with_object_and_u8_array(
            &algorithm,
            crypto_key,
            &data_array.to_vec(),
        )
        .map_err(|_| JsError::new("Failed to sign data"))?;
        
        let signature = JsFuture::from(signature_promise).await
            .map_err(|e| JsError::new(&format!("Failed to sign: {:?}", e)))?;
        
        // Convert to Vec<u8>
        let signature_array = Uint8Array::new(&signature);
        let mut signature_vec = vec![0; signature_array.length() as usize];
        signature_array.copy_to(&mut signature_vec);
        
        Ok(signature_vec)
    }
}

/// HD (Hierarchical Deterministic) key derivation for WASM
#[wasm_bindgen]
pub struct HDSigner {
    /// Mnemonic phrase
    mnemonic: String,
    /// Derivation path
    derivation_path: String,
}

#[wasm_bindgen]
impl HDSigner {
    /// Create a new HD signer from mnemonic
    #[wasm_bindgen(constructor)]
    pub fn new(mnemonic: &str, derivation_path: &str) -> Result<HDSigner, JsError> {
        // Validate mnemonic
        validate_mnemonic(mnemonic)?;
        
        // Validate derivation path format
        if !derivation_path.starts_with("m/") {
            return Err(JsError::new("Derivation path must start with 'm/'"));
        }
        
        Ok(HDSigner {
            mnemonic: mnemonic.to_string(),
            derivation_path: derivation_path.to_string(),
        })
    }

    /// Generate a new mnemonic
    #[wasm_bindgen(js_name = generateMnemonic)]
    pub fn generate_mnemonic(word_count: u32) -> Result<String, JsError> {
        let word_count = match word_count {
            12 | 15 | 18 | 21 | 24 => word_count,
            _ => return Err(JsError::new("Invalid word count. Use 12, 15, 18, 21, or 24")),
        };
        
        // Generate mnemonic using proper BIP39 implementation
        use crate::bip39::{MnemonicStrength, generate_mnemonic};
        
        let strength = match word_count {
            12 => MnemonicStrength::Words12,
            15 => MnemonicStrength::Words15,
            18 => MnemonicStrength::Words18,
            21 => MnemonicStrength::Words21,
            24 => MnemonicStrength::Words24,
            _ => return Err(JsError::new("Invalid word count")),
        };
        
        generate_mnemonic(Some(strength), None)
    }

    /// Derive a key at a specific index
    #[wasm_bindgen(js_name = deriveKey)]
    pub fn derive_key(&self, index: u32) -> Result<Vec<u8>, JsError> {
        // Derive HD key at specified index
        // In production, this would use proper BIP32 derivation
        
        // For now, create a deterministic key based on mnemonic and index
        use hex::encode;
        let seed_material = format!("{}-{}-{}", self.mnemonic, self.derivation_path, index);
        
        // Create a 32-byte key using a simple hash (in production, use proper KDF)
        let mut key = [0u8; 32];
        let hash = encode(seed_material.as_bytes());
        let hash_bytes = hash.as_bytes();
        
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = hash_bytes.get(i % hash_bytes.len()).copied().unwrap_or(0);
        }
        
        Ok(key.to_vec())
    }

    /// Get the derivation path
    #[wasm_bindgen(getter, js_name = derivationPath)]
    pub fn derivation_path(&self) -> String {
        self.derivation_path.clone()
    }
}

/// Validate a BIP39 mnemonic phrase
fn validate_mnemonic(mnemonic: &str) -> Result<(), JsError> {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    
    // Check word count
    let valid_counts = [12, 15, 18, 21, 24];
    if !valid_counts.contains(&words.len()) {
        return Err(JsError::new(&format!(
            "Invalid mnemonic length: {}. Must be one of: 12, 15, 18, 21, 24",
            words.len()
        )));
    }
    
    // Check that all words are lowercase and contain only a-z
    for word in &words {
        if word.is_empty() {
            return Err(JsError::new("Empty word in mnemonic"));
        }
        
        for ch in word.chars() {
            if !ch.is_ascii_lowercase() {
                return Err(JsError::new(&format!(
                    "Invalid character '{}' in word '{}'. Mnemonic words should only contain lowercase letters",
                    ch, word
                )));
            }
        }
        
        // Check word length (BIP39 words are typically 3-8 characters)
        if word.len() < 3 || word.len() > 8 {
            return Err(JsError::new(&format!(
                "Invalid word '{}'. BIP39 words are typically 3-8 characters long",
                word
            )));
        }
    }
    
    // Now we can use the proper BIP39 validation
    use crate::bip39::WordListLanguage;
    if !crate::bip39::validate_mnemonic(&mnemonic, Some(WordListLanguage::English)) {
        return Err(JsError::new("Invalid mnemonic phrase - failed BIP39 validation"));
    }
    
    Ok(())
}

