pub mod epoch_key_constants;
pub mod operations_factory;
pub mod paths;

use serde::{Deserialize, Serialize};

// TODO: I would call it EpochTree because it represent pool,
//  not just Epoch which is more abstract thing that we will probably need in future too

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    pub index: u16,
    pub(crate) key: [u8; 2],
}

impl Epoch {
    pub fn new(index: u16) -> Self {
        let key = paths::encode_epoch_index_key(index).expect("epoch index is too high");

        Self { index, key }
    }
}
