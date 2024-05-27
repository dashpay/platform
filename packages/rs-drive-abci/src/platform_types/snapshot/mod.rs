use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use bincode::{config, Decode, Encode};
use drive::error::drive::DriveError;
use drive::error::Error::{Drive, GroveDB};
use drive::grovedb::replication::MultiStateSyncSession;
use drive::grovedb::GroveDb;
use prost::Message;
use tenderdash_abci::proto::{abci as proto, abci};

use crate::error::Error;

const SNAPSHOT_KEY: &[u8] = b"snapshots";

const DEFAULT_FREQ: i64 = 3;

const DEFAULT_NUMBER_OF_SNAPSHOTS: usize = 10;

const CHUNK_SIZE_16MB: usize = 16 * 1024 * 1024;

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

/// Snapshot manager is responsible for creating and managing snapshots to keep only the certain
/// number of snapshots and remove the old ones
#[derive(Default, Clone)]
pub struct SnapshotManager {
    freq: i64,
    number_stored_snapshots: usize,
    checkpoints_path: String,
}

/// Snapshot manager is responsible for creating and managing snapshots to keep only the certain
/// number of snapshots and remove the old ones
pub struct SnapshotFetchingSession<'db> {
    /// Snapshot accepted
    pub snapshot: abci::Snapshot,
    /// Snapshot accepted
    pub app_hash: Vec<u8>,
    // sender_metrics: Option<HashMap<String, Metrics>>,
    /// Snapshot accepted
    pub state_sync_info: Pin<Box<MultiStateSyncSession<'db>>>,
}

struct Metrics {
    success: usize,
    error: usize,
}

enum MetricType {
    Success,
    Error,
}

impl Metrics {
    fn new() -> Self {
        Self {
            success: 0,
            error: 0,
        }
    }

    fn incr(&mut self, metric: MetricType) {
        match metric {
            MetricType::Success => self.success += 1,
            MetricType::Error => self.error += 1,
        }
    }
}

impl SnapshotManager {
    /// Create a new instance of snapshot manager
    pub fn new(
        checkpoints_path: String,
        number_stored_snapshots: Option<usize>,
        freq: Option<i64>,
    ) -> Self {
        Self {
            freq: freq.unwrap_or(DEFAULT_FREQ),
            number_stored_snapshots: number_stored_snapshots.unwrap_or(DEFAULT_NUMBER_OF_SNAPSHOTS),
            checkpoints_path,
        }
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
        let checkpoint_path: PathBuf = [self.checkpoints_path.clone(), height.to_string()]
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

    /*
      fn update_sender_metric(&mut self, sender: String, metric_type: MetricType) {
          let ref mut metrics = match &self.sender_metrics {
              Some(ref mut metrics) => metrics, //metrics.get_mut(&sender),
              None => {
                  let mut metrics = HashMap::new();
                  self.sender_metrics = Some(metrics);
                  &metrics
              }
          };
          let ref mut sender_metrics = match metrics.get_mut(&sender) {
              Some(sender_metrics) => sender_metrics,
              None => {
                  let mut sender_metrics = Metrics::new();
                  &sender_metrics
              }
          };
          sender_metrics.incr(metric_type);
    }

       */
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    /*
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
               let manager = SnapshotManager::new(
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
           let mut manager = SnapshotManager::new("".to_string(), Some(3), Some(1));
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
       }

    */
}
