//! The public payment addresses a DashPay profile publishes, between the
//! address strings people read and paste and the bytes the profile stores.
//!
//! The profile stores a Core or Platform address as its storage type byte
//! (0x00 P2PKH, 0x01 P2SH) followed by the 20-byte HASH160, and a shielded
//! address as the raw 43-byte Orchard address. People see a Core address as
//! Base58Check and a Platform or shielded address as bech32m, each with the
//! prefix of its network. A string of another network is refused, so a
//! testnet profile cannot publish a mainnet address by mistake.

use std::str::FromStr;

use dashcore::address::Payload;
use dashcore::{Address, Network, PubkeyHash, ScriptHash};
#[cfg(feature = "shielded")]
use dpp::address_funds::OrchardAddress;
use dpp::address_funds::PlatformAddress;

use super::profile::{valid_transparent_payment_address, validated_shielded_address};
use crate::error::PlatformWalletError;

/// Storage type byte of a pay-to-public-key-hash address.
const STORAGE_P2PKH: u8 = 0x00;
/// Storage type byte of a pay-to-script-hash address.
const STORAGE_P2SH: u8 = 0x01;

/// Which of the three profile payment addresses a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfilePaymentAddressKind {
    /// `corePaymentAddress`, shown as a Base58Check Core address
    Core,
    /// `platformPaymentAddress`, shown as a bech32m Platform address
    Platform,
    /// `shieldedAddress`, shown as a bech32m Orchard address
    Shielded,
}

impl ProfilePaymentAddressKind {
    /// The profile document property that holds this address.
    pub fn property_name(self) -> &'static str {
        match self {
            Self::Core => "corePaymentAddress",
            Self::Platform => "platformPaymentAddress",
            Self::Shielded => "shieldedAddress",
        }
    }
}

/// Which payment addresses a profile write may set on the connected network.
/// A field is missing when the network's DashPay contract has no such
/// property yet (before protocol version 14), and the shielded address is also
/// missing when this build has no shielded support.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProfilePaymentAddressSupport {
    /// `corePaymentAddress` can be set
    pub core: bool,
    /// `platformPaymentAddress` can be set
    pub platform: bool,
    /// `shieldedAddress` can be set
    pub shielded: bool,
}

impl ProfilePaymentAddressSupport {
    /// Whether `kind` can be set.
    pub fn supports(&self, kind: ProfilePaymentAddressKind) -> bool {
        match kind {
            ProfilePaymentAddressKind::Core => self.core,
            ProfilePaymentAddressKind::Platform => self.platform,
            ProfilePaymentAddressKind::Shielded => self.shielded,
        }
    }
}

/// Turn the address `text` into the bytes a profile stores for `kind`.
///
/// `text` must be an address of `network`: Base58Check for a Core address,
/// bech32m for a Platform or shielded address. Surrounding whitespace is
/// ignored. A Core or Platform address must be P2PKH or P2SH, and a shielded
/// address must decode to a valid Orchard address; a shielded address needs a
/// build with shielded support.
pub fn profile_payment_address_from_string(
    kind: ProfilePaymentAddressKind,
    text: &str,
    network: Network,
) -> Result<Vec<u8>, PlatformWalletError> {
    let text = text.trim();
    let invalid = |reason: String| {
        PlatformWalletError::InvalidIdentityData(format!(
            "'{text}' is not a {} address for {network:?}: {reason}",
            kind_label(kind)
        ))
    };
    match kind {
        ProfilePaymentAddressKind::Core => {
            let address = Address::from_str(text)
                .map_err(|e| invalid(e.to_string()))?
                .require_network(network)
                .map_err(|e| invalid(e.to_string()))?;
            match address.payload() {
                Payload::PubkeyHash(hash) => Ok(storage_bytes(STORAGE_P2PKH, hash.as_ref())),
                Payload::ScriptHash(hash) => Ok(storage_bytes(STORAGE_P2SH, hash.as_ref())),
                _ => Err(invalid(
                    "only P2PKH and P2SH addresses can be published".into(),
                )),
            }
        }
        ProfilePaymentAddressKind::Platform => {
            ensure_bech32m_network(text, network).map_err(&invalid)?;
            let address =
                PlatformAddress::from_bech32m_string(text).map_err(|e| invalid(e.to_string()))?;
            Ok(address.to_bytes())
        }
        ProfilePaymentAddressKind::Shielded => shielded_from_string(text, network, &invalid),
    }
}

/// Render the stored `bytes` of a `kind` address as the address string of
/// `network`, or `None` when the bytes are not a valid address of that kind
/// (or, for a shielded address, this build has no shielded support).
pub fn profile_payment_address_to_string(
    kind: ProfilePaymentAddressKind,
    bytes: &[u8],
    network: Network,
) -> Option<String> {
    match kind {
        ProfilePaymentAddressKind::Core => {
            if !valid_transparent_payment_address(bytes) {
                return None;
            }
            let hash: [u8; 20] = bytes[1..].try_into().ok()?;
            let payload = if bytes[0] == STORAGE_P2PKH {
                Payload::PubkeyHash(PubkeyHash::from_byte_array(hash))
            } else {
                Payload::ScriptHash(ScriptHash::from_byte_array(hash))
            };
            Some(Address::new(network, payload).to_string())
        }
        ProfilePaymentAddressKind::Platform => {
            if !valid_transparent_payment_address(bytes) {
                return None;
            }
            PlatformAddress::from_bytes(bytes)
                .ok()
                .map(|address| address.to_bech32m_string(network))
        }
        ProfilePaymentAddressKind::Shielded => shielded_to_string(bytes, network),
    }
}

fn kind_label(kind: ProfilePaymentAddressKind) -> &'static str {
    match kind {
        ProfilePaymentAddressKind::Core => "Core",
        ProfilePaymentAddressKind::Platform => "Platform",
        ProfilePaymentAddressKind::Shielded => "shielded",
    }
}

fn storage_bytes(type_byte: u8, hash: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1 + hash.len());
    bytes.push(type_byte);
    bytes.extend_from_slice(hash);
    bytes
}

/// Refuse a bech32m address whose prefix belongs to another network. The
/// decoders accept both prefixes, so the network is checked here.
fn ensure_bech32m_network(text: &str, network: Network) -> Result<(), String> {
    let mainnet = PlatformAddress::is_mainnet_bech32m(text).map_err(|e| e.to_string())?;
    if mainnet == (network == Network::Mainnet) {
        Ok(())
    } else {
        Err(format!(
            "the address prefix is not '{}'",
            PlatformAddress::hrp_for_network(network)
        ))
    }
}

#[cfg(feature = "shielded")]
fn shielded_from_string(
    text: &str,
    network: Network,
    invalid: &dyn Fn(String) -> PlatformWalletError,
) -> Result<Vec<u8>, PlatformWalletError> {
    ensure_bech32m_network(text, network).map_err(invalid)?;
    let address = OrchardAddress::from_bech32m_string(text).map_err(|e| invalid(e.to_string()))?;
    Ok(address.to_raw_bytes().to_vec())
}

#[cfg(not(feature = "shielded"))]
fn shielded_from_string(
    _text: &str,
    _network: Network,
    _invalid: &dyn Fn(String) -> PlatformWalletError,
) -> Result<Vec<u8>, PlatformWalletError> {
    Err(PlatformWalletError::InvalidIdentityData(
        "Shielded addresses require shielded support".to_string(),
    ))
}

#[cfg(feature = "shielded")]
fn shielded_to_string(bytes: &[u8], network: Network) -> Option<String> {
    let raw = validated_shielded_address(bytes)?;
    OrchardAddress::from_raw_bytes(&raw)
        .ok()
        .map(|address| address.to_bech32m_string(network))
}

#[cfg(not(feature = "shielded"))]
fn shielded_to_string(bytes: &[u8], _network: Network) -> Option<String> {
    // Validated so the two builds agree on which bytes are an address.
    validated_shielded_address(bytes)?;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: [u8; 20] = [0x11; 20];

    fn core_string(payload: Payload, network: Network) -> String {
        Address::new(network, payload).to_string()
    }

    #[test]
    fn should_store_a_core_p2pkh_and_p2sh_address_under_their_storage_type_bytes() {
        let p2pkh = core_string(
            Payload::PubkeyHash(PubkeyHash::from_byte_array(HASH)),
            Network::Testnet,
        );
        let p2sh = core_string(
            Payload::ScriptHash(ScriptHash::from_byte_array(HASH)),
            Network::Testnet,
        );

        let stored = profile_payment_address_from_string(
            ProfilePaymentAddressKind::Core,
            &format!("  {p2pkh}\n"),
            Network::Testnet,
        )
        .expect("a testnet P2PKH address");
        assert_eq!(stored, storage_bytes(STORAGE_P2PKH, &HASH));
        assert_eq!(
            profile_payment_address_to_string(
                ProfilePaymentAddressKind::Core,
                &stored,
                Network::Testnet
            ),
            Some(p2pkh)
        );

        let stored = profile_payment_address_from_string(
            ProfilePaymentAddressKind::Core,
            &p2sh,
            Network::Testnet,
        )
        .expect("a testnet P2SH address");
        assert_eq!(stored, storage_bytes(STORAGE_P2SH, &HASH));
        assert_eq!(
            profile_payment_address_to_string(
                ProfilePaymentAddressKind::Core,
                &stored,
                Network::Testnet
            ),
            Some(p2sh)
        );
    }

    #[test]
    fn should_refuse_a_core_address_of_another_network() {
        let mainnet = core_string(
            Payload::PubkeyHash(PubkeyHash::from_byte_array(HASH)),
            Network::Mainnet,
        );
        assert!(profile_payment_address_from_string(
            ProfilePaymentAddressKind::Core,
            &mainnet,
            Network::Testnet
        )
        .is_err());
    }

    #[test]
    fn should_store_a_platform_address_in_storage_form_not_its_bech32m_type_byte() {
        let address = PlatformAddress::P2pkh(HASH);
        let text = address.to_bech32m_string(Network::Testnet);

        let stored = profile_payment_address_from_string(
            ProfilePaymentAddressKind::Platform,
            &text,
            Network::Testnet,
        )
        .expect("a testnet Platform address");
        assert_eq!(stored, storage_bytes(STORAGE_P2PKH, &HASH));
        assert_eq!(
            profile_payment_address_to_string(
                ProfilePaymentAddressKind::Platform,
                &stored,
                Network::Testnet
            ),
            Some(text)
        );
    }

    #[test]
    fn should_refuse_a_platform_address_of_another_network() {
        let text = PlatformAddress::P2sh(HASH).to_bech32m_string(Network::Mainnet);
        assert!(profile_payment_address_from_string(
            ProfilePaymentAddressKind::Platform,
            &text,
            Network::Testnet
        )
        .is_err());
    }

    #[test]
    fn should_refuse_an_address_pasted_into_the_wrong_kind() {
        let platform = PlatformAddress::P2pkh(HASH).to_bech32m_string(Network::Testnet);
        let core = core_string(
            Payload::PubkeyHash(PubkeyHash::from_byte_array(HASH)),
            Network::Testnet,
        );
        assert!(profile_payment_address_from_string(
            ProfilePaymentAddressKind::Core,
            &platform,
            Network::Testnet
        )
        .is_err());
        assert!(profile_payment_address_from_string(
            ProfilePaymentAddressKind::Platform,
            &core,
            Network::Testnet
        )
        .is_err());
        assert!(profile_payment_address_from_string(
            ProfilePaymentAddressKind::Shielded,
            &platform,
            Network::Testnet
        )
        .is_err());
    }

    #[test]
    fn should_render_nothing_for_bytes_that_are_not_an_address() {
        for kind in [
            ProfilePaymentAddressKind::Core,
            ProfilePaymentAddressKind::Platform,
        ] {
            assert_eq!(
                profile_payment_address_to_string(kind, &[0x02; 21], Network::Testnet),
                None
            );
            assert_eq!(
                profile_payment_address_to_string(kind, &[0x00; 20], Network::Testnet),
                None
            );
        }
        assert_eq!(
            profile_payment_address_to_string(
                ProfilePaymentAddressKind::Shielded,
                &[0x00; 42],
                Network::Testnet
            ),
            None
        );
    }

    #[cfg(feature = "shielded")]
    #[test]
    fn should_round_trip_a_shielded_address_and_refuse_another_network() {
        use orchard::keys::{FullViewingKey, Scope, SpendingKey};

        let spending_key = SpendingKey::from_bytes([7; 32]).expect("a valid spending key");
        let raw = FullViewingKey::from(&spending_key)
            .address_at(0u32, Scope::External)
            .to_raw_address_bytes();
        let text = OrchardAddress::from_raw_bytes(&raw)
            .expect("a valid Orchard address")
            .to_bech32m_string(Network::Testnet);

        let stored = profile_payment_address_from_string(
            ProfilePaymentAddressKind::Shielded,
            &text,
            Network::Testnet,
        )
        .expect("a testnet shielded address");
        assert_eq!(stored, raw.to_vec());
        assert_eq!(
            profile_payment_address_to_string(
                ProfilePaymentAddressKind::Shielded,
                &stored,
                Network::Testnet
            ),
            Some(text.clone())
        );
        assert!(profile_payment_address_from_string(
            ProfilePaymentAddressKind::Shielded,
            &text,
            Network::Mainnet
        )
        .is_err());
    }
}
