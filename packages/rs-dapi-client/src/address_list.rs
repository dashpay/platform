//! Subsystem to manage peers.

use std::time;

use http::Uri;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};

const DEFAULT_BASE_BAN_TIME: time::Duration = time::Duration::from_secs(60);

/// TODO
#[derive(Debug)]
pub struct Address {
    base_ban_time: time::Duration,
    ban_count: usize,
    banned_until: Option<time::Instant>,
    uri: Uri,
}

impl Address {
    pub fn ban(&mut self) {
        let coefficient = (self.ban_count as f64).exp();
        let ban_period =
            time::Duration::from_secs_f64(self.base_ban_time.as_secs_f64() * coefficient);

        self.banned_until = Some(time::Instant::now() + ban_period);
        self.ban_count += 1;
    }

    pub fn clear_ban(&mut self) {
        self.ban_count = 0;
        self.banned_until = None;
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}

/// TODO
#[derive(Debug)]
pub struct AddressList {
    addresses: Vec<Address>,
    base_ban_time: time::Duration,
}

impl AddressList {
    pub fn new() -> Self {
        AddressList::with_settings(DEFAULT_BASE_BAN_TIME)
    }

    pub fn with_settings(base_ban_time: time::Duration) -> Self {
        AddressList {
            addresses: Vec::new(),
            base_ban_time,
        }
    }

    // TODO: this is the most simple way to add an address
    // however we need to support bulk loading (e.g. providing a network name)
    // and also fetch updated from SML.
    pub fn add_uri(&mut self, uri: Uri) {
        self.addresses.push(Address {
            ban_count: 0,
            banned_until: None,
            base_ban_time: self.base_ban_time,
            uri,
        });
    }

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
