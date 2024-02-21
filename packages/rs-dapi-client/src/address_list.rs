//! Subsystem to manage DAPI nodes.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time;
use std::time::Duration;

use http::Uri;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};

const DEFAULT_BASE_BAN_PERIOD: Duration = Duration::from_secs(60);

/// DAPI address.
#[derive(Debug, Clone, Eq)]
pub struct Address {
    ban_count: usize,
    banned_until: Option<time::Instant>,
    uri: Uri,
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

impl From<Uri> for Address {
    fn from(uri: Uri) -> Self {
        Address {
            ban_count: 0,
            banned_until: None,
            uri,
        }
    }
}

impl Address {
    /// Ban the [Address] so it won't be available through [AddressList::get_live_address] for some time.
    fn ban(&mut self, base_ban_period: &Duration) {
        let coefficient = (self.ban_count as f64).exp();
        let ban_period = Duration::from_secs_f64(base_ban_period.as_secs_f64() * coefficient);

        self.banned_until = Some(time::Instant::now() + ban_period);
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
pub enum AddressListError {
    #[error("address {0} not found in the list")]
    AddressNotFound(Uri),
}

/// A structure to manage DAPI addresses to select from
/// for [DapiRequest](crate::DapiRequest) execution.
#[derive(Debug)]
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

    // TODO: this is the most simple way to add an address
    //  however we need to support bulk loading (e.g. providing a network name)
    //  and also fetch updated from SML.
    /// Add a node [Address] to [AddressList] by [Uri].
    /// Returns false if the address is already in the list.
    pub fn add_uri(&mut self, uri: Uri) -> bool {
        self.addresses.insert(uri.into())
    }

    /// Randomly select a not banned address.
    pub fn get_live_address(&self) -> Option<&Address> {
        let now = time::Instant::now();
        let mut rng = SmallRng::from_entropy();

        self.addresses
            .iter()
            .filter(|addr| {
                addr.banned_until
                    .map(|banned_until| banned_until < now)
                    .unwrap_or(true)
            })
            .choose(&mut rng)
    }
}

impl From<&str> for AddressList {
    fn from(value: &str) -> Self {
        let uri_list: Vec<Uri> = value
            .split(',')
            .map(|uri| Uri::from_str(uri).expect("invalid uri"))
            .collect();

        Self::from_iter(uri_list)
    }
}

impl FromIterator<Uri> for AddressList {
    fn from_iter<T: IntoIterator<Item = Uri>>(iter: T) -> Self {
        let mut address_list = Self::new();
        for uri in iter {
            address_list.add_uri(uri);
        }

        address_list
    }
}
