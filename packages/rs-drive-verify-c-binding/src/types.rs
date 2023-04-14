/// Type alias for a public key hash
pub(crate) type PublicKeyHash = [u8; 20];

/// Represents proof verification result + full identity
#[repr(C)]
pub struct IdentityVerificationResult {
    pub is_valid: bool,
    pub root_hash: *const [u8; 32],
    pub has_identity: bool,
    pub identity: *const Identity,
}

impl Default for IdentityVerificationResult {
    fn default() -> Self {
        Self {
            is_valid: false,
            root_hash: std::ptr::null(),
            has_identity: false,
            identity: std::ptr::null(),
        }
    }
}

/// Represent proof verification result + multiple identities
#[repr(C)]
pub struct MultipleIdentityVerificationResult {
    pub is_valid: bool,
    pub root_hash: *const [u8; 32],
    pub public_key_hash_identity_map: *const *const PublicKeyHashIdentityMap,
    pub map_size: usize,
}

impl Default for MultipleIdentityVerificationResult {
    fn default() -> Self {
        Self {
            is_valid: false,
            root_hash: std::ptr::null(),
            public_key_hash_identity_map: std::ptr::null(),
            map_size: 0,
        }
    }
}

/// Maps a public key hash to an identity
#[repr(C)]
pub struct PublicKeyHashIdentityMap {
    pub public_key_hash: *const u8,
    pub public_key_hash_length: usize,
    pub has_identity: bool,
    pub identity: *const Identity,
}

/// Represents proof verification result + identity id result
#[repr(C)]
pub struct IdentityIdVerificationResult {
    pub is_valid: bool,
    pub root_hash: *const [u8; 32],
    pub has_identity_id: bool,
    pub identity_id: *const u8,
    pub id_size: usize,
}

impl Default for IdentityIdVerificationResult {
    fn default() -> Self {
        Self {
            is_valid: false,
            root_hash: std::ptr::null(),
            has_identity_id: false,
            identity_id: std::ptr::null(),
            id_size: 0,
        }
    }
}

/// Represent proof verification result + multiple identity balance result
#[repr(C)]
pub struct MultipleIdentityBalanceVerificationResult {
    pub is_valid: bool,
    pub root_hash: *const [u8; 32],
    pub identity_id_balance_map: *const *const IdentityIdBalanceMap,
    pub map_size: usize,
}

impl Default for MultipleIdentityBalanceVerificationResult {
    fn default() -> Self {
        Self {
            is_valid: true,
            root_hash: std::ptr::null(),
            identity_id_balance_map: std::ptr::null(),
            map_size: 0,
        }
    }
}

/// Maps from an identity id to an optional balance
#[repr(C)]
pub struct IdentityIdBalanceMap {
    pub identity_id: *const u8,
    pub id_size: usize,
    pub has_balance: bool,
    pub balance: u64,
}

/// Represents proof verification result + multiple identity id result
#[repr(C)]
pub struct MultipleIdentityIdVerificationResult {
    pub is_valid: bool,
    pub root_hash: *const [u8; 32],
    pub map_size: usize,
    pub public_key_hash_identity_id_map: *const *const PublicKeyHashIdentityIdMap,
}

impl Default for MultipleIdentityIdVerificationResult {
    fn default() -> Self {
        Self {
            is_valid: true,
            root_hash: std::ptr::null(),
            map_size: 0,
            public_key_hash_identity_id_map: std::ptr::null(),
        }
    }
}

/// Maps a public key hash to an identity id
#[repr(C)]
pub struct PublicKeyHashIdentityIdMap {
    pub public_key_hash: *const u8,
    pub public_key_hash_size: usize,
    pub has_identity_id: bool,
    pub identity_id: *const u8,
    pub id_size: usize,
}

/// Represents an identity
#[repr(C)]
pub struct Identity {
    pub protocol_version: u32,
    pub id: *const [u8; 32],
    pub public_keys_count: usize,
    pub public_keys: *const *const IdPublicKeyMap,
    pub balance: u64,
    pub revision: u64,
    pub has_asset_lock_proof: bool,
    pub asset_lock_proof: *const AssetLockProof,
    pub has_metadata: bool,
    pub meta_data: *const MetaData,
}

/// Maps a key id to a public key
#[repr(C)]
pub struct IdPublicKeyMap {
    pub key: u32,
    pub public_key: *const IdentityPublicKey,
}

/// Represents an identity public key
#[repr(C)]
pub struct IdentityPublicKey {
    pub id: u32,

    // AUTHENTICATION = 0,
    // ENCRYPTION = 1,
    // DECRYPTION = 2,
    // WITHDRAW = 3
    pub purpose: u8,

    // MASTER = 0,
    // CRITICAL = 1,
    // HIGH = 2,
    // MEDIUM = 3
    pub security_level: u8,

    // ECDSA_SECP256K1 = 0,
    // BLS312_381 = 1,
    // ECDSA_HASH160 = 2,
    // BIP13_SCRIPT_HASH = 3
    pub key_type: u8,

    pub read_only: bool,
    pub data_length: usize,
    pub data: *const u8,
    pub has_disabled_at: bool,
    pub disabled_at: u64,
}

/// Represents an asset lock proof
// TODO: add the actual asset lock types
#[repr(C)]
pub struct AssetLockProof {
    pub is_instant: bool,
    // pub instant_asset_lock_proof: *const InstantAssetLocKProof,
    pub is_chain: bool,
    // pub chain_asset_lock_proof: *const ChainAssetLockProof,
}

/// Represents identity metat data
#[repr(C)]
pub struct MetaData {
    pub block_height: u64,
    pub core_chain_locked_height: u64,
    pub time_ms: u64,
    pub protocol_version: u32,
}
