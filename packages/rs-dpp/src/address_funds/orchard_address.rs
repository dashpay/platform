use bech32::{Bech32m, Hrp};
use dashcore::Network;

use crate::address_funds::platform_address::{PLATFORM_HRP_MAINNET, PLATFORM_HRP_TESTNET};
use crate::address_funds::PlatformAddress;
use crate::ProtocolError;

/// Size of the Orchard diversifier (11 bytes).
pub const ORCHARD_DIVERSIFIER_SIZE: usize = 11;
/// Size of the Orchard diversified transmission key pk_d (32 bytes, Pallas curve point).
pub const ORCHARD_PKD_SIZE: usize = 32;
/// Total size of a raw Orchard payment address (43 bytes = diversifier + pk_d).
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
/// Wraps `grovedb_commitment_tree::PaymentAddress`. Use [`From<PaymentAddress>`]
/// to convert from the orchard crate's native type, or [`inner()`](OrchardAddress::inner)
/// / [`into_inner()`](OrchardAddress::into_inner) to access the wrapped address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrchardAddress(grovedb_commitment_tree::PaymentAddress);

impl OrchardAddress {
    /// Type byte for Orchard addresses in bech32m encoding (user-facing).
    /// Produces 'z' as the first bech32 character.
    pub const ORCHARD_TYPE: u8 = 0x10;

    /// Returns the inner [`PaymentAddress`](grovedb_commitment_tree::PaymentAddress).
    pub fn inner(&self) -> &grovedb_commitment_tree::PaymentAddress {
        &self.0
    }

    /// Consumes the wrapper and returns the inner `PaymentAddress`.
    pub fn into_inner(self) -> grovedb_commitment_tree::PaymentAddress {
        self.0
    }

    /// Creates an OrchardAddress from a 43-byte raw address.
    ///
    /// The first 11 bytes are the diversifier, the next 32 are pk_d.
    /// Returns an error if `pk_d` is not a valid Pallas curve point.
    pub fn from_raw_bytes(bytes: &[u8; ORCHARD_ADDRESS_SIZE]) -> Result<Self, ProtocolError> {
        let addr =
            Option::from(grovedb_commitment_tree::PaymentAddress::from_raw_address_bytes(bytes))
                .ok_or_else(|| {
                    ProtocolError::DecodingError(
                        "OrchardAddress pk_d is not a valid Pallas curve point".to_string(),
                    )
                })?;
        Ok(Self(addr))
    }

    /// Returns the raw 43-byte address (diversifier || pk_d).
    pub fn to_raw_bytes(&self) -> [u8; ORCHARD_ADDRESS_SIZE] {
        self.0.to_raw_address_bytes()
    }

    /// Encodes the OrchardAddress as a bech32m string for the specified network.
    ///
    /// Format: `<HRP>1<data-part>`
    /// - Data: type_byte (0x10) || diversifier (11 bytes) || pk_d (32 bytes)
    /// - Total payload: 44 bytes
    /// - Checksum: bech32m (BIP-350)
    pub fn to_bech32m_string(&self, network: Network) -> String {
        let hrp_str = PlatformAddress::hrp_for_network(network);
        let hrp = Hrp::parse(hrp_str).expect("HRP is valid");

        let raw = self.to_raw_bytes();
        let mut payload = Vec::with_capacity(1 + ORCHARD_ADDRESS_SIZE);
        payload.push(Self::ORCHARD_TYPE);
        payload.extend_from_slice(&raw);

        bech32::encode::<Bech32m>(hrp, &payload).expect("encoding should succeed")
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

        let mut raw = [0u8; ORCHARD_ADDRESS_SIZE];
        raw.copy_from_slice(&data[1..]);
        Self::from_raw_bytes(&raw).map(|addr| (addr, network))
    }
}

/// Infallible conversion from the orchard crate's `PaymentAddress` to `OrchardAddress`.
impl From<grovedb_commitment_tree::PaymentAddress> for OrchardAddress {
    fn from(addr: grovedb_commitment_tree::PaymentAddress) -> Self {
        Self(addr)
    }
}

/// Infallible conversion from a reference to `PaymentAddress`.
impl From<&grovedb_commitment_tree::PaymentAddress> for OrchardAddress {
    fn from(addr: &grovedb_commitment_tree::PaymentAddress) -> Self {
        Self(*addr)
    }
}

impl std::fmt::Display for OrchardAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = self.to_raw_bytes();
        write!(
            f,
            "Orchard(d={}, pk_d={})",
            hex::encode(&raw[..ORCHARD_DIVERSIFIER_SIZE]),
            hex::encode(&raw[ORCHARD_DIVERSIFIER_SIZE..])
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bech32::Hrp;

    fn test_orchard_address() -> OrchardAddress {
        use grovedb_commitment_tree::{FullViewingKey, Scope, SpendingKey};
        let sk = SpendingKey::from_bytes([42u8; 32]).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let payment_address = fvk.address_at(0u32, Scope::External);
        OrchardAddress::from(payment_address)
    }

    #[test]
    fn test_orchard_address_raw_bytes_roundtrip() {
        let address = test_orchard_address();
        let raw = address.to_raw_bytes();
        assert_eq!(raw.len(), 43);

        let recovered = OrchardAddress::from_raw_bytes(&raw).unwrap();
        assert_eq!(recovered, address);
    }

    #[test]
    fn test_orchard_bech32m_mainnet_roundtrip() {
        let address = test_orchard_address();

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

    #[test]
    fn test_orchard_bech32m_testnet_roundtrip() {
        let address = test_orchard_address();

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

    #[test]
    fn test_orchard_and_platform_addresses_are_distinguishable() {
        let p2pkh = PlatformAddress::P2pkh([0xAB; 20]);
        let p2sh = PlatformAddress::P2sh([0xAB; 20]);
        let orchard = test_orchard_address();

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

    #[test]
    fn test_orchard_address_from_raw_bytes_invalid_pk_d() {
        // All zeros for pk_d is not a valid Pallas curve point
        let mut raw = [0u8; 43];
        raw[0] = 0x01; // non-zero diversifier
        assert!(OrchardAddress::from_raw_bytes(&raw).is_err());
    }

    #[test]
    fn test_orchard_address_display() {
        let address = test_orchard_address();
        let display = format!("{}", address);
        assert!(display.starts_with("Orchard(d="));
        assert!(display.contains("pk_d="));
    }
}
