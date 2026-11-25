//! The block transaction is rewound to a savepoint in two places only: a genesis re-proposal
//! (`init_chain` sets the savepoint, and each proposal round at the genesis height rewinds to
//! it), and a proposer dropping from the block it builds a transition that ended as an
//! internal error or an unpaid consensus error (`process_raw_state_transitions`).
//!
//! Nothing else rewinds it. In particular, a write that validation admitted and Drive cannot
//! apply stays an internal error: it is not rolled back and executed again as a paid refusal.
//! The check that would have refused it belongs in validation (see "Where a check belongs" in
//! `book/src/contributing/coding-conventions.md`).
//!
//! A new entry in the list below, or a higher count on one, is a new place the block
//! transaction can be rewound: it needs the review a consensus change gets, not just this list
//! edited.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// The call that sets a savepoint on a transaction.
const SET_SAVEPOINT: &str = "set_savepoint(";
/// The call that rewinds a transaction to its last savepoint.
const ROLLBACK_TO_SAVEPOINT: &str = "rollback_to_savepoint(";

/// The sources of `drive` and `drive-abci` that name those calls, from `packages/`, each with
/// how many times it names the first and the second, comments included.
const SOURCES_USING_SAVEPOINTS: [(&str, usize, usize); 6] = [
    // The genesis re-proposal.
    ("rs-drive-abci/src/abci/handler/init_chain.rs", 1, 0),
    ("rs-drive-abci/src/abci/handler/prepare_proposal.rs", 1, 1),
    ("rs-drive-abci/src/abci/handler/process_proposal.rs", 1, 1),
    // The proposer's rollback of a transition it drops: one call of each, and a comment.
    (
        "rs-drive-abci/src/execution/platform_events/state_transition_processing/process_raw_state_transitions/v0/mod.rs",
        1,
        2,
    ),
    // A test of that rollback, and the test harness that plays the ABCI handlers' part.
    (
        "rs-drive-abci/src/execution/validation/state_transition/state_transitions/shield/tests.rs",
        2,
        3,
    ),
    ("rs-drive-abci/src/mimic/mod.rs", 1, 1),
];

fn collect_sources_using_savepoints(
    directory: &Path,
    packages: &Path,
    found: &mut BTreeMap<String, (usize, usize)>,
) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("expected to read {}: {error}", directory.display()));
    for entry in entries {
        let path = entry.expect("expected a directory entry").path();
        if path.is_dir() {
            collect_sources_using_savepoints(&path, packages, found);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("expected to read {}: {error}", path.display()));
        let calls = (
            source.matches(SET_SAVEPOINT).count(),
            source.matches(ROLLBACK_TO_SAVEPOINT).count(),
        );
        if calls != (0, 0) {
            let relative = path
                .strip_prefix(packages)
                .expect("expected a path under packages");
            found.insert(relative.to_string_lossy().replace('\\', "/"), calls);
        }
    }
}

#[test]
fn should_rewind_the_block_transaction_only_where_listed() {
    let packages = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("expected the packages directory");
    let mut found = BTreeMap::new();
    for crate_sources in ["rs-drive/src", "rs-drive-abci/src"] {
        collect_sources_using_savepoints(&packages.join(crate_sources), packages, &mut found);
    }
    let listed: BTreeMap<String, (usize, usize)> = SOURCES_USING_SAVEPOINTS
        .iter()
        .map(|(source, set, rollback)| (source.to_string(), (*set, *rollback)))
        .collect();
    assert_eq!(
        found, listed,
        "the sources that set or rewind to a savepoint, or how often they do, differ from the \
         list: a write Drive cannot apply is never rolled back and run again as a refusal, its \
         check goes in validation"
    );
}
