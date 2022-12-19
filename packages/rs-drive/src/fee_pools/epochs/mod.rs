/// Epoch key constants module
pub mod epoch_key_constants;
pub mod operations_factory;
pub mod paths;

use crate::fee::epoch::EpochIndex;
use serde::{Deserialize, Serialize};

// TODO: I would call it EpochTree because it represent pool,
//  not just Epoch which is more abstract thing that we will probably need in future too

/// Epoch struct
#[derive(Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    /// Epoch index
    pub index: EpochIndex,
    /// Epoch key
    pub(crate) key: [u8; 2],
}

impl Epoch {
    /// Create new epoch
    pub fn new(index: EpochIndex) -> Self {
        let key = paths::encode_epoch_index_key(index).expect("epoch index is too high");

        Self { index, key }
    }
}
