//! The storage engine GroveDB is opened on, and with what options.
//!
//! [`DriveConfig::grovedb_storage_engine`] names the engine. Which engines a build can
//! open is GroveDB's to say, by its cfg `grovedb_default_engine`. Where that is `either`,
//! `GroveDb` with no engine named is on the engine that is chosen when a database is
//! opened. The `.cargo` configuration of this repository asks GroveDB's build script for
//! it with `GROVEDB_DEFAULT_ENGINE=either`. Where the cfg is not set, `GroveDb` is on
//! RocksDB alone, and a configuration that names LigninDB is refused at the open.
//!
//! A database is opened here only where its directory holds a database of the engine the
//! configuration names, or, for the database of the node, none of either engine. What
//! the entries of a directory say it holds is told as GroveDB tells it
//! (`EitherEngine::found_at`).

use crate::config::{DriveConfig, LigninDbSync, StorageEngine};
use crate::drive::Drive;
use crate::error::Error;
use grovedb::engine::either::lignindb::checkpoint::{CREATING_SUFFIX, DELETING_FILE_NAME};
use grovedb::engine::either::lignindb::db::DbStats;
use grovedb::engine::either::lignindb::Options;
use grovedb::engine::either::SyncPolicy;
use grovedb::engine::{
    DefaultEngine, EitherEngine, Engine, EngineChoice, EngineKind, RocksDbEngine,
};
use grovedb::GroveDb;
use std::path::Path;
use std::time::Duration;

/// What a database is opened as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// The database of the node. It is made where there is none.
    Live,
    /// A checkpoint. None is made where there is none.
    Checkpoint,
}

/// What Drive asks of the engine that `GroveDb` with no engine named is on.
trait ChosenAtTheOpen: Engine + Sized {
    fn open_for(path: &Path, config: &DriveConfig, role: Role) -> Result<Self, grovedb::Error>;

    fn remove_checkpoint(engine: StorageEngine, path: &Path) -> Result<(), grovedb::Error>;

    fn held(&self) -> StorageEngine;

    fn lignindb_stats(&self) -> Option<DbStats>;
}

fn rocksdb_alone(engine: StorageEngine) -> Result<(), grovedb::Error> {
    match engine {
        StorageEngine::RocksDb => Ok(()),
        StorageEngine::LigninDb => Err(grovedb::Error::NotSupported(
            "the configuration names LigninDB and this build has GroveDB on RocksDB alone: \
             the storage engine is chosen at the open where GroveDB is built with \
             GROVEDB_DEFAULT_ENGINE=either"
                .to_string(),
        )),
    }
}

/// Refuses a directory that RocksDB is not to open as `role`: one whose entries say
/// LigninDB or both engines, and, for a checkpoint, one that has no file named `CURRENT`,
/// without which RocksDB finds no database and would make one. The directory is listed
/// and nothing in it is opened or changed. The look and the open that follows it are two
/// steps: the directory is the node's own, and what another writer does to it between
/// them is not seen.
fn rocksdb_opens(path: &Path, role: Role) -> Result<(), grovedb::Error> {
    let found = EitherEngine::found_at(path)?;
    if found == Some(EngineKind::Lignin) {
        return Err(grovedb::Error::NotSupported(format!(
            "the directory {} holds a database of LigninDB and the configuration names \
             RocksDB: it is not opened",
            path.display()
        )));
    }
    if role == Role::Checkpoint && !path.join("CURRENT").is_file() {
        return Err(grovedb::Error::NotSupported(format!(
            "the directory {} has no file `CURRENT` of a RocksDB database: it is not opened \
             as a checkpoint",
            path.display()
        )));
    }
    Ok(())
}

impl ChosenAtTheOpen for RocksDbEngine {
    fn open_for(path: &Path, config: &DriveConfig, role: Role) -> Result<Self, grovedb::Error> {
        rocksdb_alone(config.grovedb_storage_engine)?;
        rocksdb_opens(path, role)?;
        Ok(RocksDbEngine::open(path)?)
    }

    fn remove_checkpoint(engine: StorageEngine, path: &Path) -> Result<(), grovedb::Error> {
        rocksdb_alone(engine)?;
        RocksDbEngine::delete_checkpoint(path)
    }

    fn held(&self) -> StorageEngine {
        StorageEngine::RocksDb
    }

    fn lignindb_stats(&self) -> Option<DbStats> {
        None
    }
}

/// The options of a database on LigninDB. A checkpoint gets its cache and nothing else of
/// the configuration: neither the sync policy nor a reserve.
fn lignindb_options(config: &DriveConfig, role: Role) -> Result<Options, grovedb::Error> {
    let caches = config
        .lignindb_caches()
        .map_err(grovedb::Error::NotSupported)?;
    Ok(match role {
        Role::Live => Options {
            cache_size: caches.live,
            sync: match config.grovedb_lignindb_sync {
                LigninDbSync::Never => SyncPolicy::Never,
                LigninDbSync::IntervalMs(milliseconds) => {
                    SyncPolicy::Interval(Duration::from_millis(milliseconds))
                }
                LigninDbSync::EveryCommit => SyncPolicy::EveryCommit,
            },
            reserve: config.grovedb_lignindb_reserve_bytes,
            ..Options::default()
        },
        Role::Checkpoint => Options {
            cache_size: caches.checkpoint,
            ..Options::default()
        },
    })
}

impl ChosenAtTheOpen for EitherEngine {
    /// A database on RocksDB is opened with the options RocksDB opens every database
    /// with, whatever its role, as it was before an engine could be chosen.
    fn open_for(path: &Path, config: &DriveConfig, role: Role) -> Result<Self, grovedb::Error> {
        Ok(match (config.grovedb_storage_engine, role) {
            (StorageEngine::RocksDb, _) => {
                rocksdb_opens(path, role)?;
                EitherEngine::open_with(path, &EngineChoice::RocksDb)?
            }
            (StorageEngine::LigninDb, Role::Live) => {
                let choice = EngineChoice::Lignin(lignindb_options(config, role)?);
                EitherEngine::open_with(path, &choice)?
            }
            (StorageEngine::LigninDb, Role::Checkpoint) => {
                let choice = EngineChoice::Lignin(lignindb_options(config, role)?);
                EitherEngine::open_checkpoint_with(path, &choice)?
            }
        })
    }

    fn remove_checkpoint(engine: StorageEngine, path: &Path) -> Result<(), grovedb::Error> {
        match engine {
            StorageEngine::RocksDb => RocksDbEngine::delete_checkpoint(path),
            StorageEngine::LigninDb => EitherEngine::delete_checkpoint_of(EngineKind::Lignin, path),
        }
    }

    fn held(&self) -> StorageEngine {
        match self.kind() {
            EngineKind::RocksDb => StorageEngine::RocksDb,
            EngineKind::Lignin => StorageEngine::LigninDb,
        }
    }

    fn lignindb_stats(&self) -> Option<DbStats> {
        self.lignin().map(|engine| engine.database().stats())
    }
}

/// Opens the database at `path` as `role`, on the engine and with the options that
/// `config` names.
pub(crate) fn open_grove(path: &Path, config: &DriveConfig, role: Role) -> Result<GroveDb, Error> {
    let engine = <DefaultEngine as ChosenAtTheOpen>::open_for(path, config, role)?;
    Ok(GroveDb::from_engine(engine))
}

/// Whether no entry of the directory at `path` says a database of either engine. An empty
/// directory and one that has lost every entry that says a database give the same answer.
/// A directory that cannot be listed is not one.
pub(crate) fn no_entry_says_a_database(path: &Path) -> bool {
    matches!(EitherEngine::found_at(path), Ok(None))
}

/// The storage engine `grove` is on.
pub(crate) fn storage_engine_of(grove: &GroveDb) -> StorageEngine {
    grove.engine().held()
}

/// The counts LigninDB keeps of `grove`, where that is the engine it is on: among them the
/// memory of its cache (`memory`) and what it did with its reserve for a full disk
/// (`reserve`).
pub fn lignindb_stats_of(grove: &GroveDb) -> Option<DbStats> {
    grove.engine().lignindb_stats()
}

fn not_found(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
}

/// Whether a directory has the name LigninDB builds the checkpoint at `path` under:
/// the name of `path` with a suffix. A creation that was cut short can leave one.
fn a_directory_is_beside(path: &Path) -> std::io::Result<bool> {
    let Some(name) = path.file_name() else {
        return Ok(false);
    };
    let mut building = name.to_os_string();
    building.push(CREATING_SUFFIX);
    match std::fs::symlink_metadata(path.with_file_name(building)) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if not_found(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

/// [`Checkpoint::delete_closed`](crate::drive::Checkpoint::delete_closed). On RocksDB the
/// directory is listed once, here, and RocksDB's own removal is then asked without
/// another listing. The listing, that removal and the removal behind it are separate
/// steps on a path: the directory is the node's own, and one that another writer puts
/// in its place between them is not told from it.
pub(crate) fn delete_closed_checkpoint(path: &Path, engine: StorageEngine) -> std::io::Result<()> {
    let unless_gone = |removed: std::io::Result<()>| match removed {
        Err(error) if !not_found(&error) => Err(error),
        _ => Ok(()),
    };
    let told = |error: grovedb::Error| std::io::Error::other(error.to_string());
    let not_listed = |error| std::io::Error::other(grovedb::Error::from(error).to_string());
    let missing = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return unless_gone(std::fs::remove_file(path));
        }
        Ok(_) => false,
        Err(error) if not_found(&error) => true,
        Err(error) => return Err(error),
    };
    let removal = || <DefaultEngine as ChosenAtTheOpen>::remove_checkpoint(engine, path);
    match engine {
        StorageEngine::RocksDb if missing => Ok(()),
        StorageEngine::RocksDb => match EitherEngine::found_at(path).map_err(not_listed)? {
            Some(EngineKind::Lignin) => Err(std::io::Error::other(format!(
                "the directory {} holds a database of LigninDB: it is not deleted as a \
                 checkpoint of RocksDB",
                path.display()
            ))),
            None => unless_gone(std::fs::remove_dir_all(path)),
            Some(EngineKind::RocksDb) => {
                let Err(refused) = removal() else {
                    return Ok(());
                };
                tracing::warn!(
                    error = ?refused,
                    ?path,
                    "grovedb did not delete a checkpoint; removing its directory"
                );
                unless_gone(std::fs::remove_dir_all(path))
            }
        },
        StorageEngine::LigninDb => {
            if missing && !a_directory_is_beside(path)? {
                return Ok(());
            }
            removal().map_err(told)
        }
    }
}

/// The name of LigninDB's manifest in the directory of a database.
const LIGNINDB_MANIFEST: &str = "MANIFEST";

/// Hands the directory at `path` to LigninDB's removal where it looks like what a creation
/// or a deletion of a checkpoint left that was cut short:
///
/// - a directory under the name a checkpoint of a height is built under, whatever it
///   holds, or
/// - a directory named after a height that holds a file with the name of a deletion's
///   mark and nothing with the name of the manifest. LigninDB then removes it only where
///   the mark holds a manifest.
///
/// A directory named after a height that holds both names is not handed over: the removal
/// would take a manifest that is whole for the database to delete, and the mark may be a
/// file of something else. It is left for the open to refuse and an operator to look at.
/// Nor is a symbolic link, whatever it names. `None` where `path` is none of these, or
/// where `config` does not name LigninDB.
pub(crate) fn remove_checkpoint_leftover(
    path: &Path,
    config: &DriveConfig,
) -> Option<std::io::Result<()>> {
    if config.grovedb_storage_engine != StorageEngine::LigninDb {
        return None;
    }
    let is_a_directory = std::fs::symlink_metadata(path).is_ok_and(|entry| entry.is_dir());
    let is_a_height = |name: &str| name.parse::<u64>().is_ok();
    let name = path.file_name()?.to_str()?;
    let named_as_being_built = name.strip_suffix(CREATING_SUFFIX).is_some_and(is_a_height);
    let marked_as_being_deleted = || {
        let mark = std::fs::symlink_metadata(path.join(DELETING_FILE_NAME));
        let manifest = std::fs::symlink_metadata(path.join(LIGNINDB_MANIFEST));
        is_a_height(name)
            && mark.is_ok_and(|mark| mark.is_file())
            && manifest.is_err_and(|error| not_found(&error))
    };
    (is_a_directory && (named_as_being_built || marked_as_being_deleted()))
        .then(|| delete_closed_checkpoint(path, StorageEngine::LigninDb))
}

impl Drive {
    /// Opens the database at `path` alone, with none of the rest of a Drive, on the storage
    /// engine and with the options `config` names, as [`Drive::open`] opens it.
    pub fn open_grovedb<P: AsRef<Path>>(path: P, config: &DriveConfig) -> Result<GroveDb, Error> {
        open_grove(path.as_ref(), config, Role::Live)
    }

    /// The storage engine the database is on.
    pub fn storage_engine(&self) -> StorageEngine {
        storage_engine_of(&self.grove)
    }

    /// The counts LigninDB keeps of the database, where that is its storage engine.
    pub fn lignindb_stats(&self) -> Option<DbStats> {
        lignindb_stats_of(&self.grove)
    }

    /// Opens the checkpoint at `path` on the storage engine of this Drive's configuration.
    /// On LigninDB it gets the cache the configuration gives a checkpoint. No database is
    /// made: a directory in which the engine would find none is refused before the engine
    /// is asked, on RocksDB by its `CURRENT` file, and nothing is written to a directory
    /// that is refused so. An engine that is asked and finds a damaged database can have
    /// made its lock file by the time it fails.
    pub fn open_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<GroveDb, Error> {
        open_grove(path.as_ref(), &self.config, Role::Checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb::engine::LigninEngine;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn on(engine: StorageEngine) -> DriveConfig {
        DriveConfig {
            grovedb_storage_engine: engine,
            ..Default::default()
        }
    }

    /// Whether `GroveDb` is on the engine that is chosen at the open in this build.
    fn the_engine_is_chosen_at_the_open() -> bool {
        std::any::type_name::<DefaultEngine>() == std::any::type_name::<EitherEngine>()
    }

    /// The engines a database can be opened on in this build.
    fn engines() -> Vec<StorageEngine> {
        match the_engine_is_chosen_at_the_open() {
            true => vec![StorageEngine::RocksDb, StorageEngine::LigninDb],
            false => vec![StorageEngine::RocksDb],
        }
    }

    /// Every entry below `path` with what it holds: the bytes of a file, and nothing for
    /// anything else.
    fn contents(path: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        let mut found = BTreeMap::new();
        let mut directories = vec![path.to_path_buf()];
        while let Some(directory) = directories.pop() {
            for entry in std::fs::read_dir(&directory).expect("the entries") {
                let entry = entry.expect("an entry").path();
                let metadata = std::fs::symlink_metadata(&entry).expect("the entry");
                let held = metadata
                    .is_file()
                    .then(|| std::fs::read(&entry).expect("the file"));
                if metadata.is_dir() {
                    directories.push(entry.clone());
                }
                found.insert(entry, held);
            }
        }
        found
    }

    /// A build that asks GroveDB for the engine chosen at the open, as this repository's
    /// does, has it.
    #[test]
    fn a_build_that_asks_for_the_choice_of_engine_has_it() {
        if option_env!("GROVEDB_DEFAULT_ENGINE") == Some("either") {
            assert!(the_engine_is_chosen_at_the_open());
        }
    }

    #[test]
    fn a_database_is_opened_on_the_engine_the_configuration_names() {
        for engine in engines() {
            let directory = tempfile::tempdir().expect("a directory");
            let (drive, _) =
                Drive::open(directory.path().join("db"), Some(on(engine))).expect("a database");
            assert_eq!(drive.storage_engine(), engine);
        }
    }

    /// Where the build has GroveDB on RocksDB alone, a configuration that names LigninDB
    /// is refused and no directory is made.
    #[test]
    fn lignindb_is_opened_only_where_the_engine_is_chosen_at_the_open() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        let opened = Drive::open(&path, Some(on(StorageEngine::LigninDb)));
        match (the_engine_is_chosen_at_the_open(), opened) {
            (true, Ok((drive, _))) => assert_eq!(drive.storage_engine(), StorageEngine::LigninDb),
            (false, Err(refused)) => {
                assert!(
                    refused
                        .to_string()
                        .contains("GROVEDB_DEFAULT_ENGINE=either"),
                    "{refused}"
                );
                assert!(!path.exists(), "the refused open made the directory");
            }
            (true, Err(refused)) => panic!("LigninDB was refused: {refused}"),
            (false, Ok(_)) => panic!("a build on RocksDB alone opened LigninDB"),
        }
    }

    /// A database of LigninDB is opened by RocksDB neither as the database of a node nor
    /// as a checkpoint, in a build with the choice of engine and in one on RocksDB alone,
    /// and neither its directory nor any file below it is changed.
    #[test]
    fn a_database_of_lignindb_is_not_opened_by_rocksdb_and_is_left_as_it_was() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        drop(GroveDb::<LigninEngine>::open_with_engine(&path).expect("a database of LigninDB"));
        let before = contents(&path);
        assert!(before.values().any(Option::is_some), "a database has files");

        for role in [Role::Live, Role::Checkpoint] {
            let Err(refused) = open_grove(&path, &on(StorageEngine::RocksDb), role) else {
                panic!("a database of LigninDB was opened by RocksDB as {role:?}");
            };
            assert!(
                matches!(refused, Error::GroveDB(_)),
                "unexpected error: {refused:?}"
            );
            assert!(
                contents(&path) == before,
                "the refused open as {role:?} changed the database"
            );
        }
    }

    /// A database of RocksDB is opened by LigninDB neither as the database of a node nor
    /// as a checkpoint, and neither its directory nor any file below it is changed.
    #[test]
    fn a_database_of_rocksdb_is_not_opened_by_lignindb_and_is_left_as_it_was() {
        if !the_engine_is_chosen_at_the_open() {
            return;
        }
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        drop(Drive::open(&path, Some(on(StorageEngine::RocksDb))).expect("a database"));
        let before = contents(&path);
        assert!(before.values().any(Option::is_some), "a database has files");

        for role in [Role::Live, Role::Checkpoint] {
            let Err(refused) = open_grove(&path, &on(StorageEngine::LigninDb), role) else {
                panic!("a database of RocksDB was opened by LigninDB as {role:?}");
            };
            assert!(
                matches!(refused, Error::GroveDB(_)),
                "unexpected error: {refused:?}"
            );
            assert!(
                contents(&path) == before,
                "the refused open as {role:?} changed the database"
            );
        }
        let (drive, _) =
            Drive::open(&path, Some(on(StorageEngine::RocksDb))).expect("its own engine");
        assert_eq!(drive.storage_engine(), StorageEngine::RocksDb);
    }

    /// A directory that holds a file with a name of RocksDB's and no database is no
    /// checkpoint on either engine: RocksDB, which would make a database in it, is not
    /// asked, and the directory is left as it was.
    #[test]
    fn a_directory_with_a_name_of_rocksdbs_and_no_database_is_no_checkpoint() {
        for engine in engines() {
            let directory = tempfile::tempdir().expect("a directory");
            let (drive, _) =
                Drive::open(directory.path().join("db"), Some(on(engine))).expect("a database");
            let path = directory.path().join("7");
            std::fs::create_dir(&path).expect("a directory");
            std::fs::write(path.join("OPTIONS-000007"), b"options").expect("a file");
            let before = contents(&path);

            assert!(drive.open_checkpoint(&path).is_err(), "on {engine}");
            assert!(
                contents(&path) == before,
                "the refused open changed the directory on {engine}"
            );
        }
    }

    /// The options a database of LigninDB is opened with are the configuration's: the
    /// live database has what the budget leaves, its sync policy and its reserve, and a
    /// checkpoint its cache and no reserve.
    #[test]
    fn the_options_of_lignindb_are_the_configurations() {
        let config = DriveConfig {
            grovedb_lignindb_memory_budget_mb: 200,
            grovedb_lignindb_checkpoint_cache_mb: 8,
            grovedb_lignindb_sync: LigninDbSync::IntervalMs(250),
            grovedb_lignindb_reserve_bytes: 4 << 20,
            open_checkpoints: 5,
            ..on(StorageEngine::LigninDb)
        };
        let live = lignindb_options(&config, Role::Live).expect("the options");
        assert_eq!(live.cache_size, (200 - 5 * 8) << 20);
        assert_eq!(live.sync, SyncPolicy::Interval(Duration::from_millis(250)));
        assert_eq!(live.reserve, 4 << 20);
        assert!(live.create_if_missing);
        let checkpoint = lignindb_options(&config, Role::Checkpoint).expect("the options");
        assert_eq!(checkpoint.cache_size, 8 << 20);
        assert_eq!(checkpoint.sync, SyncPolicy::Never);
        assert_eq!(checkpoint.reserve, 0);

        for (sync, policy) in [
            (LigninDbSync::Never, SyncPolicy::Never),
            (LigninDbSync::EveryCommit, SyncPolicy::EveryCommit),
        ] {
            let config = DriveConfig {
                grovedb_lignindb_sync: sync,
                ..on(StorageEngine::LigninDb)
            };
            let live = lignindb_options(&config, Role::Live).expect("the options");
            assert_eq!(live.sync, policy);
        }

        let too_little = DriveConfig {
            grovedb_lignindb_memory_budget_mb: 16,
            ..on(StorageEngine::LigninDb)
        };
        let refused = lignindb_options(&too_little, Role::Live).expect_err("too little");
        assert!(
            refused
                .to_string()
                .contains("GROVEDB_LIGNINDB_MEMORY_BUDGET_MB=16"),
            "{refused}"
        );
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        assert!(Drive::open(&path, Some(too_little)).is_err());
        assert!(!path.exists(), "the refused open made the directory");
    }

    /// The cache capacities LigninDB reports for the live database and for as many open
    /// checkpoints as the configuration counts are the budget together. It is the
    /// capacities that are summed, not what the caches hold at a moment, and a node that
    /// has more checkpoints open than the configuration counts is outside this test.
    #[test]
    fn the_capacities_of_the_live_cache_and_the_counted_checkpoints_are_the_budget() {
        if !the_engine_is_chosen_at_the_open() {
            return;
        }
        let config = DriveConfig {
            grovedb_lignindb_memory_budget_mb: 128,
            grovedb_lignindb_checkpoint_cache_mb: 16,
            open_checkpoints: 4,
            ..on(StorageEngine::LigninDb)
        };
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        let (drive, _) = Drive::open(&path, Some(config)).expect("a database");
        let live = drive.lignindb_stats().expect("LigninDB");
        assert_eq!(
            live.reserve.len,
            crate::config::DEFAULT_LIGNINDB_RESERVE_BYTES
        );
        let mut together = live.memory.capacity;
        assert_eq!(together, (128 - 4 * 16) << 20);

        std::fs::create_dir(path.join("checkpoints")).expect("the directory of checkpoints");
        let checkpoints: Vec<GroveDb> = (0..4)
            .map(|height| {
                let at = path.join("checkpoints").join(height.to_string());
                drive.grove.create_checkpoint(&at).expect("a checkpoint");
                drive.open_checkpoint(&at).expect("the checkpoint opened")
            })
            .collect();
        for checkpoint in &checkpoints {
            let stats = lignindb_stats_of(checkpoint).expect("LigninDB");
            assert_eq!(stats.reserve.len, 0, "a checkpoint keeps no reserve");
            together += stats.memory.capacity;
        }
        assert_eq!(together, 128 << 20);
    }

    /// At a start, a directory named after a height that holds no database is no
    /// checkpoint: it is not loaded, and no database is made in it.
    #[test]
    fn a_start_loads_the_checkpoints_and_makes_none() {
        for engine in engines() {
            let directory = tempfile::tempdir().expect("a directory");
            let path = directory.path().join("db");
            let checkpoints = path.join("checkpoints");
            {
                let (drive, _) = Drive::open(&path, Some(on(engine))).expect("a database");
                std::fs::create_dir(&checkpoints).expect("the directory of checkpoints");
                (drive.grove.create_checkpoint(checkpoints.join("5"))).expect("a checkpoint");
            }
            std::fs::create_dir(checkpoints.join("6")).expect("an empty directory");
            std::fs::write(checkpoints.join("7"), b"a file").expect("a file");

            let (drive, _) = Drive::open(&path, Some(on(engine))).expect("the database again");
            let loaded = drive.checkpoints.load();
            assert_eq!(
                loaded.keys().copied().collect::<Vec<_>>(),
                [5],
                "on {engine}"
            );
            assert_eq!(
                storage_engine_of(loaded[&5].checkpoint.grove_db()),
                engine,
                "a checkpoint is on the engine of its database"
            );
            assert!(
                contents(&checkpoints.join("6")).is_empty(),
                "the start wrote to the empty directory on {engine}"
            );
            assert_eq!(
                std::fs::read(checkpoints.join("7")).expect("the file"),
                b"a file"
            );
        }
    }

    /// At a start on LigninDB the engine removes what a creation or a deletion of a
    /// checkpoint that was cut short left: the directory a checkpoint was being built in,
    /// and a checkpoint whose deletion had begun. The first stands beside a checkpoint of
    /// its own height, which is loaded: a removal that named the checkpoint in place of
    /// the directory would take it. A directory that is neither is left as it was.
    #[test]
    fn a_start_on_lignindb_removes_what_an_interrupted_creation_or_deletion_left() {
        if !the_engine_is_chosen_at_the_open() {
            return;
        }
        let engine = StorageEngine::LigninDb;
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        let checkpoints = path.join("checkpoints");
        {
            let (drive, _) = Drive::open(&path, Some(on(engine))).expect("a database");
            std::fs::create_dir(&checkpoints).expect("the directory of checkpoints");
            for height in ["5", "7"] {
                (drive.grove.create_checkpoint(checkpoints.join(height))).expect("a checkpoint");
            }
        }
        let being_made = checkpoints.join(format!("5{CREATING_SUFFIX}"));
        std::fs::create_dir(&being_made).expect("the directory a checkpoint is built in");
        std::fs::write(being_made.join("part"), b"a part").expect("a file in it");
        let being_deleted = checkpoints.join("7");
        std::fs::rename(
            being_deleted.join(LIGNINDB_MANIFEST),
            being_deleted.join(DELETING_FILE_NAME),
        )
        .expect("the mark of a deletion that began");
        let neither = checkpoints.join("8");
        std::fs::create_dir(&neither).expect("a directory with no database");
        std::fs::write(neither.join("note"), b"a note").expect("a file in it");

        let (drive, _) = Drive::open(&path, Some(on(engine))).expect("the database again");
        let loaded = drive.checkpoints.load();
        assert_eq!(loaded.keys().copied().collect::<Vec<_>>(), [5]);
        assert!(
            !being_made.exists(),
            "what the creation left is still there"
        );
        assert!(
            !being_deleted.exists(),
            "what the deletion left is still there"
        );
        assert_eq!(
            std::fs::read(neither.join("note")).expect("the file"),
            b"a note"
        );
    }

    /// A checkpoint with its manifest in place, beside a file that has the name of a
    /// deletion's mark, is not removed at a start, whether that file is no manifest or a
    /// copy of the manifest. It is not opened either, and with the file gone the next
    /// start loads it.
    #[test]
    fn a_start_on_lignindb_keeps_a_checkpoint_whose_manifest_is_beside_a_deletions_mark() {
        if !the_engine_is_chosen_at_the_open() {
            return;
        }
        let engine = StorageEngine::LigninDb;
        for the_stray_file_is_a_manifest in [false, true] {
            let directory = tempfile::tempdir().expect("a directory");
            let path = directory.path().join("db");
            let checkpoint = path.join("checkpoints").join("7");
            {
                let (drive, _) = Drive::open(&path, Some(on(engine))).expect("a database");
                std::fs::create_dir(path.join("checkpoints"))
                    .expect("the directory of checkpoints");
                (drive.grove.create_checkpoint(&checkpoint)).expect("a checkpoint");
            }
            let stray = checkpoint.join(DELETING_FILE_NAME);
            let held = match the_stray_file_is_a_manifest {
                true => std::fs::read(checkpoint.join(LIGNINDB_MANIFEST)).expect("the manifest"),
                false => b"no manifest".to_vec(),
            };
            std::fs::write(&stray, held).expect("a file with the name of the mark");
            {
                let (drive, _) = Drive::open(&path, Some(on(engine))).expect("the database again");
                assert!(
                    drive.checkpoints.load().is_empty(),
                    "a manifest: {the_stray_file_is_a_manifest}"
                );
                assert!(
                    checkpoint.join(LIGNINDB_MANIFEST).is_file(),
                    "a manifest: {the_stray_file_is_a_manifest}"
                );
            }
            std::fs::remove_file(&stray).expect("remove the file");
            let (drive, _) =
                Drive::open(&path, Some(on(engine))).expect("the database a third time");
            let loaded = drive.checkpoints.load();
            assert_eq!(
                loaded.keys().copied().collect::<Vec<_>>(),
                [7],
                "a manifest: {the_stray_file_is_a_manifest}"
            );
        }
    }

    /// A symbolic link is not removed at a start, whatever its name and whatever it names:
    /// neither one with the name a checkpoint is built under, nor one named after a height
    /// to a directory that holds a deletion's mark.
    #[cfg(unix)]
    #[test]
    fn a_start_on_lignindb_leaves_a_symbolic_link_with_the_looks_of_a_leftover() {
        if !the_engine_is_chosen_at_the_open() {
            return;
        }
        let engine = StorageEngine::LigninDb;
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        let checkpoints = path.join("checkpoints");
        drop(Drive::open(&path, Some(on(engine))).expect("a database"));
        std::fs::create_dir(&checkpoints).expect("the directory of checkpoints");
        let built_elsewhere = directory.path().join("built");
        std::fs::create_dir(&built_elsewhere).expect("a directory");
        std::fs::write(built_elsewhere.join("part"), b"a part").expect("a file in it");
        let marked_elsewhere = directory.path().join("marked");
        std::fs::create_dir(&marked_elsewhere).expect("a directory");
        std::fs::write(marked_elsewhere.join(DELETING_FILE_NAME), b"no manifest")
            .expect("a file with the name of the mark");
        let links = [
            (
                checkpoints.join(format!("6{CREATING_SUFFIX}")),
                &built_elsewhere,
            ),
            (checkpoints.join("9"), &marked_elsewhere),
        ];
        for (link, named) in &links {
            std::os::unix::fs::symlink(named, link).expect("a symbolic link");
        }

        let (drive, _) = Drive::open(&path, Some(on(engine))).expect("the database again");
        assert!(drive.checkpoints.load().is_empty());
        for (link, _) in &links {
            let entry = std::fs::symlink_metadata(link).expect("the link");
            assert!(entry.file_type().is_symlink(), "{}", link.display());
        }
        assert_eq!(
            std::fs::read(built_elsewhere.join("part")).expect("the file"),
            b"a part"
        );
        assert_eq!(
            std::fs::read(marked_elsewhere.join(DELETING_FILE_NAME)).expect("the file"),
            b"no manifest"
        );
    }

    /// On RocksDB a start removes nothing: a directory with the name LigninDB builds a
    /// checkpoint under is left as it was.
    #[test]
    fn a_start_on_rocksdb_leaves_a_directory_with_the_name_of_lignindbs_staging() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("db");
        let being_made = path.join("checkpoints").join(format!("6{CREATING_SUFFIX}"));
        drop(Drive::open(&path, Some(on(StorageEngine::RocksDb))).expect("a database"));
        std::fs::create_dir_all(&being_made).expect("a directory");
        std::fs::write(being_made.join("part"), b"a part").expect("a file in it");

        drop(Drive::open(&path, Some(on(StorageEngine::RocksDb))).expect("the database again"));
        assert_eq!(
            std::fs::read(being_made.join("part")).expect("the file"),
            b"a part"
        );
    }

    #[test]
    fn a_checkpoint_is_not_made_where_there_is_none() {
        for engine in engines() {
            let directory = tempfile::tempdir().expect("a directory");
            let (drive, _) =
                Drive::open(directory.path().join("db"), Some(on(engine))).expect("a database");
            let missing = directory.path().join("7");
            assert!(drive.open_checkpoint(&missing).is_err());
            assert!(!missing.exists(), "the open made the directory on {engine}");

            let empty = directory.path().join("8");
            std::fs::create_dir(&empty).expect("an empty directory");
            assert!(drive.open_checkpoint(&empty).is_err());
            assert!(
                contents(&empty).is_empty(),
                "the open wrote to the directory on {engine}"
            );
        }
    }
}
