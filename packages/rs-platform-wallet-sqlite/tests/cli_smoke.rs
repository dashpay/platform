#![allow(clippy::field_reassign_with_default)]

//! TC-056..TC-075 — CLI smoke tests.

use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;

fn cli() -> Command {
    Command::cargo_bin("platform-wallet-sqlite").expect("bin built")
}

/// TC-056: migrate on a fresh DB prints `applied: <N>` then `applied: 0`.
#[test]
fn tc056_migrate_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    let out = cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .unwrap();
    assert!(out.status.success(), "first migrate failed: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("applied: ") && stdout.trim() != "applied: 0",
        "unexpected first-run stdout: {stdout}"
    );
    let out2 = cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .unwrap();
    assert!(out2.status.success(), "second migrate failed");
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert_eq!(stdout2.trim(), "applied: 0");
}

/// TC-062: restore without --yes refuses (exit 2).
#[test]
fn tc062_restore_without_yes_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .expect("migrate ran");
    let fake_src = tmp.path().join("not-a-backup.db");
    std::fs::write(&fake_src, b"x").unwrap();
    let out = cli()
        .args([
            "--db",
            db.to_str().unwrap(),
            "restore",
            "--from",
            fake_src.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "expected exit 2; got {:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// TC-065: prune without --keep-last or --max-age is a usage error.
#[test]
fn tc065_prune_requires_a_rule() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    let dir = tmp.path().join("bk");
    std::fs::create_dir(&dir).unwrap();
    let out = cli()
        .args([
            "--db",
            db.to_str().unwrap(),
            "prune",
            "--in",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// TC-070: invalid wallet-id format exits 2.
#[test]
fn tc070_inspect_invalid_wallet_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .expect("migrate ran");
    for bad in ["zzzz", "00"] {
        let out = cli()
            .args(["--db", db.to_str().unwrap(), "inspect", "--wallet-id", bad])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "expected exit 2 for `{bad}`; got {:?}",
            out.status.code()
        );
    }
}

/// TC-072: delete-wallet without --yes exits 2.
#[test]
fn tc072_delete_wallet_without_yes_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .expect("migrate ran");
    let out = cli()
        .args([
            "--db",
            db.to_str().unwrap(),
            "delete-wallet",
            "--wallet-id",
            &"aa".repeat(32),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// TC-068: inspect TSV format prints `table\tcount` lines.
#[test]
fn tc068_inspect_tsv() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .expect("migrate ran");
    let out = cli()
        .args(["--db", db.to_str().unwrap(), "inspect", "--format", "tsv"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 18,
        "expected ≥18 lines of TSV, got {}",
        lines.len()
    );
    for line in lines {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 2, "bad TSV line: `{line}`");
        let n: i64 = cols[1].parse().expect(line);
        assert!(n >= 0);
    }
}

/// TC-059: backup --out <dir> writes a timestamped file.
#[test]
fn tc059_backup_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("w.db");
    cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .expect("migrate ran");
    let out_dir = tmp.path().join("bk");
    std::fs::create_dir(&out_dir).unwrap();
    let out = cli()
        .args([
            "--db",
            db.to_str().unwrap(),
            "backup",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "backup failed: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let path = stdout.trim();
    assert!(path.ends_with(".db"));
    assert!(std::path::Path::new(path).exists());
}
