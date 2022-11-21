/// Epoch key constants module
pub mod epoch_key_constants;
/// Operations factory module
pub mod operations_factory;
/// Paths module
pub mod paths;

use serde::{Deserialize, Serialize};

// TODO: I would call it EpochTree because it represent pool,
//  not just Epoch which is more abstract thing that we will probably need in future too

/// Epoch struct
#[derive(Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    /// Epoch index
    pub index: u16,
    /// Epoch key
    pub(crate) key: [u8; 2],
}

impl Epoch {
    /// Create new epoch
    pub fn new(index: u16) -> Self {
        let key = paths::encode_epoch_index_key(index).expect("epoch index is too high");

        Self { index, key }
    }
}
