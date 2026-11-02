//! Drive Setup Helpers.
//!
//! Defines helper functions pertinent to setting up Drive.
//!

#[cfg(feature = "full")]
use crate::config::DriveConfig;
use crate::drive::Drive;
use dpp::block::block_info::BlockInfo;

use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
#[cfg(feature = "full")]
use tempfile::TempDir;

/// Struct with options regarding setting up fee pools.
pub struct SetupFeePoolsOptions {
    /// Bool indicating whether the fee pool structure should be applied upon setup.
    pub apply_fee_pool_structure: bool,
}

impl Default for SetupFeePoolsOptions {
    /// The default is true for applying the fee pool structure upon setting up fee pools.
    fn default() -> SetupFeePoolsOptions {
        SetupFeePoolsOptions {
            apply_fee_pool_structure: true,
        }
    }
}

#[cfg(feature = "full")]
/// Sets up Drive using a temporary directory and the optionally given Drive configuration settings.
///
/// The returned Drive owns the directory, which is removed when the Drive is dropped.
pub fn setup_drive(drive_config: Option<DriveConfig>) -> Drive {
    let tmp_dir = TempDir::new().unwrap();

    let (mut drive, _) =
        Drive::open(tmp_dir.path(), drive_config).expect("should open Drive successfully");
    drive.temp_dir = Some(tmp_dir);

    drive
}

#[cfg(feature = "full")]
/// Sets up Drive using a temporary directory and the default initial state structure.
pub fn setup_drive_with_initial_state_structure(
    specific_platform_version: Option<&PlatformVersion>,
) -> Drive {
    let drive = setup_drive(Some(DriveConfig {
        batching_consistency_verification: true,
        ..Default::default()
    }));

    let platform_version = specific_platform_version.unwrap_or(PlatformVersion::latest());
    drive
        .create_initial_state_structure(None, platform_version)
        .expect("should create root tree successfully");

    drive
}

/// A function to setup system data contract
pub fn setup_system_data_contract(
    drive: &Drive,
    data_contract: &DataContract,
    transaction: TransactionArg,
) {
    let platform_version = PlatformVersion::latest();
    drive
        .apply_contract(
            data_contract,
            BlockInfo::default(),
            true,
            None,
            transaction,
            platform_version,
        )
        .unwrap();
}

/// Setup document for a contract
pub fn setup_document(
    drive: &Drive,
    document: &Document,
    data_contract: &DataContract,
    document_type: DocumentTypeRef,
    transaction: TransactionArg,
) {
    let platform_version = PlatformVersion::latest();
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((document, None)),
                    owner_id: None,
                },
                contract: data_contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            transaction,
            platform_version,
            None,
        )
        .unwrap();
}

#[cfg(all(test, feature = "full"))]
mod tests {
    use super::*;
    use crate::drive::system::misc_path;
    use grovedb::Element;

    #[test]
    fn should_write_new_database_files_after_setup_returns() {
        let drive = setup_drive_with_initial_state_structure(None);
        let grove_version = &PlatformVersion::latest().drive.grove_version;
        let item = Element::new_item(vec![7; 1024]);

        drive
            .grove
            .insert(
                &misc_path(),
                b"after setup",
                item.clone(),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("should insert an item");

        // A flush starts a new log file and writes the memtable to a new table file, both
        // created in the Drive's directory, so it fails once that directory is gone.
        drive
            .grove
            .flush()
            .expect("should flush the memtable to disk");

        let stored = drive
            .grove
            .get(&misc_path(), b"after setup", None, grove_version)
            .unwrap()
            .expect("should read the item back");
        assert_eq!(stored, item);
    }

    #[test]
    fn should_remove_the_temp_directory_when_the_drive_is_dropped() {
        let drive = setup_drive(None);
        let path = drive
            .temp_dir
            .as_ref()
            .expect("setup_drive should hand its temporary directory to the Drive")
            .path()
            .to_path_buf();
        assert!(path.exists());

        drop(drive);

        assert!(!path.exists());
    }
}
