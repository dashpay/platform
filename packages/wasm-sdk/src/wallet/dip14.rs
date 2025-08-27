//! DIP14 - Extended HD Key Derivation using 256-bit Unsigned Integers
//! 
//! This module implements DIP14, which extends BIP32 to support 256-bit derivation indices
//! instead of the standard 31-bit limitation.

use dash_sdk::key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
use dash_sdk::dpp::dashcore::secp256k1::{self, Secp256k1, SecretKey, PublicKey, Scalar};
use dash_sdk::dpp::dashcore::Network;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::convert::TryInto;
use dash_sdk::dpp::dashcore::hashes::{sha256, ripemd160, Hash};
use hex;
use dash_sdk::dpp::dashcore;
use dash_sdk::key_wallet;

type HmacSha512 = Hmac<Sha512>;

/// Serialize a 256-bit unsigned integer as a 32-byte sequence, most significant byte first
fn ser256(value: &[u8; 32]) -> [u8; 32] {
    *value
}

/// Custom error type for DIP14 operations
#[derive(Debug)]
pub enum Dip14Error {
    InvalidKey,
    InvalidIndex,
    DerivationFailed(String),
}

impl std::fmt::Display for Dip14Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dip14Error::InvalidKey => write!(f, "Invalid key"),
            Dip14Error::InvalidIndex => write!(f, "Invalid index"),
            Dip14Error::DerivationFailed(msg) => write!(f, "Derivation failed: {}", msg),
        }
    }
}

impl std::error::Error for Dip14Error {}

/// DIP14 Extended Private Key
pub struct Dip14ExtendedPrivKey {
    pub network: Network,
    pub depth: u8,
    pub parent_fingerprint: [u8; 4],
    pub child_number: [u8; 32], // 256-bit child number
    pub chain_code: [u8; 32],
    pub private_key: SecretKey,
}

// DIP14 Version bytes
const DIP14_VERSION_MAINNET_PRIVATE: [u8; 4] = [0x04, 0x9e, 0xdd, 0x93]; // dpms
const DIP14_VERSION_MAINNET_PUBLIC: [u8; 4] = [0x04, 0x9e, 0xdc, 0x93];  // dpmp
const DIP14_VERSION_TESTNET_PRIVATE: [u8; 4] = [0x04, 0xa5, 0x90, 0x8e]; // dpts
const DIP14_VERSION_TESTNET_PUBLIC: [u8; 4] = [0x04, 0xa5, 0x8f, 0x51];  // dptp

impl Dip14ExtendedPrivKey {
    /// Create from a standard BIP32 ExtendedPrivKey (for master key)
    pub fn from_bip32(key: &ExtendedPrivKey) -> Self {
        let mut child_number = [0u8; 32];
        // For BIP32 compatibility, store the 32-bit child number in the last 4 bytes
        let child_u32: u32 = key.child_number.into();
        child_number[28..32].copy_from_slice(&child_u32.to_be_bytes());
        
        Self {
            network: key.network,
            depth: key.depth,
            parent_fingerprint: key.parent_fingerprint.as_ref().try_into().unwrap(),
            child_number,
            chain_code: key.chain_code.as_ref().try_into().unwrap(),
            private_key: key.private_key,
        }
    }
    
    /// Convert to standard BIP32 ExtendedPrivKey (only valid for indices < 2^32-1)
    pub fn to_bip32(&self) -> Result<ExtendedPrivKey, Dip14Error> {
        // Check if the child number fits in 32 bits
        if !self.child_number[0..28].iter().all(|&b| b == 0) {
            return Err(Dip14Error::InvalidIndex);
        }
        
        let child_bytes: [u8; 4] = self.child_number[28..32].try_into()
            .map_err(|_| Dip14Error::InvalidIndex)?;
        let child_number = key_wallet::bip32::ChildNumber::from(u32::from_be_bytes(child_bytes));
        
        Ok(ExtendedPrivKey {
            network: self.network,
            depth: self.depth,
            parent_fingerprint: key_wallet::bip32::Fingerprint::from_bytes(self.parent_fingerprint),
            child_number,
            chain_code: key_wallet::bip32::ChainCode::from_bytes(self.chain_code),
            private_key: self.private_key,
        })
    }
    
    /// Derive a child key using DIP14 extended derivation
    pub fn derive_child(&self, index: &[u8; 32], hardened: bool) -> Result<Self, Dip14Error> {
        let secp = Secp256k1::new();
        
        // Prepare HMAC input based on hardened flag
        let mut hmac = HmacSha512::new_from_slice(&self.chain_code)
            .map_err(|_| Dip14Error::DerivationFailed("Invalid chain code".to_string()))?;
        
        if hardened {
            // Hardened: 0x00 || ser256(k_parent) || ser256(i)
            hmac.update(&[0x00]);
            hmac.update(&self.private_key.secret_bytes());
            hmac.update(&ser256(index));
        } else {
            // Non-hardened: ser_P(point(k_parent)) || ser256(i)
            let public_key = PublicKey::from_secret_key(&secp, &self.private_key);
            hmac.update(&public_key.serialize());
            hmac.update(&ser256(index));
        }
        
        let result = hmac.finalize().into_bytes();
        let (il_bytes, child_chain_code) = result.split_at(32);
        
        // Perform scalar addition: child_key = IL + parent_key (mod n)
        // This is the core of BIP32/DIP14 child key derivation
        
        // First, try to create a secret key from IL
        let il_scalar = match SecretKey::from_slice(il_bytes) {
            Ok(key) => key,
            Err(_) => return Err(Dip14Error::DerivationFailed("Invalid IL bytes (IL >= n)".to_string())),
        };
        
        // Add parent key to IL
        // In secp256k1, we perform scalar addition: child_key = parent_key + IL (mod n)
        // Convert IL to a Scalar for the tweak operation
        let il_scalar_bytes = il_scalar.secret_bytes();
        let tweak = Scalar::from_be_bytes(il_scalar_bytes)
            .map_err(|_| Dip14Error::DerivationFailed("Failed to convert IL to scalar".to_string()))?;
        
        // Debug: log parent and IL values before addition
        web_sys::console::log_1(&format!("Parent key: {}", hex::encode(&self.private_key.secret_bytes())).into());
        web_sys::console::log_1(&format!("IL (tweak): {}", hex::encode(&il_scalar_bytes)).into());
        
        // Perform scalar addition: child_key = parent_key + IL (mod n)
        let child_key = self.private_key.add_tweak(&tweak)
            .map_err(|e| Dip14Error::DerivationFailed(format!("Failed to add tweak: {}", e)))?;
            
        // Debug: log result after addition
        web_sys::console::log_1(&format!("Child key after addition: {}", hex::encode(&child_key.secret_bytes())).into());
        
        // Calculate parent fingerprint (first 4 bytes of parent pubkey hash160)
        let parent_pubkey = PublicKey::from_secret_key(&secp, &self.private_key);
        // Use sha256 then ripemd160 to create hash160
        let sha256_hash = sha256::Hash::hash(&parent_pubkey.serialize());
        let parent_pubkey_hash = dash_sdk::dpp::dashcore::hashes::ripemd160::Hash::hash(&sha256_hash[..]);
        let mut parent_fingerprint = [0u8; 4];
        parent_fingerprint.copy_from_slice(&parent_pubkey_hash[0..4]);
        
        Ok(Self {
            network: self.network,
            depth: self.depth + 1,
            parent_fingerprint,
            child_number: *index,
            chain_code: child_chain_code.try_into()
                .map_err(|_| Dip14Error::DerivationFailed("Invalid chain code length".to_string()))?,
            private_key: child_key,
        })
    }
    
    /// Get the extended public key
    pub fn to_extended_pub_key(&self, secp: &Secp256k1<secp256k1::All>) -> Dip14ExtendedPubKey {
        let public_key = PublicKey::from_secret_key(secp, &self.private_key);
        
        Dip14ExtendedPubKey {
            network: self.network,
            depth: self.depth,
            parent_fingerprint: self.parent_fingerprint,
            child_number: self.child_number,
            chain_code: self.chain_code,
            public_key,
        }
    }
}

/// DIP14 Extended Public Key
pub struct Dip14ExtendedPubKey {
    pub network: Network,
    pub depth: u8,
    pub parent_fingerprint: [u8; 4],
    pub child_number: [u8; 32], // 256-bit child number
    pub chain_code: [u8; 32],
    pub public_key: PublicKey,
}

impl Dip14ExtendedPubKey {
    /// Convert to standard BIP32 ExtendedPubKey (only valid for indices < 2^32-1)
    pub fn to_bip32(&self) -> Result<ExtendedPubKey, Dip14Error> {
        // Check if the child number fits in 32 bits
        if !self.child_number[0..28].iter().all(|&b| b == 0) {
            return Err(Dip14Error::InvalidIndex);
        }
        
        let child_bytes: [u8; 4] = self.child_number[28..32].try_into()
            .map_err(|_| Dip14Error::InvalidIndex)?;
        let child_number = key_wallet::bip32::ChildNumber::from(u32::from_be_bytes(child_bytes));
        
        Ok(ExtendedPubKey {
            network: self.network,
            depth: self.depth,
            parent_fingerprint: key_wallet::bip32::Fingerprint::from_bytes(self.parent_fingerprint),
            child_number,
            chain_code: key_wallet::bip32::ChainCode::from_bytes(self.chain_code),
            public_key: self.public_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ser256() {
        let mut value = [0u8; 32];
        value[31] = 1;
        let serialized = ser256(&value);
        assert_eq!(serialized[31], 1);
        assert_eq!(serialized[0], 0);
    }
    
    #[test]
    fn test_bip32_compatibility() {
        // Test that indices < 2^32-1 produce compatible results
        let seed = [0x42u8; 64];
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();
        let dip14_master = Dip14ExtendedPrivKey::from_bip32(&master);
        
        // Should be able to convert back for small indices
        let bip32_key = dip14_master.to_bip32().unwrap();
        assert_eq!(master.private_key, bip32_key.private_key);
        assert_eq!(master.chain_code, bip32_key.chain_code);
    }
}