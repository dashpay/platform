#![allow(clippy::field_reassign_with_default)]

//! Content-level schema-freeze guards.
//!
//! TC-B-040: pin the rendered migration SQL with a golden fingerprint so an
//! in-place DDL edit (which the identity-only fingerprint is documented not
//! to catch) breaks CI. TC-B-041: assert the retired cross-branch table
//! names never appear as SQL identifiers in the writer/reader/migration/
//! backup SQL — the drift the content-blind fingerprint cannot catch.

use std::path::Path;

use platform_wallet_storage::sqlite::{migrations as mig, schema::versions::Domain};

/// Golden `(version, name)` fingerprint of the frozen migration set. Bump
/// deliberately only when adding/removing/renaming a migration file.
const EXPECTED_ID_FINGERPRINT: &str =
    "0ef38f22126957b909c3672c29073447d92caa51044a3727834c4e6e41d30908";

/// Golden content-level fingerprint over every migration's rendered SQL.
/// Bump it only when ADDING a migration file; a body change on an already
/// applied migration is a defect, not a golden to refresh.
const EXPECTED_SQL_FINGERPRINT: &str =
    "220433359df1d1b59267fd2486447416682e3d58c0d980f5ac119302bb72f33b";

/// The migrations merged `v4.2-dev` already ships. Refinery keys
/// `refinery_schema_history` by version and validates an applied migration's
/// checksum against the embedded migration of the SAME version, so pointing one
/// of these versions at different DDL stops every database that applied the
/// original from opening. New work appends after the highest entry here.
const MERGED_MIGRATION_VERSIONS: &[(i32, &str)] = &[
    (1, "initial"),
    (2, "address_height_pin"),
    (3, "invitations"),
    (4, "asset_lock_recovered_status"),
    (5, "dpns_name_states"),
    (6, "tracked_masternodes"),
];

/// Table names retired by `V007__rehydration_base_schema`. They are part of
/// the migration history up to and including V007 — V001-V006 are byte-frozen
/// published migrations that legitimately name them — so the guards below
/// scope to what comes AFTER the rename, plus all writer/reader SQL.
const FIRST_VERSION_AFTER_RENAME: i32 = 8;

/// Migration files whose SQL may legitimately name a retired table: the
/// published base set and the migration that performs the rename.
const PRE_RENAME_MIGRATION_FILES: &[&str] = &[
    "V001__initial.rs",
    "V002__address_height_pin.rs",
    "V003__invitations.rs",
    "V004__asset_lock_recovered_status.rs",
    "V005__dpns_name_states.rs",
    "V006__tracked_masternodes.rs",
    "V007__rehydration_base_schema.rs",
];

const RETIRED_SQL_NAMES: &[&str] = &[
    "wallet_metadata",
    "account_address_pools",
    "core_derived_addresses",
];

#[test]
fn domain_labels_are_live_sql_names() {
    for domain in Domain::ALL {
        assert!(
            !RETIRED_SQL_NAMES.contains(&domain.as_str()),
            "Domain::{domain:?} uses retired SQL name `{}`",
            domain.as_str()
        );
    }
}

/// A version already merged to a base branch keeps the name it shipped with.
///
/// IF THIS FAILS: a migration file was renumbered onto a version some other
/// branch already published. Give the new work the next free version instead —
/// reusing a published one is not a naming preference, it is a database that
/// stops opening.
#[test]
fn merged_migration_versions_keep_their_shipped_names() {
    let embedded = mig::embedded_migrations();
    for (version, name) in MERGED_MIGRATION_VERSIONS {
        let found = embedded
            .iter()
            .find(|(v, _)| v == version)
            .unwrap_or_else(|| panic!("migration version {version} is missing from the set"));
        assert_eq!(
            found.1.as_str(),
            *name,
            "version {version} must stay `{name}`; it is owned by merged history"
        );
    }
}

/// TC-B-040 (identity) — the migration set's identity is pinned.
#[test]
fn tc_b_040_identity_fingerprint_pinned() {
    assert_eq!(
        hex::encode(mig::embedded_migrations_fingerprint()),
        EXPECTED_ID_FINGERPRINT,
        "migration set identity changed; a file was added/removed/renamed. \
         If intentional, update EXPECTED_ID_FINGERPRINT."
    );
}

/// TC-B-040 (content) — the rendered migration SQL is pinned, closing the
/// content-blind gap the identity fingerprint documents.
#[test]
fn tc_b_040_sql_fingerprint_pinned() {
    assert_eq!(
        hex::encode(mig::embedded_migrations_sql_fingerprint()),
        EXPECTED_SQL_FINGERPRINT,
        "a migration's DDL body changed. Refinery checksums rendered SQL, so \
         editing a migration that any database has already applied stops that \
         database opening, permanently. Widen a schema by APPENDING a migration. \
         Update EXPECTED_SQL_FINGERPRINT only when adding a migration file."
    );
}

/// The retired names appear nowhere as table identifiers in migration SQL.
#[test]
fn tc_b_041_migration_sql_has_no_retired_names() {
    for (version, sql) in mig::embedded_migrations_sql_by_version() {
        if version < FIRST_VERSION_AFTER_RENAME {
            continue;
        }
        for name in RETIRED_SQL_NAMES {
            for keyword in ["FROM", "INTO", "UPDATE", "TABLE", "JOIN", "ON"] {
                assert!(
                    !sql.contains(&format!("{keyword} {name}")),
                    "retired table name `{name}` present in migration SQL"
                );
            }
        }
    }
}

/// TC-B-041 — no writer/reader/migration/backup SQL string references a
/// retired table name. `wallet_metadata` / `account_address_pools` are also
/// legitimate Rust changeset fields, so the scan flags only SQL-keyword-led
/// table usage (`FROM`/`INTO`/`UPDATE`/`TABLE`/`JOIN`/`ON <name>`), never a
/// bare `cs.<field>` access.
#[test]
fn tc_b_041_no_retired_table_name_in_sql_strings() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let sql_keywords = ["FROM", "INTO", "UPDATE", "TABLE", "JOIN", "ON"];

    let mut offenders = Vec::new();
    for dir in [src, migrations_dir] {
        visit(&dir, &mut |path, line_no, line| {
            let file = path.file_name().unwrap_or_default().to_string_lossy();
            if PRE_RENAME_MIGRATION_FILES.contains(&file.as_ref()) {
                return;
            }
            for name in RETIRED_SQL_NAMES {
                for kw in sql_keywords {
                    if line.contains(&format!("{kw} {name}")) {
                        offenders.push(format!("{}:{line_no}: {}", path.display(), line.trim()));
                    }
                }
            }
        });
    }
    assert!(
        offenders.is_empty(),
        "retired table name used in SQL: {offenders:#?}"
    );
}

fn visit(dir: &Path, on_line: &mut impl FnMut(&Path, usize, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            visit(&p, on_line);
        } else if p.extension().is_some_and(|e| e == "rs") {
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            for (i, line) in text.lines().enumerate() {
                on_line(&p, i + 1, line);
            }
        }
    }
}
