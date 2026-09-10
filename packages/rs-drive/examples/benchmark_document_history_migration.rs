//! Measures activation on an owned copy of an offline committed Drive database.
use drive::drive::Drive;
use platform_version::version::PlatformVersion;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn copy_directory(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir(target)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if kind.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), destination)?;
        } else {
            return Err("snapshot contains a symlink or special file".into());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(
            "usage: benchmark_document_history_migration COMMITTED_SNAPSHOT NEW_WORKING_COPY"
                .into(),
        );
    }
    let source = fs::canonicalize(&arguments[0])?;
    let target = Path::new(&arguments[1]);
    if target.exists() {
        return Err("working-copy destination must not exist".into());
    }
    let parent = fs::canonicalize(
        target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    )?;
    if parent.starts_with(&source) {
        return Err("working copy must be outside the source snapshot".into());
    }
    copy_directory(&source, target)?;
    let (drive, _) = Drive::open(target, None)?;
    let version = PlatformVersion::get(14)?;
    let root_before = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value?;
    let transaction = drive.grove.start_transaction();
    let start = Instant::now();
    let stats = drive.migrate_document_history_storage(&transaction, version)?;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    let root_after = drive
        .grove
        .root_hash(Some(&transaction), &version.drive.grove_version)
        .value?;
    drop(transaction);
    assert_eq!(
        drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value?,
        root_before
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "contracts": stats.contracts,
            "types": stats.types,
            "documents": stats.documents,
            "revisions": stats.revisions,
            "revision_payload_bytes": stats.revision_bytes,
            "maximum_document_revisions": stats.maximum_document_revisions,
            "index_entries": stats.index_entries,
            "migrated_documents": stats.migrated_documents,
            "batches": stats.batches,
            "duration_ms": duration_ms,
            "seek_count": stats.cost.seek_count,
            "added_bytes": stats.cost.storage_cost.added_bytes,
            "replaced_bytes": stats.cost.storage_cost.replaced_bytes,
            "loaded_bytes": stats.cost.storage_loaded_bytes,
            "hash_node_calls": stats.cost.hash_node_calls,
            "root_before": hex::encode(root_before),
            "root_after": hex::encode(root_after),
            "transaction_rolled_back": true,
        }))?
    );
    Ok(())
}
