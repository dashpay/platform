pub(crate) mod cleaned_block;
pub(crate) mod cleaned_block_id;
pub(crate) mod cleaned_commit_info;
pub(crate) mod cleaned_header;
pub(crate) mod finalized_block_cleaned_request;

fn hash_or_default(hash: Vec<u8>) -> Result<[u8; 32], <Vec<u8> as TryInto<[u8; 32]>>::Error> {
    if hash.is_empty() {
        // hash is empty at genesis, we assume it is zeros
        Ok([0u8; 32])
    } else {
        hash.try_into()
    }
}
