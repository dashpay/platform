//! Subsystem to manage DAPI nodes.

use crate::Uri;
use chrono::Utc;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

const DEFAULT_BASE_BAN_PERIOD: Duration = Duration::from_secs(60);

/// DAPI address.
#[derive(Debug, Clone, Eq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct Address(#[cfg_attr(feature = "mocks", serde(with = "http_serde::uri"))] Uri);

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
        self.0 == other.0
    }
}

impl PartialEq<Uri> for Address {
    fn eq(&self, other: &Uri) -> bool {
        self.0 == *other
    }
}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl TryFrom<Uri> for Address {
    type Error = AddressListError;

    fn try_from(value: Uri) -> Result<Self, Self::Error> {
        if value.host().is_none() {
            return Err(AddressListError::InvalidAddressUri(
                "uri must contain host".to_string(),
            ));
        }

        Ok(Address(value))
    }
}

impl Address {
    /// Get [Uri] of a node.
    pub fn uri(&self) -> &Uri {
        &self.0
    }
}

/// Address status
/// Contains information about the number of bans and the time until the next ban is lifted.
#[derive(Debug, Default, Clone)]
pub struct AddressStatus {
    ban_count: usize,
    banned_until: Option<chrono::DateTime<Utc>>,
}

impl AddressStatus {
    /// Ban the [Address] so it won't be available through [AddressList::get_live_address] for some time.
    pub fn ban(&mut self, base_ban_period: &Duration) {
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
    pub fn unban(&mut self) {
        self.ban_count = 0;
        self.banned_until = None;
    }
}

/// [AddressList] errors
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum AddressListError {
    /// A valid uri is required to create an Address
    #[error("unable parse address: {0}")]
    #[cfg_attr(feature = "mocks", serde(skip))]
    InvalidAddressUri(String),
}

/// A structure to manage DAPI addresses to select from
/// for [DapiRequest](crate::DapiRequest) execution.
#[derive(Debug, Clone)]
pub struct AddressList {
    addresses: Arc<RwLock<HashMap<Address, AddressStatus>>>,
    base_ban_period: Duration,
}

impl Default for AddressList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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
            addresses: Arc::new(RwLock::new(HashMap::new())),
            base_ban_period,
        }
    }

    /// Bans address
    /// Returns false if the address is not in the list.
    pub fn ban(&self, address: &Address) -> bool {
        let mut guard = self.addresses.write().unwrap();

        let Some(status) = guard.get_mut(address) else {
            return false;
        };

        status.ban(&self.base_ban_period);

        true
    }

    /// Clears address' ban record
    /// Returns false if the address is not in the list.
    pub fn unban(&self, address: &Address) -> bool {
        let mut guard = self.addresses.write().unwrap();

        let Some(status) = guard.get_mut(address) else {
            return false;
        };

        status.unban();

        true
    }

    /// Check if the address is banned.
    pub fn is_banned(&self, address: &Address) -> bool {
        let guard = self.addresses.read().unwrap();

        guard
            .get(address)
            .map(|status| status.is_banned())
            .unwrap_or(false)
    }

    /// Adds a node [Address] to [AddressList]
    /// Returns false if the address is already in the list.
    pub fn add(&mut self, address: Address) -> bool {
        let mut guard = self.addresses.write().unwrap();

        match guard.entry(address) {
            Entry::Occupied(_) => false,
            Entry::Vacant(e) => {
                e.insert(AddressStatus::default());

                true
            }
        }
    }

    /// Remove address from the list
    /// Returns [AddressStatus] if the address was in the list.
    pub fn remove(&mut self, address: &Address) -> Option<AddressStatus> {
        let mut guard = self.addresses.write().unwrap();

        guard.remove(address)
    }

    #[deprecated]
    // TODO: Remove in favor of add
    /// Add a node [Address] to [AddressList] by [Uri].
    /// Returns false if the address is already in the list.
    pub fn add_uri(&mut self, uri: Uri) -> bool {
        self.add(Address::try_from(uri).expect("valid uri"))
    }

    /// Randomly select a not banned address.
    pub fn get_live_address(&self) -> Option<Address> {
        let guard = self.addresses.read().unwrap();

        let mut rng = SmallRng::from_entropy();

        let now = chrono::Utc::now();

        guard
            .iter()
            .filter(|(_, status)| {
                status
                    .banned_until
                    .map(|banned_until| banned_until < now)
                    .unwrap_or(true)
            })
            .choose(&mut rng)
            .map(|(addr, _)| addr.clone())
    }

    /// Get number of all addresses, both banned and not banned.
    pub fn len(&self) -> usize {
        self.addresses.read().unwrap().len()
    }

    /// Check if the list is empty.
    /// Returns true if there are no addresses in the list.
    /// Returns false if there is at least one address in the list.
    /// Banned addresses are also counted.
    pub fn is_empty(&self) -> bool {
        self.addresses.read().unwrap().is_empty()
    }
}

impl IntoIterator for AddressList {
    type Item = (Address, AddressStatus);
    type IntoIter = std::collections::hash_map::IntoIter<Address, AddressStatus>;

    fn into_iter(self) -> Self::IntoIter {
        let mut guard = self.addresses.write().unwrap();

        let addresses_map = mem::take(&mut *guard);

        addresses_map.into_iter()
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
