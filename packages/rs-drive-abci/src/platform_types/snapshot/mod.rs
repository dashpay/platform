use crate::error::Error;
use bincode::{config, Decode, Encode};
use drive::error::drive::DriveError;
use drive::error::Error::{Drive, GroveDB};
use drive::grovedb::GroveDb;
use std::mem::take;
use std::path::Path;
use tenderdash_abci::proto::abci;
use tenderdash_abci::proto::abci::response_offer_snapshot;

const SNAPSHOT_KEY: &[u8] = b"snapshots";

const DEFAULT_FREQ: i64 = 3;

const DEFAULT_NUMBER_OF_SNAPSHOTS: usize = 10;

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

/// Offered snapshot entity
struct OfferedSnapshot {
    pub snapshot: abci::Snapshot,
    pub app_hash: Vec<u8>,
}

/// Snapshot manager is responsible for creating and managing snapshots to keep only the certain
/// number of snapshots and remove the old ones
pub struct Manager {
    freq: i64,
    number_stored_snapshots: usize,
    checkpoints_path: String,
    offered_snapshot: Option<OfferedSnapshot>,
}

impl Manager {
    /// Create a new instance of snapshot manager
    pub fn new(
        checkpoints_path: String,
        number_stored_snapshots: Option<usize>,
        freq: Option<i64>,
    ) -> Self {
        let mut manager = Self {
            freq: DEFAULT_FREQ,
            number_stored_snapshots: DEFAULT_NUMBER_OF_SNAPSHOTS,
            checkpoints_path,
            offered_snapshot: None,
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

    /// Offers a snapshot to the replication, if previously the snapshot was already offered
    /// then it will be deleted and replaced with the new one
    pub fn offer_snapshot(
        &mut self,
        grove: &GroveDb,
        snapshot: abci::Snapshot,
        app_hash: Vec<u8>,
    ) -> Result<response_offer_snapshot::Result, Error> {
        match take(&mut self.offered_snapshot) {
            Some(mut offered_snapshot) => {
                if offered_snapshot.snapshot.height == snapshot.height {
                    return Ok(response_offer_snapshot::Result::Reject);
                }
                if offered_snapshot.snapshot.version != snapshot.version {
                    return Ok(response_offer_snapshot::Result::Reject);
                }
                grove.wipe().map_err(|e| GroveDB(e))?;
            }
            _ => {}
        };
        self.offered_snapshot = Some(OfferedSnapshot { snapshot, app_hash });
        Ok(response_offer_snapshot::Result::Accept)
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
        let mut checkpoint_path = self.checkpoints_path.to_owned();
        checkpoint_path.push('/');
        checkpoint_path.push_str(height.to_string().as_str());
        grove
            .create_checkpoint(&checkpoint_path)
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        let root_hash = grove
            .root_hash(None)
            .unwrap()
            .map_err(|e| Error::Drive(Drive(DriveError::Snapshot(e.to_string()))))?;

        let snapshot = Snapshot {
            height,
            version: 0,
            path: checkpoint_path,
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

    #[test]
    fn test_offer_snapshot() {
        let grove_dir = tempfile::tempdir().unwrap();
        let replication_dir = tempfile::tempdir().unwrap();
        let grove = GroveDb::open(grove_dir.path()).unwrap();
        let mut manager = Manager::new("".to_string(), Some(3), Some(1));
        let app_hash = vec![1, 2, 3, 4, 5];
        let snapshot_1000 = abci::Snapshot {
            height: 1000,
            version: 0,
            hash: app_hash.clone(),
            metadata: vec![],
        };
        manager
            .offer_snapshot(&grove, snapshot_1000.clone(), app_hash.clone())
            .unwrap();
        let snapshot_2000 = abci::Snapshot {
            height: 2000,
            version: 0,
            hash: app_hash.clone(),
            metadata: vec![],
        };
        manager
            .offer_snapshot(&grove, snapshot_2000.clone(), app_hash.clone())
            .unwrap();
        fs::remove_dir_all(grove_dir.path()).unwrap();
    }
}
