//! Subsystem to manage DAPI nodes.

use chrono::Utc;
use dapi_grpc::tonic::transport::Uri;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::Duration;

const DEFAULT_BASE_BAN_PERIOD: Duration = Duration::from_secs(60);

/// DAPI address.
#[derive(Debug, Clone, Eq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct Address {
    ban_count: usize,
    banned_until: Option<chrono::DateTime<Utc>>,
    #[cfg_attr(feature = "mocks", serde(with = "http_serde::uri"))]
    uri: Uri,
}

impl FromStr for Address {
    type Err = AddressListError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uri::from_str(s)
            .map_err(|e| AddressListError::InvalidAddressUri(e.to_string()))
            .map(Address::try_from)?
    }
}

impl PartialEq<Self> for Address {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri
    }
}

impl PartialEq<Uri> for Address {
    fn eq(&self, other: &Uri) -> bool {
        self.uri == *other
    }
}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uri.hash(state);
    }
}

impl TryFrom<Uri> for Address {
    type Error = AddressListError;

    fn try_from(value: Uri) -> Result<Self, Self::Error> {
        if value.host().is_none() {
            return Err(AddressListError::InvalidAddressUri("uri must contain host".to_string()));
        }

        Ok(Address {
            ban_count: 0,
            banned_until: None,
            uri: value,
        })
    }
}

impl Address {
    /// Ban the [Address] so it won't be available through [AddressList::get_live_address] for some time.
    fn ban(&mut self, base_ban_period: &Duration) {
        let coefficient = (self.ban_count as f64).exp();
        let ban_period = Duration::from_secs_f64(base_ban_period.as_secs_f64() * coefficient);

        self.banned_until = Some(chrono::Utc::now() + ban_period);
        self.ban_count += 1;
    }

    /// Check if [Address] is banned.
    pub fn is_banned(&self) -> bool {
        self.ban_count > 0
    }

    /// Clears ban record.
    fn unban(&mut self) {
        self.ban_count = 0;
        self.banned_until = None;
    }

    /// Get [Uri] of a node.
    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}

/// [AddressList] errors
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum AddressListError {
    /// Specified address is not present in the list
    #[error("address {0} not found in the list")]
    AddressNotFound(#[cfg_attr(feature = "mocks", serde(with = "http_serde::uri"))] Uri),
    /// A valid uri is required to create an Address
    #[error("unable parse address: {0}")]
    #[cfg_attr(feature = "mocks", serde(skip))]
    InvalidAddressUri(String),
}

/// A structure to manage DAPI addresses to select from
/// for [DapiRequest](crate::DapiRequest) execution.
#[derive(Debug, Clone)]
pub struct AddressList {
    addresses: HashSet<Address>,
    base_ban_period: Duration,
}

impl Default for AddressList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.uri.fmt(f)
    }
}

impl AddressList {
    /// Creates an empty [AddressList] with default base ban time.
    pub fn new() -> Self {
        AddressList::with_settings(DEFAULT_BASE_BAN_PERIOD)
    }

    /// Creates an empty [AddressList] with adjustable base ban time.
    pub fn with_settings(base_ban_period: Duration) -> Self {
        AddressList {
            addresses: HashSet::new(),
            base_ban_period,
        }
    }

    /// Bans address
    pub(crate) fn ban_address(&mut self, address: &Address) -> Result<(), AddressListError> {
        if !self.addresses.remove(address) {
            return Err(AddressListError::AddressNotFound(address.uri.clone()));
        };

        let mut banned_address = address.clone();
        banned_address.ban(&self.base_ban_period);

        self.addresses.insert(banned_address);

        Ok(())
    }

    /// Clears address' ban record
    pub(crate) fn unban_address(&mut self, address: &Address) -> Result<(), AddressListError> {
        if !self.addresses.remove(address) {
            return Err(AddressListError::AddressNotFound(address.uri.clone()));
        };

        let mut unbanned_address = address.clone();
        unbanned_address.unban();

        self.addresses.insert(unbanned_address);

        Ok(())
    }

    /// Adds a node [Address] to [AddressList]
    /// Returns false if the address is already in the list.
    pub fn add(&mut self, address: Address) -> bool {
        self.addresses.insert(address)
    }

    /// Randomly select a not banned address.
    pub fn get_live_address(&self) -> Option<&Address> {
        let mut rng = SmallRng::from_entropy();

        self.unbanned().into_iter().choose(&mut rng)
    }

    /// Get all addresses that are not banned.
    fn unbanned(&self) -> Vec<&Address> {
        let now = Utc::now();

        self.addresses
            .iter()
            .filter(|addr| {
                addr.banned_until
                    .map(|banned_until| banned_until < now)
                    .unwrap_or(true)
            })
            .collect()
    }

    /// Get number of available, not banned addresses.
    pub fn available(&self) -> usize {
        self.unbanned().len()
    }

    /// Get number of all addresses, both banned and not banned.
    pub fn len(&self) -> usize {
        self.addresses.len()
    }

    /// Check if the list is empty.
    /// Returns true if there are no addresses in the list.
    /// Returns false if there is at least one address in the list.
    /// Banned addresses are also counted.
    pub fn is_empty(&self) -> bool {
        self.addresses.is_empty()
    }
}

impl FromStr for AddressList {
    type Err = AddressListError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri_list: Vec<Address> = s
            .split(',')
            .map(Address::from_str)
            .collect::<Result<_, _>>()?;

        Ok(Self::from_iter(uri_list))
    }
}

impl FromIterator<Address> for AddressList {
    fn from_iter<T: IntoIterator<Item = Address>>(iter: T) -> Self {
        let mut address_list = Self::new();
        for uri in iter {
            address_list.add(uri);
        }

        address_list
    }
}

impl IntoIterator for AddressList {
    type Item = Address;
    type IntoIter = std::collections::hash_set::IntoIter<Address>;

    fn into_iter(self) -> Self::IntoIter {
        self.addresses.into_iter()
    }
}
