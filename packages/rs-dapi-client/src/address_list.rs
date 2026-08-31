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

/// Default number of addresses that receive traffic at a time.
///
/// Kept small so requests reuse warm connections instead of sampling the whole
/// list (hundreds of nodes on mainnet), where nearly every request would land
/// on a cold host and pay a fresh TCP + TLS handshake.
const DEFAULT_ACTIVE_SET_SIZE: usize = 5;

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
    /// Applies exponential backoff: the ban window is `base × e^ban_count`
    /// (where `ban_count` is the value *before* this call), and `banned_until`
    /// is always re-based to `now + window` unconditionally, regardless of any
    /// existing active ban.  Concretely, a health failure on a node that already
    /// holds a longer rate-limit window (set via [`AddressStatus::ban_for`]) will
    /// re-base `banned_until` to the exponential value, which may be shorter.
    /// This is intentional: the exponential health-ban ladder owns the window for
    /// genuinely-unhealthy nodes; the no-shorten guarantee is deliberately scoped
    /// to `ban_for → ban_for` sequences only.
    ///
    /// `ban_count` is incremented and `ban_reason` is updated unconditionally.
    /// The counter resets to 0 on [`AddressStatus::unban`].
    pub fn ban_with_reason(&mut self, base_ban_period: &Duration, reason: Option<String>) {
        let coefficient = (self.ban_count as f64).exp();
        let ban_period = Duration::from_secs_f64(base_ban_period.as_secs_f64() * coefficient);

        self.banned_until = Some(chrono::Utc::now() + ban_period);
        self.ban_count += 1;
        self.ban_reason = reason;
    }

    /// Ban the address for an exact `period` (server-advertised), bypassing the
    /// exponential ladder used by [`AddressStatus::ban_with_reason`].
    ///
    /// The ban window is flat (not exponential).  `banned_until` is advanced to
    /// `now + period` only when that timestamp is **later** than the current
    /// `banned_until`, so a short-reset call never shortens a longer active ban
    /// (health ban or a prior longer rate-limit ban).  `ban_reason` is updated
    /// only when the window is extended.  `ban_count` is raised to
    /// `max(ban_count, 1)` unconditionally so that `is_banned()` and
    /// `ban_info()` correctly report the node as banned.  Side-effect: a
    /// previously-clean node (ban_count 0) enters the ladder at floor 1,
    /// meaning its *next* genuine health failure via
    /// [`AddressStatus::ban_with_reason`] uses `60 s × e¹ ≈ 163 s` rather
    /// than the first-rung `60 s × e⁰ = 60 s`.  The counter resets to 0 on
    /// [`AddressStatus::unban`].
    ///
    /// Note: the no-shorten guard applies only to `ban_for → ban_for` call
    /// sequences.  [`AddressStatus::ban_with_reason`] re-bases `banned_until`
    /// unconditionally — see its docs for the intentional cross-method semantics.
    pub fn ban_for(&mut self, period: Duration, reason: Option<String>) {
        let advertised_until = chrono::Utc::now() + period;
        if self
            .banned_until
            .map(|current| current < advertised_until)
            .unwrap_or(true)
        {
            self.banned_until = Some(advertised_until);
            self.ban_reason = reason;
        }
        self.ban_count = self.ban_count.max(1);
    }

    /// Check if [Address] is banned.
    pub fn is_banned(&self) -> bool {
        self.ban_count > 0
    }

    /// Check if [Address] is live at `now`: never banned, or its ban period has
    /// already expired.
    fn is_live(&self, now: chrono::DateTime<Utc>) -> bool {
        self.banned_until
            .map(|banned_until| banned_until < now)
            .unwrap_or(true)
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

/// Sticky rotation state: the addresses currently receiving traffic, and the
/// most recently served one that round-robin selection advances from.
#[derive(Debug, Default)]
struct Rotation {
    active: Vec<Address>,
    last_served: Option<Address>,
}

/// A structure to manage DAPI addresses to select from
/// for [DapiRequest](crate::DapiRequest) execution.
///
/// Address selection is sticky: requests rotate over a small active set of
/// addresses and the rest of the list serves as failover standby (see
/// [AddressList::get_live_address]).
#[derive(Debug, Clone)]
pub struct AddressList {
    addresses: Arc<RwLock<HashMap<Address, AddressStatus>>>,
    rotation: Arc<RwLock<Rotation>>,
    base_ban_period: Duration,
    active_set_size: usize,
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
            rotation: Arc::new(RwLock::new(Rotation::default())),
            base_ban_period,
            active_set_size: DEFAULT_ACTIVE_SET_SIZE,
        }
    }

    /// Set how many addresses receive traffic at a time (minimum 1).
    ///
    /// Smaller values maximize connection reuse, larger values spread load over
    /// more nodes.
    pub fn with_active_set_size(mut self, size: usize) -> Self {
        self.active_set_size = size.max(1);
        self
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

    /// Ban the address for an exact `period` (server-advertised); delegates to
    /// [`AddressStatus::ban_for`] — see that method for the full contract
    /// including the `ban_count` floor and ladder side-effect.
    ///
    /// Returns `false` if the address is not in the list.
    pub fn ban_for(&self, address: &Address, period: Duration, reason: Option<String>) -> bool {
        let mut guard = self.addresses.write().unwrap();

        let Some(status) = guard.get_mut(address) else {
            return false;
        };

        status.ban_for(period, reason);

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

    /// Select a not-banned address to send the next request to.
    ///
    /// Selection is sticky: requests rotate round-robin over a small active
    /// set of addresses (see [AddressList::with_active_set_size]) instead of
    /// sampling the whole list, so connections to those hosts stay warm. An
    /// active address that got banned or removed is dropped from the set here
    /// and a random live standby address is promoted in its place, so the ban
    /// ladder remains the only health signal.
    ///
    /// An address is considered live when it has never been banned or when its
    /// ban period has already expired.
    pub fn get_live_address(&self) -> Option<Address> {
        // TODO(low): module-wide `.read()/.write().unwrap()` panics on a
        // poisoned lock; adopt poison-tolerant locking consistently (SEC-003).
        let guard = self.addresses.read().unwrap();

        let now = chrono::Utc::now();

        // Lock ordering: `addresses` before `rotation`; this is the only place
        // both locks are held at once.
        let mut rotation = self.rotation.write().unwrap();

        // Drop active addresses that are banned or no longer in the list.
        rotation.active.retain(|address| {
            guard
                .get(address)
                .map(|status| status.is_live(now))
                .unwrap_or(false)
        });

        // Refill vacancies with random live standby addresses.
        let vacancies = self.active_set_size.saturating_sub(rotation.active.len());
        if vacancies > 0 {
            let promoted = guard
                .iter()
                .filter(|&(address, status)| {
                    status.is_live(now) && !rotation.active.contains(address)
                })
                .choose_multiple(&mut SmallRng::from_entropy(), vacancies);

            rotation
                .active
                .extend(promoted.into_iter().map(|(address, _)| address.clone()));
        }

        if rotation.active.is_empty() {
            return None;
        }

        // Advance relative to the last-served address rather than a bare index:
        // eviction shifts indices, and an index-based cursor could serve the
        // same address twice in a row after churn. Start from the head when the
        // last-served address is gone (or nothing has been served yet).
        let last_position = rotation
            .last_served
            .as_ref()
            .and_then(|last| rotation.active.iter().position(|address| address == last));

        let index = last_position.map_or(0, |position| (position + 1) % rotation.active.len());
        let address = rotation.active[index].clone();
        rotation.last_served = Some(address.clone());
        Some(address)
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
            .filter(|(_, status)| status.is_live(now))
            .map(|(addr, _)| addr.clone())
            .collect()
    }

    /// Get an owned snapshot of every address' ban state.
    ///
    /// Clones the current state into an owned `Vec<AddressBanInfo>` so
    /// it can be inspected without holding the internal lock. The
    /// `banned` flag reflects the *currently effectively banned*
    /// semantics used by [`AddressList::get_live_address`]: the address
    /// has been banned at least once (`ban_count > 0`) and its ban
    /// period has not yet expired (`banned_until` is in the future).
    pub fn ban_info(&self) -> Vec<AddressBanInfo> {
        let guard = self.addresses.read().unwrap();

        let now = chrono::Utc::now();

        guard
            .iter()
            .map(|(addr, status)| {
                let banned = status.ban_count > 0 && !status.is_live(now);
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
    fn test_get_live_address_sticks_to_small_active_set() {
        let mut list = AddressList::new();
        for i in 0..50 {
            list.add(format!("http://127.0.0.1:{}", 3000 + i).parse().unwrap());
        }

        let distinct: std::collections::HashSet<String> = (0..200)
            .map(|_| list.get_live_address().unwrap().to_string())
            .collect();

        assert_eq!(
            distinct.len(),
            DEFAULT_ACTIVE_SET_SIZE,
            "all traffic must rotate over exactly the active set"
        );
    }

    #[test]
    fn test_get_live_address_round_robins_over_active_set() {
        let mut list = AddressList::new().with_active_set_size(2);
        for i in 0..5 {
            list.add(format!("http://127.0.0.1:{}", 3000 + i).parse().unwrap());
        }

        let picks: Vec<String> = (0..6)
            .map(|_| list.get_live_address().unwrap().to_string())
            .collect();

        // Strict alternation between the two active members.
        assert_ne!(picks[0], picks[1]);
        assert_eq!(picks[0], picks[2]);
        assert_eq!(picks[1], picks[3]);
        assert_eq!(picks[0], picks[4]);
        assert_eq!(picks[1], picks[5]);
    }

    #[test]
    fn test_get_live_address_ban_evicts_active_and_promotes_standby() {
        let mut list = AddressList::new().with_active_set_size(1);
        for i in 0..3 {
            list.add(format!("http://127.0.0.1:{}", 3000 + i).parse().unwrap());
        }

        let first = list.get_live_address().unwrap();
        for _ in 0..5 {
            assert_eq!(
                list.get_live_address().unwrap(),
                first,
                "selection must be sticky until the active address fails"
            );
        }

        list.ban(&first);

        let second = list.get_live_address().unwrap();
        assert_ne!(second, first, "banned address must leave the active set");
        for _ in 0..5 {
            assert_eq!(
                list.get_live_address().unwrap(),
                second,
                "selection must stick to the promoted standby"
            );
        }
    }

    #[test]
    fn test_get_live_address_no_immediate_repeat_after_other_member_evicted() {
        // Regression: with an index-based cursor, evicting an active member
        // other than the one just served shifted indices and could serve the
        // same address twice in a row.
        let mut list = AddressList::new().with_active_set_size(2);
        for i in 0..3 {
            list.add(format!("http://127.0.0.1:{}", 3000 + i).parse().unwrap());
        }

        let first = list.get_live_address().unwrap();
        let second = list.get_live_address().unwrap();
        let third = list.get_live_address().unwrap();
        assert_eq!(first, third, "two-member set alternates");

        // Ban the member that was NOT just served; a standby gets promoted.
        list.ban(&second);

        let next = list.get_live_address().unwrap();
        assert_ne!(
            next, third,
            "must not serve the same address twice in a row after eviction"
        );
        assert_ne!(next, second, "banned address must not be served");
    }

    #[test]
    fn test_get_live_address_removed_address_pruned_from_active_set() {
        let mut list = AddressList::new().with_active_set_size(1);
        list.add("http://127.0.0.1:3000".parse().unwrap());
        list.add("http://127.0.0.1:3001".parse().unwrap());

        let first = list.get_live_address().unwrap();
        list.remove(&first);

        let second = list.get_live_address().unwrap();
        assert_ne!(second, first);
    }

    #[test]
    fn test_get_live_address_with_fewer_live_addresses_than_active_set() {
        let mut list = AddressList::new(); // default active set size 5
        list.add("http://127.0.0.1:3000".parse().unwrap());
        list.add("http://127.0.0.1:3001".parse().unwrap());

        let distinct: std::collections::HashSet<String> = (0..10)
            .map(|_| list.get_live_address().unwrap().to_string())
            .collect();
        assert_eq!(distinct.len(), 2, "both live addresses rotate");
    }

    #[test]
    fn test_get_live_address_all_banned_returns_none() {
        let mut list = AddressList::new().with_active_set_size(1);
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        assert!(list.get_live_address().is_some());
        list.ban(&addr);
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

    #[test]
    fn test_address_status_ban_for_sets_exact_window_and_min_ban_count() {
        let mut status = AddressStatus::default();
        assert_eq!(status.ban_count, 0);
        assert!(status.banned_until.is_none());

        let before = chrono::Utc::now();
        status.ban_for(Duration::from_secs(45), Some("rate limited".into()));
        let after = chrono::Utc::now();

        // ban_count must be at least 1 so is_banned() / ban_info().banned are consistent.
        assert_eq!(status.ban_count, 1, "ban_for sets ban_count to max(0,1)=1");

        // banned_until should be roughly now + 45 s.
        let until = status.banned_until.expect("banned_until must be set");
        let lower = (until - before).num_milliseconds() as f64 / 1000.0;
        let upper = (until - after).num_milliseconds() as f64 / 1000.0;
        assert!(
            lower >= 44.9,
            "banned_until lower bound too short: {lower}s"
        );
        assert!(upper <= 45.1, "banned_until upper bound too long: {upper}s");
        assert_eq!(status.ban_reason.as_deref(), Some("rate limited"));
    }

    /// `ban_for` on a fresh node (ban_count = 0) raises ban_count to 1 (the
    /// ladder floor).  That means the *next* genuine health ban will escalate
    /// from position 1 (~163 s) instead of position 0 (~60 s).  This pins the
    /// documented side-effect so regressions are caught.
    #[test]
    fn test_ban_for_raises_fresh_node_to_ladder_floor() {
        let mut status = AddressStatus::default();
        assert_eq!(status.ban_count, 0, "starts clean");

        // Rate-limit ban on a never-before-banned node.
        status.ban_for(Duration::from_secs(10), Some("rl".into()));
        assert_eq!(
            status.ban_count, 1,
            "ban_for must raise ban_count 0 → 1 (ladder floor)"
        );

        // Subsequent genuine health failure must escalate from the floor (1),
        // yielding ~60 s × e^1 ≈ 163 s, NOT the first-rung ~60 s × e^0 = 60 s.
        let base = Duration::from_secs(60);
        let before = chrono::Utc::now();
        status.ban_with_reason(&base, None); // ban_count 1 → 2; window = 60s × e^1
        let after = chrono::Utc::now();
        assert_eq!(status.ban_count, 2);

        let until = status.banned_until.expect("banned_until set");
        let lo = (until - before).num_milliseconds() as f64 / 1000.0;
        let hi = (until - after).num_milliseconds() as f64 / 1000.0;
        let expected = 60.0_f64 * std::f64::consts::E; // ≈ 163 s
        assert!(
            lo >= expected - 0.5,
            "window lower {lo:.1}s < expected {expected:.1}s (should escalate from floor 1)"
        );
        assert!(
            hi <= expected + 0.5,
            "window upper {hi:.1}s > expected {expected:.1}s"
        );
    }

    #[test]
    fn test_address_status_ban_for_does_not_inflate_existing_ban_count() {
        // A node already health-banned (ban_count = 3) gets rate-limited.
        // ban_count must stay at 3, not grow to 4.
        let mut status = AddressStatus::default();
        let base = Duration::from_secs(60);
        status.ban_with_reason(&base, None); // → 1
        status.ban_with_reason(&base, None); // → 2
        status.ban_with_reason(&base, None); // → 3
        status.ban_for(Duration::from_secs(30), Some("rl".into()));
        assert_eq!(
            status.ban_count, 3,
            "ban_for must not inflate ban_count above its existing value"
        );
    }

    #[test]
    fn test_address_list_ban_for_returns_false_for_unknown() {
        let list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        assert!(!list.ban_for(&addr, Duration::from_secs(5), None));
    }

    #[test]
    fn test_address_list_ban_for_bans_known_address() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        assert!(list.ban_for(&addr, Duration::from_secs(60), Some("rl".into())));
        // The address must now be hidden from get_live_address.
        assert!(list.get_live_address().is_none());
        // ban_count is 1 (ban_for sets max(0,1)).
        let info = list.ban_info();
        assert_eq!(info.len(), 1);
        assert!(info[0].banned);
        assert_eq!(info[0].ban_count, 1);
    }

    /// After `ban_for`'s window expires the address re-enters rotation via
    /// `get_live_address`.  We verify both directions: the node is hidden during
    /// an active window, and becomes live once the window has passed.
    ///
    /// Window-expiry reinstatement is orthogonal to `unban()`: `get_live_address`
    /// reinstates a node purely on `banned_until < now` regardless of `ban_count`,
    /// so after expiry the node is live again while `is_banned()` (ban_count > 0)
    /// is still true.  This is a different path from `unban()`, which also zeroes
    /// `ban_count`.
    #[test]
    fn test_ban_for_address_re_enters_rotation_after_window_expires() {
        let mut list = AddressList::new();
        let addr: Address = "http://127.0.0.1:3000".parse().unwrap();
        list.add(addr.clone());

        // Active 300-second window → node hidden.
        assert!(list.ban_for(&addr, Duration::from_secs(300), Some("rl".into())));
        assert!(
            list.get_live_address().is_none(),
            "node must be hidden during active ban window"
        );

        // Simulate window expiry by back-dating banned_until — do NOT touch ban_count.
        {
            let mut guard = list.addresses.write().unwrap();
            let status = guard.get_mut(&addr).expect("addr must be in list");
            status.banned_until = Some(chrono::Utc::now() - Duration::from_secs(1));
        }

        // After window expiry the node must re-enter rotation …
        assert!(
            list.get_live_address().is_some(),
            "address must re-enter rotation after ban window expires"
        );
        // … but ban_count is still > 0, so is_banned() remains true.
        // This distinguishes window-expiry from an explicit unban().
        assert!(
            list.is_banned(&addr),
            "is_banned() must still be true after window expiry (ban_count not reset)"
        );
    }

    /// Invariant 1 at the ladder source: the exponential ban window is
    /// `base × e^ban_count`, `ban_count` incrementing on each ban. This pins the
    /// exact formula independently of the `update_address_ban_status` entrypoint.
    #[test]
    fn test_ban_ladder_windows_match_exponential_formula() {
        let mut status = AddressStatus::default();
        let base_secs = 60.0_f64;
        let base = Duration::from_secs(60);

        for n in 0..3usize {
            // coefficient uses ban_count BEFORE this ban (== n here).
            let before = chrono::Utc::now();
            status.ban(&base);
            let after = chrono::Utc::now();

            assert_eq!(status.ban_count, n + 1, "ban_count must increment");
            let period = base_secs * (n as f64).exp();
            let banned_until = status.banned_until.expect("banned_until is set");
            let lower = (banned_until - before).num_milliseconds() as f64 / 1000.0;
            let upper = (banned_until - after).num_milliseconds() as f64 / 1000.0;
            assert!(
                lower >= period - 0.05,
                "ban #{} window lower bound {lower}s < expected {period}s",
                n + 1
            );
            assert!(
                upper <= period + 0.05,
                "ban #{} window upper bound {upper}s > expected {period}s",
                n + 1
            );
        }
    }
}
