use std::os::raw::{c_char, c_uchar};

/// Network types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    Mainnet = 0,
    Testnet = 1,
    Devnet = 2,
    Regtest = 3,
}

impl NetworkType {
    pub fn to_dash_network(&self) -> dashcore::Network {
        match self {
            NetworkType::Mainnet => dashcore::Network::Dash,
            NetworkType::Testnet => dashcore::Network::Testnet,
            NetworkType::Devnet => dashcore::Network::Devnet,
            NetworkType::Regtest => dashcore::Network::Regtest,
        }
    }

    pub fn from_dash_network(network: dashcore::Network) -> Self {
        match network {
            dashcore::Network::Dash => NetworkType::Mainnet,
            dashcore::Network::Testnet => NetworkType::Testnet,
            dashcore::Network::Devnet => NetworkType::Devnet,
            dashcore::Network::Regtest => NetworkType::Regtest,
            _ => NetworkType::Mainnet,
        }
    }
}

/// Identifier (32 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IdentifierBytes {
    pub bytes: [c_uchar; 32],
}

impl IdentifierBytes {
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self { bytes })
    }

    pub fn to_identifier(&self) -> Result<dpp::prelude::Identifier, dpp::ProtocolError> {
        dpp::prelude::Identifier::from_bytes(&self.bytes)
            .map_err(|_| dpp::ProtocolError::Generic("Invalid identifier bytes".to_string()))
    }
}

impl From<dpp::prelude::Identifier> for IdentifierBytes {
    fn from(id: dpp::prelude::Identifier) -> Self {
        let bytes: [u8; 32] = id.to_buffer();
        Self { bytes }
    }
}

/// Block time structure
#[repr(C)]
pub struct BlockTime {
    pub height: u64,
    pub core_height: u32,
    pub timestamp: u64,
}

impl From<platform_wallet::BlockTime> for BlockTime {
    fn from(bt: platform_wallet::BlockTime) -> Self {
        Self {
            height: bt.height,
            core_height: bt.core_height,
            timestamp: bt.timestamp,
        }
    }
}

impl From<BlockTime> for platform_wallet::BlockTime {
    fn from(bt: BlockTime) -> Self {
        Self {
            height: bt.height,
            core_height: bt.core_height,
            timestamp: bt.timestamp,
        }
    }
}

/// Contact request structure
#[repr(C)]
pub struct ContactRequest {
    pub identity_id: IdentifierBytes,
    pub label: *mut c_char,
    pub timestamp: u64,
}

/// Established contact structure
#[repr(C)]
pub struct EstablishedContact {
    pub identity_id: IdentifierBytes,
    pub label: *mut c_char,
    pub established_at: u64,
}

/// Array wrapper for returning multiple items
#[repr(C)]
pub struct IdentifierArray {
    pub items: *mut IdentifierBytes,
    pub count: usize,
}

impl IdentifierArray {
    pub fn new(identifiers: Vec<dpp::prelude::Identifier>) -> Self {
        let count = identifiers.len();
        if count == 0 {
            return Self {
                items: std::ptr::null_mut(),
                count: 0,
            };
        }

        let mut items: Vec<IdentifierBytes> = identifiers.into_iter().map(|id| id.into()).collect();

        let ptr = items.as_mut_ptr();
        std::mem::forget(items);

        Self { items: ptr, count }
    }
}

/// Free identifier array
#[no_mangle]
pub extern "C" fn platform_wallet_identifier_array_free(array: IdentifierArray) {
    if !array.items.is_null() && array.count > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(array.items, array.count, array.count);
        }
    }
}

/// Free a C string
#[no_mangle]
pub extern "C" fn platform_wallet_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_type_conversion() {
        assert_eq!(
            NetworkType::Mainnet.to_dash_network(),
            dashcore::Network::Dash
        );
        assert_eq!(
            NetworkType::Testnet.to_dash_network(),
            dashcore::Network::Testnet
        );
        assert_eq!(
            NetworkType::from_dash_network(dashcore::Network::Testnet),
            NetworkType::Testnet
        );
    }

    #[test]
    fn test_identifier_bytes_from_slice() {
        let bytes = [0u8; 32];
        let id_bytes = IdentifierBytes::from_slice(&bytes);
        assert!(id_bytes.is_some());

        let short_bytes = [0u8; 16];
        let id_bytes = IdentifierBytes::from_slice(&short_bytes);
        assert!(id_bytes.is_none());
    }

    #[test]
    fn test_block_time_conversion() {
        let bt = platform_wallet::BlockTime {
            height: 100,
            core_height: 200,
            timestamp: 1234567890,
        };

        let ffi_bt: BlockTime = bt.into();
        assert_eq!(ffi_bt.height, 100);
        assert_eq!(ffi_bt.core_height, 200);
        assert_eq!(ffi_bt.timestamp, 1234567890);

        let back: platform_wallet::BlockTime = ffi_bt.into();
        assert_eq!(back.height, 100);
        assert_eq!(back.core_height, 200);
        assert_eq!(back.timestamp, 1234567890);
    }

    #[test]
    fn test_identifier_array_empty() {
        let array = IdentifierArray::new(vec![]);
        assert!(array.items.is_null());
        assert_eq!(array.count, 0);
    }

    #[test]
    fn test_identifier_array_with_items() {
        let id1 = dpp::prelude::Identifier::random();
        let id2 = dpp::prelude::Identifier::random();

        let array = IdentifierArray::new(vec![id1, id2]);
        assert!(!array.items.is_null());
        assert_eq!(array.count, 2);

        platform_wallet_identifier_array_free(array);
    }
}
