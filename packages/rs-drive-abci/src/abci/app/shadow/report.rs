//! What shadow mode writes: its counts, a line for a block, and what it keeps of a
//! divergence.

use super::compare::{BlockLine, Difference, DivergenceKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The name of the file the counts are kept in.
pub const STATUS_FILE: &str = "status.json";

const BLOCKS_IN_A_FILE: u64 = 100_000;

const GAUGE_BLOCKS_COMPARED: &str = "shadow_blocks_compared";
const GAUGE_DIVERGENCES: &str = "shadow_divergences";
const GAUGE_HEIGHT: &str = "shadow_height";
const GAUGE_STOPPED: &str = "shadow_stopped";
const LABEL_KIND: &str = "kind";

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// A height the shadow held already when the primary was given its block again after a
/// start, so that the shadow did not execute it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedHeight {
    /// The height.
    pub height: u64,
    /// Whether the app hash the primary committed at it was held against the shadow's:
    /// it is at the shadow's own height, the only one whose app hash the shadow has.
    pub app_hash_compared: bool,
}

/// Blocks that were counted, when the node committed them, as not compared in full: the
/// reason says what was not compared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotCompared {
    /// The first height.
    pub height: u64,
    /// The last height: the first where it is one block.
    pub through: u64,
    /// Why.
    pub reason: String,
}

/// A run of heights.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeightRange {
    /// The first height.
    pub height: u64,
    /// The last height.
    pub through: u64,
}

/// A start at which the primary's database was below the height of the counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rewind {
    /// The height of the counts.
    pub counted: u64,
    /// The height of the primary's database.
    pub held: u64,
}

/// Where and why the shadow stopped applying blocks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stopped {
    /// The first height the shadow did not apply, as far as it is known: the reason says
    /// where it is not known whether the shadow's database holds the block at this
    /// height. A stop that a comparison of stored records asked for and that a start
    /// wrote down is at the height after the higher of two: the primary's, as its
    /// database has it at that start, and the shadow's, as the counts have it.
    pub height: u64,
    /// Why.
    pub reason: String,
}

/// One comparison of the stored records of the two databases.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreComparison {
    /// The height both databases were at.
    pub height: u64,
    /// The subtrees whose dumps from the two databases are equal.
    pub subtrees_compared: u64,
    /// The subtrees a dump refuses on both databases for one reason, whose records were
    /// not compared. A comparison that has any did not cover every record.
    pub subtrees_not_dumped: u64,
    /// The elements of the root of the primary, which were held against the shadow's as a
    /// raw query returns them.
    pub root_elements_compared: u64,
    /// The bytes of the dumps of one database that were equal to the other's.
    pub bytes: u64,
    /// The places the two databases differ in, the first ones.
    pub differences: Vec<String>,
    /// The number of places they differ in.
    pub differing: u64,
    /// An error that ended the comparison early.
    pub error: Option<String>,
    /// The seconds it took.
    pub seconds: u64,
}

/// The counts of the comparisons of stored records.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreComparisons {
    /// Comparisons that ran to their end.
    pub made: u64,
    /// Comparisons that found a difference, whether they ran to their end or not.
    pub differing: u64,
    /// Comparisons that did not begin or that an error ended early.
    pub failed: u64,
    /// Whether one is running.
    pub running: bool,
    /// The last one.
    pub last: Option<StoreComparison>,
}

/// The counts of shadow mode, kept across starts in [`STATUS_FILE`].
///
/// Every block the node commits in shadow mode is counted once, when it is committed: in
/// `blocks_compared`, `blocks_diverged`, `heights_skipped`, `blocks_not_compared` or
/// `blocks_without_shadow`. A block that was committed and not counted when the node
/// stopped is in none of them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Counts {
    /// The storage engine of the primary.
    pub primary_engine: String,
    /// The storage engine of the shadow.
    pub shadow_engine: String,
    /// When the report was first opened, in seconds since 1970.
    pub since: u64,
    /// When the report was last opened, at a start of the node, in seconds since 1970.
    pub started: u64,
    /// When the counts were last written, in seconds since 1970.
    pub updated: u64,
    /// The number of starts.
    pub starts: u64,
    /// The height of the primary: that of the last block that was counted, or the one its
    /// database was at when the node last started.
    pub height: u64,
    /// The height of the shadow as far as it is known: read from its database at a start
    /// that opens it, and written when it commits a block.
    pub shadow_height: u64,
    /// The first height that is counted in `blocks_compared`.
    pub first_compared_height: Option<u64>,
    /// The blocks both engines executed, whose executions were held against each other
    /// before the commits and whose roots after them, and that differed in nothing.
    pub blocks_compared: u64,
    /// The blocks that were counted as diverged when the primary had committed them: the
    /// shadow was given the block in that `FinalizeBlock`, and a divergence was found in
    /// it before the block was counted, before or after the commits, a failure of the
    /// shadow included.
    pub blocks_diverged: u64,
    /// The heights the shadow skipped after a start.
    pub heights_skipped: Vec<SkippedHeight>,
    /// The blocks that were counted, when the node committed them, as not compared in
    /// full, each with what was not compared: nothing that was compared of the block
    /// differed.
    pub blocks_not_compared: Vec<NotCompared>,
    /// The blocks the primary committed after the shadow stopped.
    pub blocks_without_shadow: u64,
    /// The divergences, by kind. A comparison of stored records that differs is one,
    /// with the height of its checkpoints in `divergence_heights`.
    pub divergences: BTreeMap<String, u64>,
    /// The heights of the divergences.
    pub divergence_heights: Vec<u64>,
    /// Where the shadow stopped applying blocks, if it did.
    pub shadow_stopped: Option<Stopped>,
    /// Why the shadow is to stop, said by a comparison of stored records and not yet
    /// written down as a stop with the height of the first block the shadow did not
    /// apply: the shadow applies no block that begins after the comparison's thread has
    /// stopped it, which it does once this has been given to be written. A start that
    /// finds it starts with the shadow stopped.
    pub shadow_stop_pending: Option<String>,
    /// The first height that was counted when the primary committed it.
    pub first_counted_height: Option<u64>,
    /// What starts saw: the heights between the height of the counts and the one the
    /// primary's database was at, where that was above. A start does not know how the
    /// database came by them. Writing a range down adds nothing to a count of blocks,
    /// and says nothing of whether a height in it was counted before.
    pub blocks_uncounted: Vec<HeightRange>,
    /// What starts saw: the primary's database below the height of the counts. Blocks it
    /// commits after such a start are counted when they are committed, whether or not
    /// they were counted before.
    pub rewinds: Vec<Rewind>,
    /// The heights of the divergences that were found, at a block or at `InitChain`,
    /// while an error of Dash Core was noted: one it answered a question of the shadow
    /// with since the shadow last began to execute a block, or since the node started
    /// where it began none. The errors are written with what is kept of such a
    /// divergence, as far as that is written. A difference in stored records is not
    /// among them.
    pub divergences_with_core_errors: Vec<u64>,
    /// The comparisons of stored records.
    pub store_comparisons: StoreComparisons,
    /// For the last block that both committed with nothing found after the commits and
    /// whose execution by the primary the comparison held: the milliseconds of the
    /// primary's handler that executed it (`ProcessProposal`, or `PrepareProposal` on the
    /// node that proposed it) and of the one that finalized it.
    pub last_primary_ms: u64,
    /// For the same block: the milliseconds from where the shadow began to execute it to
    /// where its database was flushed, with the comparison of the two executions in
    /// between.
    pub last_shadow_ms: u64,
}

impl Counts {
    /// Counts a divergence of `kind` at `height`.
    pub(super) fn divergence(&mut self, height: u64, kind: DivergenceKind) {
        *self.divergences.entry(kind.name().to_string()).or_default() += 1;
        self.divergence_heights.push(height);
    }

    /// The number of divergences of every kind.
    pub fn divergences_in_all(&self) -> u64 {
        self.divergences.values().sum()
    }

    /// The number of blocks in `blocks_not_compared`.
    pub fn blocks_not_compared_in_all(&self) -> u64 {
        self.blocks_not_compared
            .iter()
            .map(|not_compared| not_compared.through - not_compared.height + 1)
            .sum()
    }

    /// The number of heights in `blocks_uncounted`.
    pub fn blocks_uncounted_in_all(&self) -> u64 {
        self.blocks_uncounted
            .iter()
            .map(|range| range.through - range.height + 1)
            .sum()
    }

    /// The number of blocks that were counted when the node committed them in shadow
    /// mode, since the counts began: a block that was committed twice and counted twice
    /// is in it twice. A range in `blocks_uncounted` adds nothing to it.
    pub fn blocks_in_all(&self) -> u64 {
        self.blocks_compared
            + self.blocks_diverged
            + self.heights_skipped.len() as u64
            + self.blocks_not_compared_in_all()
            + self.blocks_without_shadow
    }
}

/// The directory shadow mode writes to.
#[derive(Debug)]
pub struct Report {
    directory: PathBuf,
    counts: Mutex<Counts>,
}

fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut beside = path.as_os_str().to_os_string();
    beside.push(".new");
    let beside = PathBuf::from(beside);
    fs::write(&beside, bytes)?;
    fs::rename(&beside, path)
}

impl Report {
    /// Opens the report in `directory`, which is made where it is not there, and goes on
    /// from the counts it holds. Counts of another pair of engines are refused.
    pub fn open(directory: &Path, primary_engine: &str, shadow_engine: &str) -> io::Result<Self> {
        Self::open_with(directory, primary_engine, shadow_engine, |_| ())
    }

    /// The same, with `begin` changing the counts of this start before they are written:
    /// the start is one write, and a node that stops in it leaves the counts of the start
    /// before or those of this one.
    pub(super) fn open_with(
        directory: &Path,
        primary_engine: &str,
        shadow_engine: &str,
        begin: impl FnOnce(&mut Counts),
    ) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let mut counts = Self::counts_in(directory)?.unwrap_or_else(|| Counts {
            primary_engine: primary_engine.to_string(),
            shadow_engine: shadow_engine.to_string(),
            since: now(),
            ..Default::default()
        });
        if counts.primary_engine != primary_engine || counts.shadow_engine != shadow_engine {
            return Err(io::Error::other(format!(
                "the counts of shadow mode in {} are of {} with {} as shadow, and this node is \
                 {primary_engine} with {shadow_engine} as shadow",
                directory.display(),
                counts.primary_engine,
                counts.shadow_engine
            )));
        }
        counts.started = now();
        counts.starts += 1;
        counts.store_comparisons.running = false;
        let report = Self {
            directory: directory.to_path_buf(),
            counts: Mutex::new(counts),
        };
        report.change(begin)?;
        Ok(report)
    }

    /// The counts a report in `directory` holds, read and nothing written: `None` where
    /// it holds none.
    pub(super) fn counts_in(directory: &Path) -> io::Result<Option<Counts>> {
        match fs::read(directory.join(STATUS_FILE)) {
            Ok(bytes) => serde_json::from_slice::<Counts>(&bytes)
                .map(Some)
                .map_err(|error| {
                    io::Error::other(format!(
                        "the counts of shadow mode in {} are not read: {error}",
                        directory.join(STATUS_FILE).display()
                    ))
                }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Where the counts in `directory` say the shadow stopped, or is to stop, without
    /// opening the report or writing anything: `None` where there are no counts or they
    /// say neither. A stop that is not yet written down with its height is given with the
    /// height after the higher of the two the counts have, the primary's and the
    /// shadow's; the start that writes it down goes by the primary's database.
    pub fn stopped_in(directory: &Path) -> io::Result<Option<Stopped>> {
        Ok(Self::counts_in(directory)?.and_then(|counts| {
            let pending = counts.shadow_stop_pending.map(|reason| Stopped {
                height: counts.height.max(counts.shadow_height) + 1,
                reason,
            });
            counts.shadow_stopped.or(pending)
        }))
    }

    /// The engines the counts in `directory` are of, the primary's and the shadow's,
    /// without opening the report: `None` where there are no counts.
    pub fn engines_in(directory: &Path) -> io::Result<Option<(String, String)>> {
        Ok(Self::counts_in(directory)?.map(|counts| (counts.primary_engine, counts.shadow_engine)))
    }

    /// The directory.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The counts as they are.
    pub fn counts(&self) -> Counts {
        self.counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Changes the counts and writes them.
    pub(super) fn change(&self, change: impl FnOnce(&mut Counts)) -> io::Result<()> {
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        change(&mut counts);
        counts.updated = now();
        metrics::gauge!(GAUGE_BLOCKS_COMPARED).set(counts.blocks_compared as f64);
        metrics::gauge!(GAUGE_HEIGHT).set(counts.shadow_height as f64);
        metrics::gauge!(GAUGE_STOPPED).set(u8::from(counts.shadow_stopped.is_some()));
        for (kind, count) in &counts.divergences {
            metrics::gauge!(GAUGE_DIVERGENCES, LABEL_KIND => kind.clone()).set(*count as f64);
        }
        let bytes = serde_json::to_vec_pretty(&*counts).expect("counts are JSON");
        write_whole(&self.directory.join(STATUS_FILE), &bytes)
    }

    /// Writes the line of a block: what the primary made of a block the node committed
    /// while the shadow applied blocks, where the comparison held the primary's execution
    /// of it.
    pub(super) fn block(&self, line: &BlockLine) -> io::Result<()> {
        let first = line.height / BLOCKS_IN_A_FILE * BLOCKS_IN_A_FILE;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.join(format!("blocks-{first:09}.jsonl")))?;
        let mut text = line.json();
        text.push('\n');
        file.write_all(text.as_bytes())
    }

    /// Counts a divergence of `kind` at `height` and, where the counts were written,
    /// makes the directory for what is kept of it. A second divergence at a height gets
    /// a directory of its own.
    #[cfg_attr(not(feature = "shadow-store-compare"), allow(dead_code))]
    pub(super) fn divergence(&self, height: u64, kind: DivergenceKind) -> io::Result<Kept> {
        self.change(|counts| counts.divergence(height, kind))?;
        self.kept_at(height)
    }

    /// A directory for what is kept of a divergence at `height`, with nothing counted.
    pub(super) fn kept_at(&self, height: u64) -> io::Result<Kept> {
        let divergences = self.directory.join("divergences");
        let mut directory = divergences.join(height.to_string());
        let mut again = 1;
        while directory.exists() {
            again += 1;
            directory = divergences.join(format!("{height}-{again}"));
        }
        fs::create_dir_all(&directory)?;
        Ok(Kept { directory })
    }
}

/// The directory that keeps what a divergence was found in.
#[derive(Debug)]
pub struct Kept {
    directory: PathBuf,
}

impl Kept {
    /// The directory.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Writes a file into it.
    pub(super) fn file(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        fs::write(self.directory.join(name), bytes)
    }

    /// Removes the directory with what it holds: what was kept was no divergence.
    pub(super) fn discard(self) {
        if let Err(error) = fs::remove_dir_all(&self.directory) {
            tracing::warn!(?error, path = %self.directory.display(), "what was kept of no divergence stays");
        }
    }

    /// Writes the differences into it, a line each, and logs them.
    pub(super) fn differences(
        &self,
        differences: &[Difference],
        more: &[String],
    ) -> io::Result<()> {
        let mut text = String::new();
        for difference in differences {
            tracing::error!("{difference}");
            text.push_str(&difference.to_string());
            text.push('\n');
        }
        for line in more {
            tracing::error!("{line}");
            text.push_str(line);
            text.push('\n');
        }
        self.file("differs.txt", text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenderdash_abci::proto::abci as proto;

    fn line(height: u64) -> BlockLine {
        let request = proto::RequestFinalizeBlock {
            height: height as i64,
            ..Default::default()
        };
        BlockLine::executed(&request, &Default::default(), Some(&[]))
    }

    /// Every block is in one of the counts, and a run of blocks that were not compared
    /// counts as many as it holds.
    #[test]
    fn the_blocks_in_all_are_the_sum_of_what_became_of_each() {
        let counts = Counts {
            blocks_compared: 100,
            blocks_diverged: 2,
            heights_skipped: vec![SkippedHeight {
                height: 7,
                app_hash_compared: true,
            }],
            blocks_not_compared: vec![
                NotCompared {
                    height: 20,
                    through: 20,
                    reason: "one".to_string(),
                },
                NotCompared {
                    height: 31,
                    through: 34,
                    reason: "four".to_string(),
                },
            ],
            blocks_without_shadow: 10,
            ..Default::default()
        };
        assert_eq!(counts.blocks_not_compared_in_all(), 5);
        assert_eq!(counts.blocks_in_all(), 118);
        let counts = Counts {
            blocks_uncounted: vec![
                HeightRange {
                    height: 40,
                    through: 40,
                },
                HeightRange {
                    height: 90,
                    through: 92,
                },
            ],
            ..counts
        };
        assert_eq!(counts.blocks_uncounted_in_all(), 4);
        // Heights that a start found uncounted are in no count of blocks.
        assert_eq!(counts.blocks_in_all(), 118);
        assert_eq!(Counts::default().blocks_in_all(), 0);
    }

    /// What a start reads of a report before it opens anything: nothing where there is no
    /// report, the stop and the engines where there is one, and an error where the counts
    /// are not read. Reading writes nothing.
    #[test]
    fn the_stop_and_the_engines_are_read_without_opening_the_report() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("report");
        assert_eq!(Report::stopped_in(&path).expect("read"), None);
        assert_eq!(Report::engines_in(&path).expect("read"), None);
        assert!(!path.exists());

        let report = Report::open(&path, "lignindb", "rocksdb").expect("a report");
        assert_eq!(Report::stopped_in(&path).expect("read"), None);
        assert_eq!(
            Report::engines_in(&path).expect("read"),
            Some(("lignindb".to_string(), "rocksdb".to_string()))
        );
        let stopped = Stopped {
            height: 12,
            reason: "a reason".to_string(),
        };
        report
            .change(|counts| counts.shadow_stopped = Some(stopped.clone()))
            .expect("written");
        let before = std::fs::read(path.join(STATUS_FILE)).expect("the file");
        assert_eq!(Report::stopped_in(&path).expect("read"), Some(stopped));
        assert_eq!(
            std::fs::read(path.join(STATUS_FILE)).expect("the file"),
            before
        );

        // A stop that is asked for and not written down with its height is a stop too.
        report
            .change(|counts| {
                counts.shadow_stopped = None;
                counts.shadow_stop_pending = Some("asked".to_string());
                counts.height = 20;
            })
            .expect("written");
        assert_eq!(
            Report::stopped_in(&path).expect("read"),
            Some(Stopped {
                height: 21,
                reason: "asked".to_string()
            })
        );
        // The shadow's height, where the counts have it above the primary's.
        report
            .change(|counts| counts.shadow_height = 21)
            .expect("written");
        assert_eq!(
            Report::stopped_in(&path)
                .expect("read")
                .map(|stopped| stopped.height),
            Some(22)
        );

        std::fs::write(path.join(STATUS_FILE), b"not json").expect("written");
        for refused in [
            Report::stopped_in(&path).map(|_| ()),
            Report::engines_in(&path).map(|_| ()),
        ] {
            let refused = refused.expect_err("not read");
            assert!(refused.to_string().contains("are not read"), "{refused}");
        }
    }

    #[test]
    fn the_counts_go_on_from_one_start_to_the_next() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("report");
        let report = Report::open(&path, "rocksdb", "lignindb").expect("a report");
        assert_eq!(report.directory(), path);
        let first = report.counts();
        assert_eq!((first.starts, first.blocks_compared), (1, 0));
        assert_eq!(
            (first.primary_engine.as_str(), first.shadow_engine.as_str()),
            ("rocksdb", "lignindb")
        );
        assert!(first.since > 0 && first.started >= first.since && first.updated >= first.started);
        report
            .change(|counts| {
                counts.blocks_compared = 5;
                counts.store_comparisons.running = true;
            })
            .expect("written");
        let on_disk: Counts =
            serde_json::from_slice(&std::fs::read(path.join(STATUS_FILE)).expect("the file"))
                .expect("counts");
        assert_eq!(on_disk, report.counts());
        assert!(!path.join(format!("{STATUS_FILE}.new")).exists());
        drop(report);

        let again = Report::open(&path, "rocksdb", "lignindb").expect("the report again");
        let second = again.counts();
        assert_eq!((second.starts, second.blocks_compared), (2, 5));
        assert_eq!(second.since, first.since);
        // A comparison that a stopped process left running is not running.
        assert!(!second.store_comparisons.running);
    }

    #[test]
    fn counts_of_another_pair_of_engines_and_counts_that_are_not_read_are_refused() {
        let directory = tempfile::tempdir().expect("a directory");
        drop(Report::open(directory.path(), "rocksdb", "lignindb").expect("a report"));
        let refused = Report::open(directory.path(), "lignindb", "rocksdb").expect_err("refused");
        assert!(
            refused.to_string().contains(
                "are of rocksdb with lignindb as shadow, and this node is lignindb with rocksdb \
                 as shadow"
            ),
            "{refused}"
        );
        std::fs::write(directory.path().join(STATUS_FILE), b"not json").expect("written");
        let refused = Report::open(directory.path(), "rocksdb", "lignindb").expect_err("refused");
        assert!(refused.to_string().contains("are not read"), "{refused}");
        // A status file that cannot be read at all, here a directory, is the error of the read.
        std::fs::remove_file(directory.path().join(STATUS_FILE)).expect("removed");
        std::fs::create_dir(directory.path().join(STATUS_FILE)).expect("a directory");
        assert!(Report::open(directory.path(), "rocksdb", "lignindb").is_err());
    }

    #[test]
    fn the_lines_of_blocks_go_to_files_of_a_hundred_thousand_heights() {
        let directory = tempfile::tempdir().expect("a directory");
        let report = Report::open(directory.path(), "rocksdb", "lignindb").expect("a report");
        for height in [99_999, 100_000, 100_001, 734_700] {
            report.block(&line(height)).expect("written");
        }
        let lines = |name: &str| {
            std::fs::read_to_string(directory.path().join(name))
                .expect(name)
                .lines()
                .map(|line| line.to_string())
                .collect::<Vec<String>>()
        };
        assert_eq!(lines("blocks-000000000.jsonl"), vec![line(99_999).json()]);
        assert_eq!(
            lines("blocks-000100000.jsonl"),
            vec![line(100_000).json(), line(100_001).json()]
        );
        assert_eq!(lines("blocks-000700000.jsonl"), vec![line(734_700).json()]);
    }

    #[test]
    fn a_divergence_is_counted_by_its_kind_and_kept_in_a_directory_of_its_own() {
        let directory = tempfile::tempdir().expect("a directory");
        let report = Report::open(directory.path(), "rocksdb", "lignindb").expect("a report");
        let first = report.divergence(7, DivergenceKind::Cost).expect("kept");
        let second = report.divergence(7, DivergenceKind::State).expect("kept");
        let third = report.divergence(9, DivergenceKind::Cost).expect("kept");
        assert_eq!(
            first.directory(),
            directory.path().join("divergences").join("7")
        );
        assert_eq!(
            second.directory(),
            directory.path().join("divergences").join("7-2")
        );
        assert_eq!(
            third.directory(),
            directory.path().join("divergences").join("9")
        );
        let counts = report.counts();
        assert_eq!(
            counts.divergences,
            BTreeMap::from([("cost".to_string(), 2), ("state".to_string(), 1)])
        );
        assert_eq!(counts.divergences_in_all(), 3);
        assert_eq!(counts.divergence_heights, vec![7, 7, 9]);

        first.file("a.bin", b"abc").expect("written");
        assert_eq!(
            std::fs::read(first.directory().join("a.bin")).expect("read"),
            b"abc"
        );
        let difference = Difference {
            height: 7,
            field: "fee_ops",
            this: "1".to_string(),
            other: "2".to_string(),
        };
        first
            .differences(&[difference], &["more".to_string()])
            .expect("written");
        assert_eq!(
            std::fs::read_to_string(first.directory().join("differs.txt")).expect("read"),
            "DIFFERS height=7 field=fee_ops this=1 other=2\nmore\n"
        );
    }

    #[test]
    fn a_report_whose_directory_is_gone_gives_the_error_of_the_write() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("report");
        let report = Report::open(&path, "rocksdb", "lignindb").expect("a report");
        std::fs::remove_dir_all(&path).expect("removed");
        assert!(report.change(|counts| counts.blocks_compared = 9).is_err());
        assert!(report.block(&line(1)).is_err());
        assert!(report.divergence(4, DivergenceKind::State).is_err());
        // What was counted is in the counts although it was not written: the change that
        // failed, and the divergence that was not kept.
        let counts = report.counts();
        assert_eq!((counts.starts, counts.blocks_compared), (1, 9));
        assert_eq!(counts.divergences_in_all(), 1);
        // With the directory there again, the next write holds all of it.
        std::fs::create_dir_all(&path).expect("a directory");
        report.change(|_| ()).expect("written");
        let on_disk: Counts =
            serde_json::from_slice(&std::fs::read(path.join(STATUS_FILE)).expect("the file"))
                .expect("counts");
        assert_eq!(on_disk, report.counts());
        assert_eq!(on_disk.blocks_compared, 9);
    }
}
