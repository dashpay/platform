//! The comparison of the stored records of two databases.
//!
//! No public read of GroveDB returns the list of the elements that refer to an element
//! which takes backward references, and no hash covers that list. What does hold it is the
//! record of the element as it is stored, and a dump of a subtree writes those records. So
//! the comparison walks both databases and holds the dump of every subtree of the one
//! against the dump of the same subtree of the other.

use super::guard::contained;
use super::report::StoreComparison;
use dpp::dashcore::hashes::{sha256, HashEngine};
use dpp::version::PlatformVersion;
use drive::grovedb::operations::dump_load::DumpLoadError;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{Element, GroveDb, PathQuery, Query, QueryItem, SizedQuery};
use std::io::{self, Write};
use std::time::Instant;

/// The differences a comparison keeps the text of.
const DIFFERENCES_KEPT: usize = 20;

/// Hashes what is written to it and counts the bytes.
struct Hashed {
    engine: sha256::HashEngine,
    bytes: u64,
}

impl Write for Hashed {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.engine.input(bytes);
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// What the dump of one subtree gave: the hash and the length of its stream, why the
/// subtree is not one that is dumped, or the error of the dump.
#[derive(Debug, PartialEq, Eq)]
enum Dumped {
    Stream { sha256: [u8; 32], bytes: u64 },
    Refused(&'static str),
    Failed(String),
}

/// What the dumps of one subtree from the two databases say of it.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Both gave the same stream, of this many bytes.
    Equal(u64),
    /// Both refuse the subtree for one reason: its records are not compared.
    NotDumped,
    /// The two differ: in their streams, or in what they answered where not both gave a
    /// stream and not both failed.
    Differs,
    /// Both failed: neither gave a stream, and nothing is known of the records.
    NotKnown,
}

fn verdict(this: &Dumped, other: &Dumped) -> Verdict {
    match (this, other) {
        (Dumped::Stream { bytes, .. }, Dumped::Stream { .. }) if this == other => {
            Verdict::Equal(*bytes)
        }
        (Dumped::Refused(_), Dumped::Refused(_)) if this == other => Verdict::NotDumped,
        (Dumped::Failed(_), Dumped::Failed(_)) => Verdict::NotKnown,
        _ => Verdict::Differs,
    }
}

fn dump(
    grove: &GroveDb,
    path: &[Vec<u8>],
    key: &[u8],
    platform_version: &PlatformVersion,
) -> Dumped {
    let mut hashed = Hashed {
        engine: sha256::Hash::engine(),
        bytes: 0,
    };
    match grove.dump_subtree(
        path,
        key,
        &mut hashed,
        None,
        &platform_version.drive.grove_version,
    ) {
        Ok(_) => Dumped::Stream {
            sha256: sha256::Hash::from_engine(hashed.engine).to_byte_array(),
            bytes: hashed.bytes,
        },
        Err(DumpLoadError::Refused(why)) => Dumped::Refused(why),
        Err(error) => Dumped::Failed(error.to_string()),
    }
}

/// Elements of one layer, with their keys.
type Layer = Vec<(Vec<u8>, Element)>;

/// The most elements that are read of a layer at once.
const PAGE: u16 = 1000;

/// The elements of the layer at `path` after the key `after`, or from its first, in the
/// order of their keys: [`PAGE`] of them at most.
fn page(
    grove: &GroveDb,
    path: &[Vec<u8>],
    after: Option<&[u8]>,
    platform_version: &PlatformVersion,
) -> Result<Layer, String> {
    let query = match after {
        Some(after) => {
            let mut query = Query::new();
            query.insert_item(QueryItem::RangeAfter(after.to_vec()..));
            query
        }
        None => Query::new_range_full(),
    };
    let query = PathQuery::new(path.to_vec(), SizedQuery::new(query, Some(PAGE), None));
    grove
        .query_raw(
            &query,
            true,
            false,
            true,
            QueryResultType::QueryKeyElementPairResultType,
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .map(|(found, _)| found.to_key_elements())
        .map_err(|error| error.to_string())
}

/// Gives `each` every page of the layer of `grove` at `path`, in order.
fn pages(
    grove: &GroveDb,
    path: &[Vec<u8>],
    platform_version: &PlatformVersion,
    mut each: impl FnMut(Layer) -> Result<(), String>,
) -> Result<(), String> {
    let mut after: Option<Vec<u8>> = None;
    loop {
        let layer = page(grove, path, after.as_deref(), platform_version)?;
        let whole = layer.len() == usize::from(PAGE);
        after = layer.last().map(|(key, _)| key.clone());
        each(layer)?;
        if !whole {
            return Ok(());
        }
    }
}

fn place(path: &[Vec<u8>], key: &[u8]) -> String {
    let path: Vec<String> = path.iter().map(hex::encode).collect();
    format!("/{} key {}", path.join("/"), hex::encode(key))
}

/// Compares the stored records of `primary` and `shadow`, which are at one height: the
/// dump of every subtree that is reached from the root through the trees of the primary,
/// and the elements of the root as a raw query returns them.
///
/// A subtree that a dump refuses on both databases for the same reason is counted and its
/// records are not compared: the trees below it are still walked, and a comparison that
/// counts any did not cover every record. A dump that one database alone refuses or
/// fails, and one the two refuse for different reasons, is a difference. The walk ends
/// with an error where a layer of the primary is not read, where a dump fails on both
/// databases (two dumps that were not made show no difference), and where it panics.
/// What it found before an error is in what it returns. Neither database is written to.
///
/// It holds the paths it has still to walk, and of the elements a page at a time.
pub fn compare_stores(
    primary: &GroveDb,
    shadow: &GroveDb,
    height: u64,
    platform_version: &PlatformVersion,
) -> StoreComparison {
    let began = Instant::now();
    let mut comparison = StoreComparison {
        height,
        ..Default::default()
    };
    // A panic of the walk ends it as an error does: what it found before is kept.
    let walked = contained(|| walk(primary, shadow, platform_version, &mut comparison))
        .and_then(|walked| walked);
    if let Err(error) = walked {
        comparison.error = Some(error);
    }
    comparison.seconds = began.elapsed().as_secs();
    comparison
}

fn differs(comparison: &mut StoreComparison, difference: String) {
    comparison.differing += 1;
    if comparison.differences.len() < DIFFERENCES_KEPT {
        comparison.differences.push(difference);
    }
}

/// The elements of the root of the two databases, a page of each at a time.
fn roots(
    primary: &GroveDb,
    shadow: &GroveDb,
    platform_version: &PlatformVersion,
    comparison: &mut StoreComparison,
) -> Result<(), String> {
    let mut after: Option<Vec<u8>> = None;
    loop {
        let of_primary = page(primary, &[], after.as_deref(), platform_version)?;
        let of_shadow = page(shadow, &[], after.as_deref(), platform_version)?;
        comparison.root_elements_compared += of_primary.len() as u64;
        if of_primary != of_shadow {
            differs(
                comparison,
                format!(
                    "the elements of the root {}: this has {}, other has {}, and they are \
                     not equal",
                    match &after {
                        None => "from the first".to_string(),
                        Some(key) => format!("after key {}", hex::encode(key)),
                    },
                    of_primary.len(),
                    of_shadow.len()
                ),
            );
            return Ok(());
        }
        if of_primary.len() < usize::from(PAGE) {
            return Ok(());
        }
        after = of_primary.last().map(|(key, _)| key.clone());
    }
}

fn walk(
    primary: &GroveDb,
    shadow: &GroveDb,
    platform_version: &PlatformVersion,
    comparison: &mut StoreComparison,
) -> Result<(), String> {
    roots(primary, shadow, platform_version, comparison)?;
    let mut paths: Vec<Vec<Vec<u8>>> = vec![Vec::new()];
    while let Some(path) = paths.pop() {
        pages(primary, &path, platform_version, |layer| {
            for (key, element) in layer {
                if !element.is_any_tree() {
                    continue;
                }
                let this = dump(primary, &path, &key, platform_version);
                let other = dump(shadow, &path, &key, platform_version);
                match verdict(&this, &other) {
                    Verdict::Equal(bytes) => {
                        comparison.subtrees_compared += 1;
                        comparison.bytes += bytes;
                    }
                    Verdict::NotDumped => comparison.subtrees_not_dumped += 1,
                    Verdict::Differs => differs(
                        comparison,
                        format!(
                            "the subtree at {}: this={this:?} other={other:?}",
                            place(&path, &key)
                        ),
                    ),
                    Verdict::NotKnown => {
                        return Err(format!(
                            "the subtree at {} was dumped by neither database: this={this:?} \
                             other={other:?}",
                            place(&path, &key)
                        ))
                    }
                }
                if !element.uses_non_merk_data_storage() {
                    let mut below = path.clone();
                    below.push(key);
                    paths.push(below);
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_dumps_differ_only_where_one_of_them_was_made() {
        let stream = |hash: u8, bytes: u64| Dumped::Stream {
            sha256: [hash; 32],
            bytes,
        };
        let failed = |why: &str| Dumped::Failed(why.to_string());
        assert_eq!(verdict(&stream(1, 10), &stream(1, 10)), Verdict::Equal(10));
        assert_eq!(verdict(&stream(1, 10), &stream(2, 10)), Verdict::Differs);
        assert_eq!(verdict(&stream(1, 10), &stream(1, 11)), Verdict::Differs);
        assert_eq!(
            verdict(&Dumped::Refused("a"), &Dumped::Refused("a")),
            Verdict::NotDumped
        );
        assert_eq!(
            verdict(&Dumped::Refused("a"), &Dumped::Refused("b")),
            Verdict::Differs
        );
        for one in [Dumped::Refused("a"), failed("gone")] {
            assert_eq!(verdict(&stream(1, 10), &one), Verdict::Differs);
            assert_eq!(verdict(&one, &stream(1, 10)), Verdict::Differs);
        }
        assert_eq!(
            verdict(&Dumped::Refused("a"), &failed("gone")),
            Verdict::Differs
        );
        // Neither was made: with one error or with two.
        assert_eq!(verdict(&failed("gone"), &failed("gone")), Verdict::NotKnown);
        assert_eq!(verdict(&failed("gone"), &failed("lost")), Verdict::NotKnown);
    }
}
