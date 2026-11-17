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
//! A new entry in the list below is a new place the block transaction can be rewound: it needs
//! the review a consensus change gets, not just this list edited.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The calls that set a savepoint on a transaction or rewind to one.
const SAVEPOINT_CALLS: [&str; 2] = ["set_savepoint(", "rollback_to_savepoint("];

/// The sources of `drive` and `drive-abci` that name those calls, from `packages/`.
const SOURCES_USING_SAVEPOINTS: [&str; 6] = [
    // The genesis re-proposal.
    "rs-drive-abci/src/abci/handler/init_chain.rs",
    "rs-drive-abci/src/abci/handler/prepare_proposal.rs",
    "rs-drive-abci/src/abci/handler/process_proposal.rs",
    // The proposer's rollback of a transition it drops.
    "rs-drive-abci/src/execution/platform_events/state_transition_processing/process_raw_state_transitions/v0/mod.rs",
    // A test of that rollback, and the test harness that plays the ABCI handlers' part.
    "rs-drive-abci/src/execution/validation/state_transition/state_transitions/shield/tests.rs",
    "rs-drive-abci/src/mimic/mod.rs",
];

fn collect_sources_using_savepoints(
    directory: &Path,
    packages: &Path,
    found: &mut BTreeSet<String>,
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
        if SAVEPOINT_CALLS.iter().any(|call| source.contains(call)) {
            let relative = path
                .strip_prefix(packages)
                .expect("expected a path under packages");
            found.insert(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn should_rewind_the_block_transaction_only_where_listed() {
    let packages = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("expected the packages directory");
    let mut found = BTreeSet::new();
    for crate_sources in ["rs-drive/src", "rs-drive-abci/src"] {
        collect_sources_using_savepoints(&packages.join(crate_sources), packages, &mut found);
    }
    let listed: BTreeSet<String> = SOURCES_USING_SAVEPOINTS
        .iter()
        .map(|source| source.to_string())
        .collect();
    assert_eq!(
        found, listed,
        "the sources that set or rewind to a savepoint differ from the list: a write Drive \
         cannot apply is never rolled back and run again as a refusal, its check goes in \
         validation"
    );
}
