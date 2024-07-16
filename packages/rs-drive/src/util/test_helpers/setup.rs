//! Drive Setup Helpers.
//!
//! Defines helper functions pertinent to setting up Drive.
//!

#[cfg(test)]
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
#[cfg(test)]
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

#[cfg(test)]
/// Sets up Drive using a temporary directory and the optionally given Drive configuration settings.
pub fn setup_drive(drive_config: Option<DriveConfig>) -> Drive {
    let tmp_dir = TempDir::new().unwrap();

    let (drive, _) = Drive::open(tmp_dir, drive_config).expect("should open Drive successfully");

    drive
}

#[cfg(test)]
/// Sets up Drive using a temporary directory and the default initial state structure.
pub fn setup_drive_with_initial_state_structure() -> Drive {
    let drive = setup_drive(Some(DriveConfig {
        batching_consistency_verification: true,
        ..Default::default()
    }));

    let platform_version = PlatformVersion::latest();
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
