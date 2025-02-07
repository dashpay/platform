use crate::abci::app::{PlatformApplication, SnapshotManagerApplication};
use bincode::{Decode, Encode};
use drive::grovedb::GroveDb;
use tenderdash_abci::proto::abci as proto;
//use platform_version::version::PlatformVersion;
use crate::abci::AbciError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

pub fn load_snapshot_chunk<A, C>(
    app: &A,
    request: proto::RequestLoadSnapshotChunk,
) -> Result<proto::ResponseLoadSnapshotChunk, Error>
where
    A: SnapshotManagerApplication + PlatformApplication<C>,
    C: CoreRPCLike,
{
    tracing::trace!(
        "[state_sync] api load_snapshot_chunk height:{} chunk_id:{}",
        request.height,
        hex::encode(&request.chunk_id)
    );
    let matched_snapshot = app
        .snapshot_manager()
        .get_snapshot_at_height(&app.platform().drive.grove, request.height as i64)
        .map_err(|_| {
            AbciError::StateSyncInternalError(
                "load_snapshot_chunk failed: error matched snapshot".to_string(),
            )
        })?
        .ok_or_else(|| {
            AbciError::StateSyncInternalError(
                "load_snapshot_chunk failed: empty matched snapshot".to_string(),
            )
        })?;
    let db = GroveDb::open(&matched_snapshot.path).map_err(|e| {
        AbciError::StateSyncInternalError(format!(
            "load_snapshot_chunk failed: error opening grove:{}",
            e
        ))
    })?;
    let chunk = db
        .fetch_chunk(
            &request.chunk_id,
            None,
            request.version as u16,
            &PlatformVersion::latest().drive.grove_version,
        )
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "load_snapshot_chunk failed: error fetching chunk{}",
                e
            ))
        })?;

    // wrap chunk data with some metadata
    let chunk_data = ChunkData::new(&chunk).serialize()?;
    let response = proto::ResponseLoadSnapshotChunk { chunk: chunk_data };
    Ok(response)
}

fn crc32(data: &[u8]) -> [u8; 4] {
    let crc32 = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);
    let mut digest = crc32.digest();
    digest.update(data);
    digest.finalize().to_le_bytes()
}

// ChunkData wraps binary chunk data with additional metadata, like checksum and size.
//
// There is no way to update the chunk - create new ChunkData instead.
//
// TODO: Use Platform encoding instead of raw binencode?
#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct ChunkData {
    version: u8,
    crc32: [u8; 4],
    size: u64,
    chunk: Vec<u8>,
}

const CHUNK_VERSION: u8 = 1;

impl ChunkData {
    pub fn new(chunk_data: &[u8]) -> Self {
        let crc32 = crc32(chunk_data);
        let size = chunk_data.len() as u64;
        ChunkData {
            chunk: chunk_data.to_vec(),
            crc32,
            size,
            version: CHUNK_VERSION,
        }
    }

    pub fn chunk(&self) -> &[u8] {
        &self.chunk
    }

    // serialize ChunkData to bytes to send to Tenderdash.
    pub fn serialize(&mut self) -> Result<Vec<u8>, Error> {
        tracing::trace!(
            checksum = hex::encode(self.crc32),
            size = self.size,
            "state_sync crc32 checksum calculated"
        );
        let data: &ChunkData = self;

        bincode::encode_to_vec(data, bincode::config::standard()).map_err(|e| {
            tracing::error!(error = ?e, "state_sync failed to encode chunk data");
            Error::Abci(AbciError::StateSyncInternalError(format!(
                "failed to encode chunk data: {}",
                e
            )))
        })
    }

    // verify chunk checksums, etc.
    pub fn verify(&self) -> Result<(), Error> {
        if self.version != CHUNK_VERSION {
            return Err(Error::Abci(AbciError::StateSyncInternalError(format!(
                "state_sync chunk version mismatch: expected {}, got {}",
                CHUNK_VERSION, self.version
            ))));
        }

        if self.size != self.chunk.len() as u64 {
            return Err(Error::Abci(AbciError::StateSyncInternalError(format!(
                "state_sync chunk size mismatch: expected {}, got {}",
                self.size,
                self.chunk.len()
            ))));
        }

        let checksum = crc32(&self.chunk);
        if self.crc32 != checksum {
            tracing::error!(
                checksum = hex::encode(checksum),
                received = hex::encode(self.crc32),
                "state_sync crc32 checksum mismatch",
            );
            return Err(Error::Abci(AbciError::StateSyncInternalError(format!(
                "state_sync crc32 checksum mismatch: expected {}, got {}",
                hex::encode(self.crc32),
                hex::encode(checksum),
            ))));
        }
        tracing::trace!(
            checksum = hex::encode(checksum),
            "state_sync crc32 checksum verified"
        );

        Ok(())
    }

    // deserialize ChunkData from bytes received from Tenderdash and verifies it.
    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        let (data, _): (ChunkData, _) =
            bincode::decode_from_slice(data, bincode::config::standard()).map_err(|e| {
                tracing::error!(error = ?e, "state_sync failed to decode chunk data");
                Error::Abci(AbciError::StateSyncInternalError(
                    "failed to decode chunk data".to_string(),
                ))
            })?;

        data.verify()?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_chunk_data_match() {
        let chunk = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let serialized_chunk_data = ChunkData::new(&chunk).serialize().unwrap();
        assert_ne!(chunk, serialized_chunk_data);

        let deserialized_chunk_data = ChunkData::deserialize(&serialized_chunk_data).unwrap();
        let deserialized_chunk = deserialized_chunk_data.chunk();
        assert_eq!(chunk, deserialized_chunk);
    }

    #[test]
    fn test_chunk_data_mismatch() {
        let chunk = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let serialized_chunk_data = ChunkData::new(&chunk).serialize().unwrap();
        assert_ne!(chunk, serialized_chunk_data);

        let mut deserialized_chunk_data = ChunkData::deserialize(&serialized_chunk_data).unwrap();
        deserialized_chunk_data.chunk[7] = 0;
        let deserialized_chunk = deserialized_chunk_data.chunk();
        assert_ne!(chunk, deserialized_chunk);
    }
}
