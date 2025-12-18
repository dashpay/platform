use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::DefaultCoreRPC;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use std::fs::remove_file;
use std::path::PathBuf;

/// Run all verification steps:
/// - GroveDB integrity
/// - platform state app hash vs GroveDB root
/// - configuration consistency with stored protocol version
pub fn run(config: &PlatformConfig, force: bool) -> Result<(), String> {
    verify_grovedb(&config.db_path, force)?;

    let (drive, maybe_platform_version) =
        Drive::open(&config.db_path, Some(config.drive.clone())).map_err(|e| e.to_string())?;

    let platform_version = maybe_platform_version.unwrap_or_else(PlatformVersion::latest);

    let platform_state =
        Platform::<DefaultCoreRPC>::fetch_platform_state(&drive, None, platform_version)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "platform state missing from database".to_string())?;

    verify_app_hash_matches_grove_root(&drive, &platform_state, platform_version)?;
    verify_config_matches_db(config, &drive)?;

    Ok(())
}

/// Verify GroveDB integrity.
///
/// This function will execute GroveDB integrity checks if one of the following conditions is met:
/// - `force` is `true`
/// - file `.fsck` in `db_path` exists
///
/// After successful verification, .fsck file is removed.
pub fn verify_grovedb(db_path: &PathBuf, force: bool) -> Result<(), String> {
    let fsck = PathBuf::from(db_path).join(".fsck");

    if !force {
        if !fsck.exists() {
            return Ok(());
        }
        tracing::info!(
            "found {} file, starting grovedb verification",
            fsck.display()
        );
    }

    let grovedb = drive::grovedb::GroveDb::open(db_path).expect("open grovedb");
    // TODO: fetch platform version instead of taking latest
    let result = grovedb
        .visualize_verify_grovedb(
            None,
            true,
            true,
            &PlatformVersion::latest().drive.grove_version,
        )
        .map_err(|e| e.to_string());

    match result {
        Ok(data) => {
            for result in data {
                tracing::warn!(?result, "grovedb verification")
            }
            tracing::info!("grovedb verification finished");

            if fsck.exists() {
                if let Err(e) = remove_file(&fsck) {
                    tracing::warn!(
                        error = ?e,
                        path  =fsck.display().to_string(),
                        "grovedb verification: cannot remove .fsck file: please remove it manually to avoid running verification again",
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            tracing::error!("grovedb verification failed: {}", e);
            Err(e)
        }
    }
}

fn verify_app_hash_matches_grove_root(
    drive: &Drive,
    platform_state: &PlatformState,
    platform_version: &PlatformVersion,
) -> Result<(), String> {
    let app_hash = platform_state
        .last_committed_block_app_hash()
        .ok_or_else(|| "platform state missing last committed app hash".to_string())?;

    let grove_root = drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .map_err(|e| e.to_string())?;

    if grove_root != app_hash {
        return Err(format!(
            "app hash mismatch: platform state 0x{} vs grovedb root 0x{}",
            hex::encode(app_hash),
            hex::encode(grove_root)
        ));
    }

    tracing::info!("platform state app hash matches grovedb root hash");
    Ok(())
}

fn verify_config_matches_db(_config: &PlatformConfig, drive: &Drive) -> Result<(), String> {
    let stored_protocol_version = drive
        .fetch_current_protocol_version(None)
        .map_err(|e| e.to_string())?;

    if let Some(stored) = stored_protocol_version {
        let binary_supported = PlatformVersion::latest().protocol_version;
        if stored > binary_supported {
            return Err(format!(
                "database protocol version {} is newer than binary supports {}",
                stored, binary_supported
            ));
        }
        tracing::info!(
            "protocol version in DB {} is compatible with binary {}",
            stored,
            binary_supported
        );
    } else {
        tracing::warn!("no protocol version found in DB to compare with binary support");
    }

    Ok(())
}
