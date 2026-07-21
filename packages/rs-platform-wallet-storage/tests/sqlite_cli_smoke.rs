#![allow(clippy::field_reassign_with_default)]

//! CLI smoke tests for the maintenance binary.

mod common;

use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;

fn cli() -> Command {
    Command::cargo_bin("platform-wallet-storage").expect("bin built")
}

/// migrate on a fresh DB prints `applied: <N>` then `applied: 0`.
#[test]
fn tc056_migrate_idempotent() {
    let tmp = common::secure_tempdir().unwrap();
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

/// restore without --yes refuses (exit 2).
#[test]
fn tc062_restore_without_yes_refuses() {
    let tmp = common::secure_tempdir().unwrap();
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

/// prune without --keep-last or --max-age is a usage error.
#[test]
fn tc065_prune_requires_a_rule() {
    let tmp = common::secure_tempdir().unwrap();
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

/// The `inspect` subcommand is removed from the CLI; invoking it is an
/// unknown-subcommand usage error.
#[test]
fn inspect_subcommand_removed() {
    let tmp = common::secure_tempdir().unwrap();
    let db = tmp.path().join("w.db");
    cli()
        .args(["--db", db.to_str().unwrap(), "migrate"])
        .output()
        .expect("migrate ran");
    let out = cli()
        .args(["--db", db.to_str().unwrap(), "inspect"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "expected clap usage exit code 2 for removed subcommand; got {:?}",
        out.status.code()
    );
}

/// The `delete-wallet` subcommand is not part of the CLI; invoking it is
/// an unknown-subcommand usage error.
#[test]
fn tc072_delete_wallet_subcommand_removed() {
    let tmp = common::secure_tempdir().unwrap();
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
    assert_eq!(
        out.status.code(),
        Some(2),
        "expected clap usage exit code 2 for removed subcommand; got {:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unrecognized") || stderr.contains("unexpected"),
        "expected clap unknown-subcommand error, got stderr: `{stderr}`"
    );
}

/// backup --out <dir> writes a timestamped file.
#[test]
fn tc059_backup_dir() {
    let tmp = common::secure_tempdir().unwrap();
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

/// 1a: the supported `--no-auto-backup` flag disables the
/// pre-migration auto-backup. `migrate --no-auto-backup` succeeds on a
/// fresh DB without writing the `backups/auto/` sentinel snapshot.
#[test]
fn tc_code_030_1a_no_auto_backup_disables() {
    let tmp = common::secure_tempdir().unwrap();
    let db = tmp.path().join("w.db");
    let out = cli()
        .args(["--db", db.to_str().unwrap(), "migrate", "--no-auto-backup"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "migrate --no-auto-backup failed: {out:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("auto-backup skipped (--no-auto-backup)"),
        "expected `--no-auto-backup` notice on stderr, got: {stderr}"
    );
    // No `backups/auto/pre-migration-*.db` written when the flag is set.
    let auto_dir = tmp.path().join("backups").join("auto");
    if auto_dir.exists() {
        let pre_mig: Vec<_> = std::fs::read_dir(&auto_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("pre-migration"))
            .collect();
        assert!(
            pre_mig.is_empty(),
            "pre-migration backup written despite --no-auto-backup: {pre_mig:?}"
        );
    }
}
