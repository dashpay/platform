use crate::platform_types::platform_state::PlatformState;
use drive::drive::contract::DataContractFetchInfo;
use moka::sync::Cache;
use std::sync::{Arc, Mutex};

/// Block update
#[derive(Debug)]
pub struct BlockUpdate {
    /// Data contract block cache
    pub data_contracts_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
    /// Platform state
    pub platform_state: PlatformState,
}

/// Channel to transfer block updates between app running in different threads
#[derive(Default)]
pub struct BlockUpdateChannel {
    update: Mutex<Option<BlockUpdate>>,
}

impl BlockUpdateChannel {
    /// Add a new block update to the channel
    pub fn update(
        &self,
        block_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
        platform_state: PlatformState,
    ) {
        let mut update = self.update.lock().unwrap();
        update.replace(BlockUpdate {
            data_contracts_cache: block_cache,
            platform_state,
        });
    }

    /// Receive the latest block update from the channel
    pub fn receive(&self) -> Option<BlockUpdate> {
        let mut update = self.update.lock().unwrap();
        update.take()
    }
}
