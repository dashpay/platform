use sha2::{Digest, Sha256};
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    pub(super) fn hash_protxhash_with_key_data_v0(pro_tx_hash: &[u8; 32], key_data: &[u8]) -> Result<[u8; 32], Error> {
        // todo: maybe change hash functions
        let mut hasher = Sha256::new();
        hasher.update(pro_tx_hash);
        hasher.update(key_data);
        Ok(hasher
            .finalize()
            .try_into()
            .expect("expected a 32 byte hash"))
    }
}
