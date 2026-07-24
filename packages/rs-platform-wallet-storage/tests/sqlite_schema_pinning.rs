#![allow(clippy::field_reassign_with_default)]

//! Content-level schema-freeze guards.
//!
//! TC-B-040: pin the rendered migration SQL with a golden fingerprint so an
//! in-place DDL edit (which the identity-only fingerprint is documented not
//! to catch) breaks CI. TC-B-041: assert the retired cross-branch table
//! names never appear as SQL identifiers in the writer/reader/migration/
//! backup SQL — the drift the content-blind fingerprint cannot catch.

use std::path::Path;

use platform_wallet_storage::sqlite::migrations as mig;

/// Golden `(version, name)` fingerprint of the frozen migration set. Bump
/// deliberately only when adding/removing/renaming a migration file.
const EXPECTED_ID_FINGERPRINT: &str =
    "93e6ce49f81f9c40031cf98feb44e508bcf7e354f3e332f0989f39e2b5c8ef4f";

/// Golden content-level fingerprint over every migration's rendered SQL.
/// Bump deliberately only when the DDL body itself changes; an accidental
/// change (a silent table rename) must fail this test, not slip through.
const EXPECTED_SQL_FINGERPRINT: &str =
    "290bfe0c233e6d594e08e963cb7f27411dd18c2830a8fa64047135bbd5b35ca6";

/// Table names that lost the cross-branch reconciliation and must never
/// resurface as SQL identifiers on this frozen (`wallets`) baseline.
const RETIRED_SQL_NAMES: &[&str] = &[
    "wallet_metadata",
    "account_address_pools",
    "core_derived_addresses",
];

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
        "a migration's DDL body changed. On this frozen baseline that is a \
         schema-drift alarm (D0). If intentional, update EXPECTED_SQL_FINGERPRINT."
    );
}

/// The retired names appear nowhere in the rendered migration SQL.
#[test]
fn tc_b_041_migration_sql_has_no_retired_names() {
    for sql in mig::embedded_migrations_sql() {
        for name in RETIRED_SQL_NAMES {
            assert!(
                !sql.contains(name),
                "retired table name `{name}` present in migration SQL"
            );
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
