//! Subsystem to manage DAPI nodes.

use crate::address_ban_info::AddressBanInfo;
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

/// Flat cooldown applied to rate-limited nodes (gRPC `ResourceExhausted`).
///
/// Unlike the exponential ban used for unhealthy nodes, this is a short,
/// non-escalating window: the node re-enters the live pool automatically
/// after `DEFAULT_RATE_LIMIT_COOLDOWN` with its ban history intact.
///
/// 5 s is long enough to pace the retry burst without starving the pool:
/// with 13 nodes and a 5 s window, a momentarily throttled node misses
/// at most one burst cycle instead of being expelled for 60 s+ and
/// triggering a cascade.
const DEFAULT_RATE_LIMIT_COOLDOWN: Duration = Duration::from_secs(5);

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
    /// Human-readable reason for the most recent ban, if any. Cleared
    /// on [`AddressStatus::unban`]. Sourced from the error that caused
    /// the ban (see `update_address_ban_status`).
    ban_reason: Option<String>,
}

impl AddressStatus {
    /// Ban the [Address] so it won't be available through [AddressList::get_live_address] for some time.
    ///
    /// Back-compat shim for [`AddressStatus::ban_with_reason`] with no reason.
    pub fn ban(&mut self, base_ban_period: &Duration) {
        self.ban_with_reason(base_ban_period, None);
    }

    /// Ban the [Address] and record the `reason` for the ban.
    ///
    /// Applies the same exponential backoff as [`AddressStatus::ban`];
    /// the only difference is that `ban_reason` is stored so callers
    /// (diagnostics, the iOS explorer) can surface why a node was
    /// banned.
    pub fn ban_with_reason(&mut self, base_ban_period: &Duration, reason: Option<String>) {
        let coefficient = (self.ban_count as f64).exp();
        let ban_period = Duration::from_secs_f64(base_ban_period.as_secs_f64() * coefficient);

        self.banned_until = Some(chrono::Utc::now() + ban_period);
        self.ban_count += 1;
        self.ban_reason = reason;
    }

    /// Check if [Address] is banned.
    pub fn is_banned(&self) -> bool {
        self.ban_count > 0
    }

    /// Apply a short, flat cooldown for a transient (client-fixable) error.
    ///
    /// Unlike [`ban_with_reason`], this method does **not** increment `ban_count`,
    /// so the exponential backoff counter is unaffected. The address is removed from
    /// the live pool for `cooldown` and automatically re-enters when the timer expires —
    /// no explicit unban call is needed.
    ///
    /// Used for gRPC conditions that indicate a transient situation rather than node
    /// health problems: `ResourceExhausted` (rate limit), `DeadlineExceeded` (timeout
    /// under load), `Aborted` (MVCC tx conflict), `Cancelled` (client-side cancel).
    ///
    /// Applying the standard 60 s × exp(ban_count) ban for these conditions would route
    /// all traffic to fewer nodes, causing them to also hit their limits and cascading
    /// into `NoAvailableAddressesToRetry`. A short flat cooldown keeps the aggregate
    /// pool capacity intact and breaks the cascade.
    pub fn rate_limit_cooldown(&mut self, cooldown: Duration, reason: Option<String>) {
        self.banned_until = Some(chrono::Utc::now() + cooldown);
        // ban_count is intentionally NOT incremented — transient conditions are not
        // node health problems, and escalating the ban counter would give the node a
        // 60 s × exp(n) ban on its next genuine failure, which is too harsh.
        self.ban_reason = reason;
    }

    /// Clears ban record.
    pub fn unban(&mut self) {
        self.ban_count = 0;
        self.banned_until = None;
        self.ban_reason = None;
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
    ///
    /// Back-compat shim for [`AddressList::ban_with_reason`] with no reason.
    pub fn ban(&self, address: &Address) -> bool {
        self.ban_with_reason(address, None)
    }

    /// Bans address, recording the `reason` for the ban.
    /// Returns false if the address is not in the list.
    pub fn ban_with_reason(&self, address: &Address, reason: Option<String>) -> bool {
        let mut guard = self.addresses.write().unwrap();

        let Some(status) = guard.get_mut(address) else {
            return false;
        };

        status.ban_with_reason(&self.base_ban_period, reason);

        true
    }

    /// Apply a short, flat cooldown to an address for a transient (client-fixable) error.
    ///
    /// Unlike [`ban_with_reason`], this does **not** increment the address' `ban_count`,
    /// so the exponential backoff used for genuinely unhealthy nodes is unaffected.
    /// After [`DEFAULT_RATE_LIMIT_COOLDOWN`] the node automatically re-enters the live
    /// pool — no explicit unban call needed.
    ///
    /// Used for gRPC codes that signal "slow down" or client-side conditions, not node
    /// health failures: `ResourceExhausted`, `DeadlineExceeded`, `Aborted`, `Cancelled`.
    ///
    /// Returns `false` if the address is not in the list.
    pub fn rate_limit_cooldown(&self, address: &Address, reason: Option<String>) -> bool {
        let mut guard = self.addresses.write().unwrap();

        let Some(status) = guard.get_mut(address) else {
            return false;
        };

        status.rate_limit_cooldown(DEFAULT_RATE_LIMIT_COOLDOWN, reason);

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

    /// Get all not banned addresses.
    ///
    /// Returns a vector of addresses that are not currently banned or whose ban period has expired.
    /// The returned addresses use the same filtering logic as [`Self::get_live_address`], checking if the
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

    /// Get an owned snapshot of every address' ban state.
    ///
    /// Clones the current state into an owned `Vec<AddressBanInfo>` so
    /// it can be inspected without holding the internal lock. The
    /// `banned` flag reflects the *currently effectively excluded*
    /// semantics used by [`AddressList::get_live_address`]: the address
    /// has a `banned_until` timestamp in the future. This covers both:
    ///
    /// - **Full bans** (`ban_count > 0`): genuine node-health failures with
    ///   exponential backoff, applied by [`ban_with_reason`].
    /// - **Rate-limit cooldowns** (`ban_count == 0`, but `banned_until` set):
    ///   transient 5-second cooldowns for `ResourceExhausted` responses,
    ///   applied by [`rate_limit_cooldown`]. These nodes are healthy; they
    ///   merely need a brief pause.
    ///
    /// Callers can distinguish the two cases via `ban_count`:
    /// `ban_count == 0 && banned` → rate-limit cooldown, not a health issue.
    pub fn ban_info(&self) -> Vec<AddressBanInfo> {
        let guard = self.addresses.read().unwrap();

        let now = chrono::Utc::now();

        guard
            .iter()
            .map(|(addr, status)| {
                // `banned` = "currently excluded from get_live_address()" —
                // matches the filter: banned_until >= now (i.e. ban not yet expired).
                let banned = status
                    .banned_until
                    .map(|banned_until| banned_until >= now)
                    .unwrap_or(false);
                AddressBanInfo {
                    uri: addr.to_string(),
                    banned,
                    ban_count: status.ban_count,
                    banned_until: status.banned_until,
                    reason: status.ban_reason.clone(),
                }
            })
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
    fn test_address_status_ban_with_reason_stores_reason() {
        let mut status = AddressStatus::default();
        assert!(status.ban_reason.is_none());

        status.ban_with_reason(
            &Duration::from_secs(60),
            Some("transport error".to_string()),
        );
        assert_eq!(status.ban_reason.as_deref(), Some("transport error"));
        assert!(status.is_banned());
    }

    #[test]
    fn test_address_status_ban_without_reason_is_none() {
        let mut status = AddressStatus::default();
        status.ban(&Duration::from_secs(60));
        assert!(status.ban_reason.is_none());
        assert!(status.is_banned());
    }

    #[test]
    fn test_address_status_unban_clears_reason() {
        let mut status = AddressStatus::default();
        status.ban_with_reason(&Duration::from_secs(60), Some("boom".to_string()));
        assert_eq!(status.ban_reason.as_deref(), Some("boom"));

        status.unban();
        assert!(status.ban_reason.is_none());
        assert!(!status.is_banned());
    }

    #[test]
    fn test_address_list_ban_with_reason_records_reason() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        assert!(list.ban_with_reason(&addr, Some("node down".to_string())));

        let info = list.ban_info();
        assert_eq!(info.len(), 1);
        let entry = &info[0];
        assert_eq!(entry.reason.as_deref(), Some("node down"));
        assert!(entry.banned);
        assert_eq!(entry.ban_count, 1);
        assert!(entry.banned_until.is_some());
    }

    #[test]
    fn test_address_list_ban_without_reason_records_none() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        assert!(list.ban(&addr));

        let info = list.ban_info();
        assert_eq!(info.len(), 1);
        assert!(info[0].reason.is_none());
        assert!(info[0].banned);
    }

    #[test]
    fn test_address_list_unban_clears_reason_in_ban_info() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        list.ban_with_reason(&addr, Some("oops".to_string()));
        assert!(list.unban(&addr));

        let info = list.ban_info();
        assert_eq!(info.len(), 1);
        let entry = &info[0];
        assert!(entry.reason.is_none());
        assert!(!entry.banned);
        assert_eq!(entry.ban_count, 0);
        assert!(entry.banned_until.is_none());
    }

    #[test]
    fn test_ban_info_reflects_unbanned_address() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        // Never banned: banned == false, no reason, ban_count == 0.
        let info = list.ban_info();
        assert_eq!(info.len(), 1);
        let entry = &info[0];
        assert!(!entry.banned);
        assert_eq!(entry.ban_count, 0);
        assert!(entry.banned_until.is_none());
        assert!(entry.reason.is_none());
        assert!(entry.uri.contains("127.0.0.1"));
    }

    #[test]
    fn test_ban_info_empty_list() {
        let list = AddressList::new();
        assert!(list.ban_info().is_empty());
    }

    // ── rate_limit_cooldown tests ────────────────────────────────────────────

    /// A cooldown-ed node disappears from `get_live_addresses` while the
    /// timer is active, but ban_count stays 0.
    #[test]
    fn test_rate_limit_cooldown_excludes_node_temporarily() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:4000".parse().unwrap();
        list.add(addr.clone());

        // Initially live.
        assert_eq!(list.get_live_addresses().len(), 1);

        // Apply cooldown directly via AddressStatus.
        {
            let mut guard = list.addresses.write().unwrap();
            let status = guard.get_mut(&addr).unwrap();
            status.rate_limit_cooldown(Duration::from_secs(3600), Some("test".into()));
            // ban_count must NOT be incremented.
            assert_eq!(status.ban_count, 0);
            assert!(status.banned_until.is_some());
        }

        // Node is now excluded from the live pool.
        assert!(list.get_live_addresses().is_empty());
    }

    /// `rate_limit_cooldown` (used for ANY transient error — ResourceExhausted,
    /// DeadlineExceeded, Aborted, Cancelled) does NOT increment ban_count; a
    /// subsequent genuine failure starts the exponential ladder from ban_count = 0.
    #[test]
    fn test_rate_limit_cooldown_does_not_escalate_ban_count() {
        let mut status = AddressStatus::default();
        let base = Duration::from_secs(60);

        // Simulate 10 transient errors (any of the 4 client-fixable codes).
        for _ in 0..10 {
            status.rate_limit_cooldown(Duration::from_secs(5), None);
        }
        assert_eq!(
            status.ban_count, 0,
            "10 transient-error cooldowns must not increment ban_count"
        );

        // First genuine ban starts at exp(0) × 60 s = 60 s.
        status.ban_with_reason(&base, None);
        assert_eq!(status.ban_count, 1);
        let period = status.banned_until.unwrap() - chrono::Utc::now();
        // Allow a couple of seconds of wall-clock slop.
        assert!(
            period.num_seconds() >= 58 && period.num_seconds() <= 62,
            "first genuine ban must be ~60 s regardless of prior rate-limit cooldowns, got {}s",
            period.num_seconds()
        );
    }

    /// `AddressList::rate_limit_cooldown` returns false for unknown addresses.
    #[test]
    fn test_rate_limit_cooldown_unknown_address_returns_false() {
        let list = AddressList::new();
        let addr: Address = "http://127.0.0.1:4001".parse().unwrap();
        assert!(!list.rate_limit_cooldown(&addr, None));
    }

    /// `ban_info` marks a rate-limited node as `banned` (excluded from pool)
    /// even though ban_count is 0, distinguishing it from a fully unbanned node.
    #[test]
    fn test_ban_info_reflects_rate_limited_node() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:4002".parse().unwrap();
        list.add(addr.clone());

        list.rate_limit_cooldown(&addr, Some("rate limited".into()));

        let info = list.ban_info();
        assert_eq!(info.len(), 1);
        let entry = &info[0];
        assert!(
            entry.banned,
            "rate-limited node must appear as banned in ban_info"
        );
        assert_eq!(
            entry.ban_count, 0,
            "ban_count must stay 0 after rate-limit cooldown"
        );
        assert!(entry.banned_until.is_some());
        assert_eq!(entry.reason.as_deref(), Some("rate limited"));
    }
}
