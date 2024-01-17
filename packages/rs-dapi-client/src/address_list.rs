//! Subsystem to manage peers.

use std::time;

use http::Uri;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};

const DEFAULT_BASE_BAN_PERIOD: time::Duration = time::Duration::from_secs(60);

/// Peer's address.
#[derive(Debug)]
pub struct Address {
    base_ban_period: time::Duration,
    ban_count: usize,
    banned_until: Option<time::Instant>,
    uri: Uri,
}

impl Address {
    /// Ban the [Address] so it won't be available through [AddressList::get_live_address] for some time.
    pub fn ban(&mut self) {
        let coefficient = (self.ban_count as f64).exp();
        let ban_period =
            time::Duration::from_secs_f64(self.base_ban_period.as_secs_f64() * coefficient);

        self.banned_until = Some(time::Instant::now() + ban_period);
        self.ban_count += 1;
    }

    /// Clears ban record.
    pub fn clear_ban(&mut self) {
        self.ban_count = 0;
        self.banned_until = None;
    }

    /// Get [Uri] of a peer.
    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}

/// A structure to manage peer's addresses to select from
/// for [DapiRequest](crate::DapiRequest) execution.
#[derive(Debug)]
pub struct AddressList {
    addresses: Vec<Address>,
    base_ban_period: time::Duration,
}

impl Default for AddressList {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressList {
    /// Creates an empty [AddressList] with default base ban time.
    pub fn new() -> Self {
        AddressList::with_settings(DEFAULT_BASE_BAN_PERIOD)
    }

    /// Creates an empty [AddressList] with adjustable base ban time.
    pub fn with_settings(base_ban_period: time::Duration) -> Self {
        AddressList {
            addresses: Vec::new(),
            base_ban_period,
        }
    }

    // TODO: this is the most simple way to add an address
    // however we need to support bulk loading (e.g. providing a network name)
    // and also fetch updated from SML.
    /// Manually add a peer to [AddressList].
    pub fn add_uri(&mut self, uri: Uri) {
        self.addresses.push(Address {
            ban_count: 0,
            banned_until: None,
            base_ban_period: self.base_ban_period,
            uri,
        });
    }

    /// Randomly select a not banned address.
    pub fn get_live_address(&mut self) -> Option<&mut Address> {
        let now = time::Instant::now();
        let mut rng = SmallRng::from_entropy();
        self.addresses
            .iter_mut()
            .filter(|addr| {
                addr.banned_until
                    .map(|banned_until| banned_until < now)
                    .unwrap_or(true)
            })
            .choose(&mut rng)
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
