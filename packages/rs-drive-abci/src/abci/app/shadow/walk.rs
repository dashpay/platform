//! When and where the stored records of the two databases are compared.
//!
//! A comparison needs both databases at one height, which they are between two blocks. So
//! the thread that executes blocks makes a checkpoint of each after a block, and a thread
//! of its own walks the two checkpoints ([`compare_stores`]) and removes them.
//!
//! Each checkpoint is made inside the directory of the database it is of, under
//! [`DIRECTORY`], so that it is on the file system of that database and is made of links.
//! That directory is shadow mode's own, and shadow mode does not start where the path of
//! the report, of the shadow's database or of the primary's is inside it or leads through
//! it, of either database.
//! Making the two checkpoints is the part that the thread that executes blocks waits for:
//! a flush of each database, the links, and a write of the status file. A comparison that found a difference leaves its
//! checkpoints, marked as kept; what any other left behind, also at a stop of the node, is
//! removed at the next start. A panic where a comparison begins or on its thread is caught
//! and counted as a comparison that failed.

use super::guard::{contained, Latches};
use super::report::{Report, StoreComparison};
use super::store::compare_stores;
use super::{Difference, DivergenceKind, Shadow, ShadowApplication};
use crate::abci::app::PlatformApplication;
use crate::config::OnDivergence;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::config::{DriveConfig, StorageEngine};
use drive::drive::{Checkpoint, Drive};

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// The directory, inside the directory of a database, that holds the checkpoints of the
/// comparisons of stored records.
pub const DIRECTORY: &str = "shadow-compare";

/// The end of the name of the file that marks a checkpoint as kept: the evidence of a
/// comparison that found a difference.
const KEPT: &str = ".kept";

fn checkpoint_at(database: &Path, height: u64) -> PathBuf {
    database.join(DIRECTORY).join(height.to_string())
}

fn mark_of(checkpoint: &Path) -> PathBuf {
    let mut name = checkpoint.as_os_str().to_os_string();
    name.push(KEPT);
    PathBuf::from(name)
}

fn remove(checkpoint: &Path, engine: StorageEngine) {
    if let Err(error) = Checkpoint::delete_closed(checkpoint, engine) {
        tracing::warn!(?error, path = %checkpoint.display(), "a checkpoint of a comparison of stored records stays");
    }
}

/// Removes what comparisons of stored records left in the directory [`DIRECTORY`] of the
/// database at `database`, which is on `engine`: everything in it but a checkpoint that
/// is marked as kept and the marks. The directory is shadow mode's own: a checkpoint that
/// was being made when the node stopped is in it under another name than a height.
///
/// Nothing is removed where [`DIRECTORY`] is not a directory of its own but a symbolic
/// link, and a symbolic link in it is removed as a link, with what it leads to left.
pub(super) fn remove_what_a_comparison_left(database: &Path, engine: StorageEngine) {
    let own = database.join(DIRECTORY);
    match std::fs::symlink_metadata(&own) {
        Ok(metadata) if metadata.is_dir() => (),
        Ok(_) => {
            tracing::warn!(
                path = %own.display(),
                "the directory of the comparisons of stored records is no directory of its \
                 own: nothing in it is removed"
            );
            return;
        }
        Err(_) => return,
    }
    let Ok(entries) = std::fs::read_dir(&own) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_a_mark = entry.file_name().to_string_lossy().ends_with(KEPT);
        if is_a_mark || mark_of(&path).exists() {
            continue;
        }
        // The kind of the entry itself: a link is not followed.
        let is_a_directory = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_a_directory {
            remove(&path, engine);
            // What is no checkpoint of the engine's is removed as a directory.
            if path.exists() {
                let _ = std::fs::remove_dir_all(&path);
            }
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// The symbolic links that are followed on the way of one path. The system follows 40.
const LINKS_FOLLOWED: usize = 40;

/// The way the file system goes for a path, when this is asked.
struct Way {
    /// The locations the way goes into by a name, in order: a directory the path goes
    /// through, a symbolic link on it where the link is, and the same of what a link
    /// leads to. The root is not among them, nor a location the way comes back to by a
    /// `..`.
    passed: Vec<PathBuf>,
    /// Where the way ends.
    end: PathBuf,
}

/// The way of a path. `.` is taken out, and a `..` is taken from the location that was
/// reached before it. A name that is not there is taken as a directory that is not there
/// yet. No more than [`LINKS_FOLLOWED`] links are followed: the way ends at the next one.
fn way_of(path: &Path) -> Way {
    use std::path::Component;
    enum Step {
        From(PathBuf),
        Up,
        Into(std::ffi::OsString),
    }
    let steps = |path: &Path| -> Vec<Step> {
        path.components()
            .filter_map(|component| match component {
                Component::Prefix(_) | Component::RootDir => {
                    Some(Step::From(PathBuf::from(component.as_os_str())))
                }
                Component::CurDir => None,
                Component::ParentDir => Some(Step::Up),
                Component::Normal(name) => Some(Step::Into(name.to_os_string())),
            })
            .collect()
    };
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut ahead: std::collections::VecDeque<Step> = steps(&absolute).into();
    let mut end = PathBuf::new();
    let mut passed = Vec::new();
    let mut links = 0;
    while let Some(step) = ahead.pop_front() {
        match step {
            // The root, and on a system that has them the prefix before it.
            Step::From(root) => end.push(root),
            Step::Up => {
                end.pop();
            }
            Step::Into(name) => {
                end.push(name);
                passed.push(end.clone());
                if let Ok(target) = std::fs::read_link(&end) {
                    if links == LINKS_FOLLOWED {
                        break;
                    }
                    links += 1;
                    // What the link leads to is gone through from where the link is.
                    end.pop();
                    for step in steps(&target).into_iter().rev() {
                        ahead.push_front(step);
                    }
                }
            }
        }
    }
    Way { passed, end }
}

/// Why shadow mode does not start with these three directories: the path of one of them
/// is inside a directory that a start empties, [`DIRECTORY`] of the primary's database
/// or of the shadow's, or leads through it. `None` where none is.
///
/// A directory that a start empties is taken two ways: [`DIRECTORY`] in the location
/// where the way of its database ends, and the location where the way of that ends,
/// which is another where the name is a link. Of each of the three paths, each
/// location its way goes into and the one where it ends are held against those
/// ([`Way`]), the links on it and what each leads to included.
pub(super) fn inside_what_a_start_empties(
    primary_database: &Path,
    shadow_database: &Path,
    report: &Path,
) -> Option<String> {
    let emptied: Vec<PathBuf> = [primary_database, shadow_database]
        .iter()
        .flat_map(|database| {
            let own = way_of(database).end.join(DIRECTORY);
            let leads_to = way_of(&own).end;
            [own, leads_to]
        })
        .collect();
    [
        ("DB_PATH", primary_database),
        ("SHADOW_DB_PATH", shadow_database),
        ("SHADOW_REPORT_PATH", report),
    ]
    .into_iter()
    .find_map(|(setting, path)| {
        let way = way_of(path);
        emptied
            .iter()
            .find(|emptied| {
                way.passed
                    .iter()
                    .chain([&way.end])
                    .any(|at| at.starts_with(emptied))
            })
            .map(|emptied| {
                format!(
                    "{setting} is {}, which {} {}, which holds the checkpoints of the \
                     comparisons of stored records and is emptied at a start",
                    path.display(),
                    if way.end.starts_with(emptied) {
                        "is inside"
                    } else {
                        "leads through"
                    },
                    emptied.display()
                )
            })
    })
}

impl<'a, C: CoreRPCLike> Shadow<'a, C> {
    /// The name of a file in the report's directory that asks for a comparison of the
    /// stored records, whatever the interval. It is looked for at the end of a
    /// `FinalizeBlock` that counted its block as compared, as not compared in full, or as
    /// diverged in its costs alone, and there only where the interval has not passed.
    /// Where a comparison is to begin there, for the file or for the interval, the file
    /// system is first asked to remove the file; a comparison that then does not begin
    /// is counted as failed. None begins while one is running, and the file is left as
    /// it is.
    pub const COMPARE_NOW_FILE: &'static str = "compare-stores-now";

    /// Begins a comparison of the stored records where one is due or asked for and none
    /// is running: a checkpoint of each database, which both are at `height`, and a
    /// thread that walks the two checkpoints and removes them. The node waits for the
    /// checkpoints and not for the walk.
    pub(super) fn compare_the_stores_when_due<A>(
        &self,
        primary: &A,
        app: &ShadowApplication<'a, C>,
        height: u64,
    ) where
        A: PlatformApplication<C>,
    {
        let asked = self.report.directory().join(Self::COMPARE_NOW_FILE);
        let mut last = self
            .last_store_comparison
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let due = self
            .store_compare_interval
            .is_some_and(|interval| last.elapsed() >= interval);
        if !(due || asked.exists()) || self.report.counts().store_comparisons.running {
            return;
        }
        *last = Instant::now();
        drop(last);
        let _ = std::fs::remove_file(&asked);
        if let Err(error) = self.begin_store_comparison(primary, app, height) {
            // Not begun for an error or for a panic, which is caught where it happens:
            // counted, and nothing of it is running.
            tracing::error!(
                height,
                error,
                "a comparison of the stored records did not begin"
            );
            self.written(self.report.change(|counts| {
                counts.store_comparisons.failed += 1;
                counts.store_comparisons.running = false;
            }));
        }
    }

    fn begin_store_comparison<A>(
        &self,
        primary: &A,
        app: &ShadowApplication<'a, C>,
        height: u64,
    ) -> Result<(), String>
    where
        A: PlatformApplication<C>,
    {
        let primary_drive = &primary.platform().drive;
        let shadow_drive = &app.platform.drive;
        let walk = StoreWalk {
            height,
            primary: (
                checkpoint_at(&primary.platform().config.db_path, height),
                primary_drive.config.clone(),
            ),
            shadow: (
                checkpoint_at(&app.platform.config.db_path, height),
                shadow_drive.config.clone(),
            ),
            platform_version: primary
                .platform()
                .state
                .load()
                .current_platform_version()
                .map_err(|error| error.to_string())?,
            report: Arc::clone(&self.report),
            latches: Arc::clone(&self.latches),
            on_divergence: self.on_divergence,
            #[cfg(any(test, feature = "mocks"))]
            gate: self
                .gate
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        };
        // A checkpoint that is there already is the kept one of an earlier comparison at
        // this height, on databases that were at it before: it is left as it is.
        for (path, _) in [&walk.primary, &walk.shadow] {
            if path.exists() || mark_of(path).exists() {
                return Err(format!(
                    "{} is there already, kept by an earlier comparison at this height",
                    path.display()
                ));
            }
        }
        // Whatever fails from here on, by an error or by a panic, what was made is removed.
        let begun = contained(|| {
            for ((path, _), drive) in [(&walk.primary, primary_drive), (&walk.shadow, shadow_drive)]
            {
                let parent = path.parent().ok_or("a checkpoint with no directory")?;
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                drive
                    .grove
                    .create_checkpoint(path)
                    .map_err(|error| error.to_string())?;
            }
            self.report
                .change(|counts| counts.store_comparisons.running = true)
                .map_err(|error| error.to_string())
        })
        .and_then(|begun| begun);
        if let Err(error) = begun {
            walk.remove_the_checkpoints();
            return Err(error);
        }
        // The thread owns the walk from here. Where it does not start, the walk is gone
        // with its closure, and its checkpoints are removed by their paths.
        let made = [&walk.primary, &walk.shadow]
            .map(|(path, config)| (path.clone(), config.grovedb_storage_engine));
        std::thread::Builder::new()
            .name("shadow-store-compare".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || walk.run_contained())
            .map(|_| ())
            .map_err(|error| {
                for (path, engine) in &made {
                    remove(path, *engine);
                }
                error.to_string()
            })
    }
}

/// One comparison of the stored records, on a thread of its own.
pub(super) struct StoreWalk {
    pub(super) height: u64,
    pub(super) primary: (PathBuf, DriveConfig),
    pub(super) shadow: (PathBuf, DriveConfig),
    pub(super) platform_version: &'static PlatformVersion,
    pub(super) report: Arc<Report>,
    pub(super) latches: Arc<Latches>,
    pub(super) on_divergence: OnDivergence,
    #[cfg(any(test, feature = "mocks"))]
    pub(super) gate: Option<Arc<super::testing::Gate>>,
}

impl StoreWalk {
    fn remove_the_checkpoints(&self) {
        for (path, config) in [&self.primary, &self.shadow] {
            remove(path, config.grovedb_storage_engine);
        }
    }

    fn compared(&self) -> StoreComparison {
        let open = |(path, config): &(PathBuf, DriveConfig)| {
            Drive::open_checkpoint_at(path, config).map_err(|error| error.to_string())
        };
        let opened = open(&self.primary).and_then(|primary| Ok((primary, open(&self.shadow)?)));
        match opened {
            Ok((primary, shadow)) => {
                compare_stores(&primary, &shadow, self.height, self.platform_version)
            }
            Err(error) => StoreComparison {
                height: self.height,
                error: Some(error),
                ..Default::default()
            },
        }
    }

    /// What the thread of a comparison runs: [`StoreWalk::run`], with a panic of it caught
    /// and counted as a comparison that failed, so that it is no panic of the node.
    fn run_contained(self) {
        let (report, height) = (Arc::clone(&self.report), self.height);
        #[cfg(any(test, feature = "mocks"))]
        let gate = self.gate.clone();
        #[cfg(any(test, feature = "mocks"))]
        if let Some(gate) = &gate {
            if !gate.wait_until_open() {
                gate.done();
                return;
            }
        }
        if let Err(panic) = contained(|| self.run()) {
            tracing::error!(height, panic, "a comparison of the stored records failed");
            let written = report.change(|counts| {
                counts.store_comparisons.running = false;
                counts.store_comparisons.failed += 1;
            });
            if let Err(error) = written {
                tracing::error!(?error, "the report of shadow mode was not written");
            }
        }
        #[cfg(any(test, feature = "mocks"))]
        if let Some(gate) = &gate {
            gate.done();
        }
    }

    /// Walks the two checkpoints and has what it found written down, as far as the file
    /// system takes each write. A difference is a divergence: its checkpoints are left,
    /// with a mark beside each where that was written, the shadow applies no block that
    /// begins after this thread has stopped it, and the node ends where it is configured
    /// to. The write of the comparison, with the stop that is to come in it, is made
    /// before the shadow is stopped: where it was taken, a node that stops in between
    /// starts with the shadow stopped. A walk that an error ended early and that found
    /// no difference is counted as failed, and is no divergence.
    pub(super) fn run(self) {
        let comparison = match contained(|| self.compared()) {
            Ok(comparison) => comparison,
            Err(panic) => StoreComparison {
                height: self.height,
                error: Some(panic),
                ..Default::default()
            },
        };
        let written = |written: std::io::Result<()>| {
            if let Err(error) = written {
                tracing::error!(?error, "the report of shadow mode was not written");
            }
        };
        let differs = comparison.differing > 0;
        let failed = comparison.error.is_some();
        if differs {
            for (path, _) in [&self.primary, &self.shadow] {
                written(std::fs::write(mark_of(path), b""));
            }
            match self.report.divergence(self.height, DivergenceKind::Store) {
                Ok(kept) => {
                    let differences = vec![Difference {
                        height: self.height,
                        field: "stored_records",
                        this: format!("\"{}\"", self.primary.0.display()),
                        other: format!("\"{}\"", self.shadow.0.display()),
                    }];
                    written(kept.differences(&differences, &comparison.differences));
                    written(kept.file(
                        "stores.json",
                        &serde_json::to_vec_pretty(&comparison).expect("a comparison is JSON"),
                    ));
                }
                Err(error) => tracing::error!(?error, "a divergence was not kept"),
            }
        } else {
            self.remove_the_checkpoints();
        }
        tracing::info!(
            height = self.height,
            subtrees = comparison.subtrees_compared,
            not_dumped = comparison.subtrees_not_dumped,
            differing = comparison.differing,
            error = comparison.error.as_deref(),
            seconds = comparison.seconds,
            "the stored records of the two databases were compared"
        );
        let height = self.height;
        let reason = format!("the stored records of the two databases differ at height {height}");
        written(self.report.change(|counts| {
            let comparisons = &mut counts.store_comparisons;
            comparisons.running = false;
            match failed {
                true => comparisons.failed += 1,
                false => comparisons.made += 1,
            }
            comparisons.differing += u64::from(differs);
            comparisons.last = Some(comparison);
            if differs && counts.shadow_stopped.is_none() {
                counts.shadow_stop_pending = Some(reason.clone());
            }
        }));
        if differs {
            self.latches.stop_from_the_next_block(reason);
            if self.on_divergence == OnDivergence::Stop {
                self.latches.end(height);
            }
        }
    }
}
