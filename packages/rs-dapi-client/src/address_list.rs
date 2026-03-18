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

// Base ban period in seconds. Ban duration will increase exponentially with each subsequent ban.
pub(crate) const DEFAULT_BASE_BAN_PERIOD: Duration = Duration::from_secs(60);

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

    /// Check if [Address] has a ban record (has been banned at least once and not yet unbanned).
    ///
    /// Note: This checks `ban_count > 0`, not whether the ban is currently active.
    /// An address with an expired `banned_until` but non-zero `ban_count` will still
    /// return `true` here. Use [`AddressList::get_live_address`] for temporal ban checking.
    pub fn is_banned(&self) -> bool {
        self.ban_count > 0
    }

    /// Returns the number of times this address has been banned.
    pub fn ban_count(&self) -> usize {
        self.ban_count
    }

    /// Clears ban record.
    pub fn unban(&mut self) {
        self.ban_count = 0;
        self.banned_until = None;
    }
}

/// [AddressList] errors
#[derive(Debug, thiserror::Error, Clone)]
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

    /// Atomically resets the ban for an address: clears the ban history and applies
    /// a fresh base-period ban. This avoids the TOCTOU gap that would exist with
    /// separate `unban()` + `ban()` calls where a concurrent reader could observe
    /// an unbanned state between the two operations.
    pub fn reset_ban(&self, address: &Address) {
        let mut guard = self.addresses.write().unwrap();
        if let Some(status) = guard.get_mut(address) {
            status.ban_count = 1;
            status.banned_until = Some(chrono::Utc::now() + self.base_ban_period);
        }
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

    /// Get all not banned addresses.
    ///
    /// Returns a vector of addresses that are not currently banned or whose ban period has expired.
    /// The returned addresses use the same filtering logic as [get_live_address], checking if the
    /// ban period has expired based on the current time.
    ///
    /// # Examples
    ///
    /// ```
    /// use rs_dapi_client::{AddressList, Address};
    ///
    /// let mut list = AddressList::new();
    /// list.add("http://127.0.0.1:3000".parse().unwrap());
    /// list.add("http://127.0.0.1:3001".parse().unwrap());
    ///
    /// // Get all non-banned addresses
    /// let live_addresses = list.get_live_addresses();
    /// assert_eq!(live_addresses.len(), 2);
    /// ```
    pub fn get_live_addresses(&self) -> Vec<Address> {
        let guard = self.addresses.read().unwrap();

        let now = chrono::Utc::now();

        guard
            .iter()
            .filter(|(_, status)| {
                status
                    .banned_until
                    .map(|banned_until| banned_until < now)
                    .unwrap_or(true)
            })
            .map(|(addr, _)| addr.clone())
            .collect()
    }

    /// Returns ALL addresses (both banned and unbanned).
    pub fn get_all_addresses(&self) -> Vec<Address> {
        let guard = self.addresses.read().unwrap();
        guard.keys().cloned().collect()
    }

    /// Returns the earliest `banned_until` timestamp that is still in the future.
    /// Returns `None` if no addresses are currently banned with a future expiry.
    pub fn get_next_ban_expiry(&self) -> Option<chrono::DateTime<Utc>> {
        let guard = self.addresses.read().unwrap();
        let now = chrono::Utc::now();
        guard
            .values()
            .filter_map(|status| status.banned_until)
            .filter(|banned_until| *banned_until > now)
            .min()
    }

    /// Returns addresses whose ban has expired but still have a ban record (ban_count > 0).
    /// These are candidates for re-probing before being made available again.
    pub fn get_expired_ban_addresses(&self) -> Vec<Address> {
        let guard = self.addresses.read().unwrap();
        let now = chrono::Utc::now();
        guard
            .iter()
            .filter(|(_, status)| {
                status.ban_count > 0
                    && status
                        .banned_until
                        .map(|banned_until| banned_until <= now)
                        .unwrap_or(true)
            })
            .map(|(addr, _)| addr.clone())
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_live_addresses_empty_list() {
        let list = AddressList::new();
        let live_addresses = list.get_live_addresses();
        assert_eq!(live_addresses.len(), 0);
    }

    #[test]
    fn test_get_live_addresses_all_unbanned() {
        let mut list = AddressList::new();
        list.add("http://127.0.0.1:3000".parse().unwrap());
        list.add("http://127.0.0.1:3001".parse().unwrap());
        list.add("http://127.0.0.1:3002".parse().unwrap());

        let live_addresses = list.get_live_addresses();
        assert_eq!(live_addresses.len(), 3);
    }

    #[test]
    fn test_get_live_addresses_some_banned() {
        let mut list = AddressList::new();
        let addr1: Address = "http://127.0.0.1:3000".parse().unwrap();
        let addr2: Address = "http://127.0.0.1:3001".parse().unwrap();
        let addr3: Address = "http://127.0.0.1:3002".parse().unwrap();

        list.add(addr1.clone());
        list.add(addr2.clone());
        list.add(addr3.clone());

        // Ban addr2
        list.ban(&addr2);

        let live_addresses = list.get_live_addresses();
        assert_eq!(live_addresses.len(), 2);
        assert!(live_addresses.contains(&addr1));
        assert!(live_addresses.contains(&addr3));
        assert!(!live_addresses.contains(&addr2));
    }

    #[test]
    fn test_get_live_addresses_all_banned() {
        let mut list = AddressList::new();
        let addr1: Address = "http://127.0.0.1:3000".parse().unwrap();
        let addr2: Address = "http://127.0.0.1:3001".parse().unwrap();

        list.add(addr1.clone());
        list.add(addr2.clone());

        // Ban all addresses
        list.ban(&addr1);
        list.ban(&addr2);

        let live_addresses = list.get_live_addresses();
        assert_eq!(live_addresses.len(), 0);
    }

    #[test]
    fn test_get_live_addresses_unbanned_after_ban() {
        let mut list = AddressList::new();
        let addr1: Address = "http://127.0.0.1:3000".parse().unwrap();

        list.add(addr1.clone());

        // Ban and then unban
        list.ban(&addr1);
        list.unban(&addr1);

        let live_addresses = list.get_live_addresses();
        assert_eq!(live_addresses.len(), 1);
        assert!(live_addresses.contains(&addr1));
    }

    #[test]
    fn test_address_try_from_uri_without_host() {
        let uri: Uri = Uri::from_str("/path/only").unwrap();
        let result = Address::try_from(uri);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AddressListError::InvalidAddressUri(_)));
    }

    #[test]
    fn test_address_from_str_invalid_uri() {
        // Use a string with invalid URI characters that http::Uri rejects
        let result = Address::from_str("not a valid uri\x00");
        assert!(result.is_err());
    }

    #[test]
    fn test_address_uri_accessor() {
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        let uri = addr.uri();
        assert_eq!(uri.host(), Some("127.0.0.1"));
    }

    #[test]
    fn test_address_partial_eq_with_uri() {
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        let uri = Uri::from_str("http://127.0.0.1:3000").unwrap();
        assert!(addr == uri);

        let other_uri = Uri::from_str("http://127.0.0.1:4000").unwrap();
        assert!(addr != other_uri);
    }

    #[test]
    fn test_address_display() {
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        let display = format!("{}", addr);
        assert!(display.contains("127.0.0.1"));
    }

    #[test]
    fn test_address_status_is_banned() {
        let mut status = AddressStatus::default();
        assert!(!status.is_banned());

        status.ban(&Duration::from_secs(60));
        assert!(status.is_banned());

        status.unban();
        assert!(!status.is_banned());
    }

    #[test]
    fn test_address_status_exponential_ban() {
        let mut status = AddressStatus::default();
        let base_period = Duration::from_secs(1);

        // First ban: coefficient = exp(0) = 1, period = 1s
        status.ban(&base_period);
        assert_eq!(status.ban_count, 1);
        assert!(status.banned_until.is_some());

        // Second ban: coefficient = exp(1) ~= 2.718, period ~= 2.718s
        status.ban(&base_period);
        assert_eq!(status.ban_count, 2);
    }

    #[test]
    fn test_address_list_is_empty() {
        let list = AddressList::new();
        assert!(list.is_empty());

        let mut list = AddressList::new();
        list.add("http://127.0.0.1:3000".parse().unwrap());
        assert!(!list.is_empty());
    }

    #[test]
    fn test_address_list_len() {
        let mut list = AddressList::new();
        assert_eq!(list.len(), 0);

        list.add("http://127.0.0.1:3000".parse().unwrap());
        assert_eq!(list.len(), 1);

        list.add("http://127.0.0.1:3001".parse().unwrap());
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_address_list_add_duplicate() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();

        assert!(list.add(addr.clone()));
        assert!(!list.add(addr)); // duplicate returns false
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_address_list_remove() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();

        list.add(addr.clone());
        assert_eq!(list.len(), 1);

        let removed = list.remove(&addr);
        assert!(removed.is_some());
        assert_eq!(list.len(), 0);

        // Removing non-existent address returns None
        let removed = list.remove(&addr);
        assert!(removed.is_none());
    }

    #[test]
    fn test_address_list_ban_nonexistent() {
        let list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        assert!(!list.ban(&addr));
    }

    #[test]
    fn test_address_list_unban_nonexistent() {
        let list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        assert!(!list.unban(&addr));
    }

    #[test]
    fn test_address_list_is_banned() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        let unknown: Address = "http://127.0.0.1:9999".parse().unwrap();

        list.add(addr.clone());

        assert!(!list.is_banned(&addr));
        assert!(!list.is_banned(&unknown)); // unknown returns false

        list.ban(&addr);
        assert!(list.is_banned(&addr));
    }

    #[test]
    fn test_address_list_from_str() {
        let list: AddressList = "http://127.0.0.1:3000,http://127.0.0.1:3001"
            .parse()
            .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_address_list_from_str_single() {
        let list: AddressList = "http://127.0.0.1:3000".parse().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_address_list_from_str_invalid() {
        let result: Result<AddressList, _> = "not a valid uri\x00".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_address_list_get_live_address_returns_none_when_empty() {
        let list = AddressList::new();
        assert!(list.get_live_address().is_none());
    }

    #[test]
    fn test_address_list_get_live_address_returns_some_when_available() {
        let mut list = AddressList::new();
        list.add("http://127.0.0.1:3000".parse().unwrap());
        assert!(list.get_live_address().is_some());
    }

    #[test]
    fn test_address_list_into_iter() {
        let mut list = AddressList::new();
        list.add("http://127.0.0.1:3000".parse().unwrap());
        list.add("http://127.0.0.1:3001".parse().unwrap());

        let items: Vec<_> = list.into_iter().collect();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_address_list_with_settings() {
        let list = AddressList::with_settings(Duration::from_secs(120));
        assert!(list.is_empty());
    }

    #[test]
    fn test_address_list_default() {
        let list = AddressList::default();
        assert!(list.is_empty());
    }

    #[test]
    fn test_get_all_addresses_returns_both_banned_and_unbanned() {
        let mut list = AddressList::new();
        let addr1: Address = "http://127.0.0.1:3000".parse().unwrap();
        let addr2: Address = "http://127.0.0.1:3001".parse().unwrap();
        list.add(addr1.clone());
        list.add(addr2.clone());
        list.ban(&addr2);

        let all = list.get_all_addresses();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&addr1));
        assert!(all.contains(&addr2));
    }

    #[test]
    fn test_get_next_ban_expiry_returns_earliest() {
        let mut list = AddressList::new();
        let addr1: Address = "http://127.0.0.1:3000".parse().unwrap();
        let addr2: Address = "http://127.0.0.1:3001".parse().unwrap();
        list.add(addr1.clone());
        list.add(addr2.clone());

        list.ban(&addr1);
        list.ban(&addr2);
        list.ban(&addr2);

        let expiry = list.get_next_ban_expiry();
        assert!(expiry.is_some());
    }

    #[test]
    fn test_get_next_ban_expiry_none_when_no_bans() {
        let mut list = AddressList::new();
        list.add("http://127.0.0.1:3000".parse().unwrap());
        assert!(list.get_next_ban_expiry().is_none());
    }

    #[test]
    fn test_get_expired_ban_addresses() {
        let mut list = AddressList::with_settings(Duration::from_millis(1));
        let addr1: Address = "http://127.0.0.1:3000".parse().unwrap();
        let addr2: Address = "http://127.0.0.1:3001".parse().unwrap();
        list.add(addr1.clone());
        list.add(addr2.clone());

        list.ban(&addr1);
        std::thread::sleep(Duration::from_millis(50));

        let expired = list.get_expired_ban_addresses();
        assert!(expired.contains(&addr1));
        assert!(!expired.contains(&addr2));
    }

    #[test]
    fn test_reset_ban_clears_history_and_applies_base_period_ban() {
        let mut list = AddressList::with_settings(Duration::from_secs(60));
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        // Ban twice to build up ban_count
        list.ban(&addr);
        list.ban(&addr);
        assert_eq!(
            list.addresses.read().unwrap().get(&addr).unwrap().ban_count,
            2
        );

        // reset_ban should set ban_count=1 and apply a fresh base-period ban
        list.reset_ban(&addr);
        let guard = list.addresses.read().unwrap();
        let status = guard.get(&addr).unwrap();
        assert_eq!(status.ban_count, 1, "reset_ban must set ban_count to 1");
        assert!(
            status.banned_until.is_some(),
            "reset_ban must set banned_until"
        );
        // The address must be actively banned (not visible via get_live_address)
        drop(guard);
        assert!(
            list.get_live_address().is_none(),
            "reset_ban must keep address banned"
        );
    }

    #[test]
    fn test_reset_ban_on_nonexistent_address_is_noop() {
        let list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        // Must not panic
        list.reset_ban(&addr);
    }

    #[test]
    fn test_address_status_ban_count() {
        let mut status = AddressStatus::default();
        assert_eq!(status.ban_count(), 0);

        status.ban(&Duration::from_secs(60));
        assert_eq!(status.ban_count(), 1);

        status.ban(&Duration::from_secs(60));
        assert_eq!(status.ban_count(), 2);

        status.unban();
        assert_eq!(status.ban_count(), 0);
    }
}
