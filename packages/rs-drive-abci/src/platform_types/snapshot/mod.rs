use crate::error::Error;
use bincode::{config, Decode, Encode};
use drive::error::drive::DriveError;
use drive::error::Error::{Drive, GroveDB};
use drive::grovedb::GroveDb;
use std::path::{Path, PathBuf};
use tenderdash_abci::proto::abci;

const SNAPSHOT_KEY: &[u8] = b"snapshots";

const DEFAULT_FREQ: i64 = 3;

const DEFAULT_NUMBER_OF_SNAPSHOTS: usize = 10;

const SNAPSHOT_VERSION: u16 = 1;

/// Snapshot entity
#[derive(Clone, Encode, Decode, PartialEq, Debug)]
pub struct Snapshot {
    /// Block height
    pub height: i64,
    /// Version
    pub version: u16,
    /// Path to the checkpoint
    pub path: String,
    /// Root hash of the checkpoint
    pub hash: [u8; 32],
    /// Metadata
    pub metadata: Vec<u8>,
}

impl From<Snapshot> for abci::Snapshot {
    fn from(snapshot: Snapshot) -> Self {
        abci::Snapshot {
            height: snapshot.height as u64,
            version: snapshot.version as u32,
            hash: snapshot.hash.into(),
            metadata: snapshot.metadata,
        }
    }
}

/// Snapshot manager is responsible for creating and managing snapshots to keep only the certain
/// number of snapshots and remove the old ones
pub struct Manager {
    freq: i64,
    number_stored_snapshots: usize,
    checkpoints_path: String,
}

impl Manager {
    /// Create a new instance of snapshot manager
    pub fn new(db_path: String, number_stored_snapshots: Option<usize>, freq: Option<i64>) -> Self {
        let mut manager = Self {
            freq: DEFAULT_FREQ,
            number_stored_snapshots: DEFAULT_NUMBER_OF_SNAPSHOTS,
            checkpoints_path: db_path,
        };
        match freq {
            Some(freq) => manager.freq = freq,
            _ => {}
        }
        match number_stored_snapshots {
            Some(number) => manager.number_stored_snapshots = number,
            _ => {}
        }
        manager
    }

    /// Return a persisted list of snapshots
    pub fn get_snapshots(&self, grove: &GroveDb) -> Result<Vec<Snapshot>, Error> {
        let data = grove
            .get_aux(SNAPSHOT_KEY, None)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        match data {
            Some(data) => {
                let conf = config::standard();
                let (mut decoded, _): (Vec<Snapshot>, usize) =
                    bincode::decode_from_slice(data.as_slice(), conf)
                        .map_err(|e| Error::Drive(Drive(DriveError::Snapshot(e.to_string()))))?;
                decoded.sort_by(|a, b| a.height.cmp(&b.height));
                Ok(decoded)
            }
            None => Ok(vec![]),
        }
    }

    /// Create a new snapshot for the given height, if a height is not a multiple of N,
    /// it will be skipped.
    pub fn create_snapshot(&self, grove: &GroveDb, height: i64) -> Result<(), Error> {
        if height == 0 || height % self.freq != 0 {
            return Ok(());
        }
        let mut checkpoint_path: PathBuf = [self.checkpoints_path.clone(), height.to_string()]
            .iter()
            .collect();
        grove
            .create_checkpoint(&checkpoint_path)
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        let root_hash = grove
            .root_hash(None)
            .unwrap()
            .map_err(|e| Error::Drive(Drive(DriveError::Snapshot(e.to_string()))))?;

        let snapshot = Snapshot {
            height,
            version: SNAPSHOT_VERSION,
            path: checkpoint_path.to_str().unwrap().to_string(),
            hash: root_hash as [u8; 32],
            metadata: vec![],
        };

        let mut snapshots = self.get_snapshots(grove)?;
        snapshots.push(snapshot);
        snapshots = self.prune_excess_snapshots(snapshots)?;
        self.save_snapshots(grove, snapshots)
    }

    fn prune_excess_snapshots(&self, snapshots: Vec<Snapshot>) -> Result<Vec<Snapshot>, Error> {
        if snapshots.len() <= self.number_stored_snapshots {
            return Ok(snapshots);
        }
        let separator = snapshots.len() - self.number_stored_snapshots;
        for snapshot in &snapshots[0..separator] {
            if Path::new(&snapshot.path).is_dir() {
                std::fs::remove_dir_all(&snapshot.path)
                    .map_err(|e| Error::Drive(Drive(DriveError::Snapshot(e.to_string()))))?;
            }
        }
        Ok(snapshots[separator..].to_vec())
    }

    fn save_snapshots(&self, grove: &GroveDb, snapshots: Vec<Snapshot>) -> Result<(), Error> {
        let conf = config::standard();
        let data: Vec<u8> = bincode::encode_to_vec(snapshots, conf)
            .map_err(|e| Error::Drive(Drive(DriveError::Snapshot(e.to_string()))))?;
        grove
            .put_aux(SNAPSHOT_KEY, data.as_slice(), None, None)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_snapshot() {
        let test_cases = vec![
            (1000, 1000, vec![1000]),
            (1000, 1001, vec![1000, 1001]),
            (1000, 1002, vec![1000, 1001, 1002]),
            (1000, 1004, vec![1002, 1003, 1004]),
            (1000, 1005, vec![1003, 1004, 1005]),
        ];
        for (start, end, want) in test_cases {
            let grove_dir = tempfile::tempdir().unwrap();
            let checkpoints_dir = tempfile::tempdir().unwrap();
            let grove = GroveDb::open(grove_dir.path()).unwrap();
            let manager = Manager::new(
                checkpoints_dir.path().to_str().unwrap().to_string(),
                Some(3),
                Some(1),
            );
            for height in start..=end {
                manager.create_snapshot(&grove, height).unwrap();
            }
            let snapshots = manager.get_snapshots(&grove).unwrap();
            let res: Vec<i64> = snapshots.iter().map(|s| s.height).collect();
            assert_eq!(want, res);

            let paths: Vec<String> = snapshots.iter().map(|s| s.path.to_string()).collect();
            for path in paths {
                assert!(Path::new(&path).exists());
            }
            fs::remove_dir_all(grove_dir.path()).unwrap();
        }
    }
}
