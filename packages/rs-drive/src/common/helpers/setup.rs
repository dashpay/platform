use crate::drive::config::DriveConfig;
use crate::drive::Drive;
use tempfile::TempDir;

pub struct SetupFeePoolsOptions {
    pub apply_fee_pool_structure: bool,
}

impl Default for SetupFeePoolsOptions {
    fn default() -> SetupFeePoolsOptions {
        SetupFeePoolsOptions {
            apply_fee_pool_structure: true,
        }
    }
}

pub fn setup_drive(drive_config: Option<DriveConfig>) -> Drive {
    let tmp_dir = TempDir::new().unwrap();
    let drive: Drive = Drive::open(tmp_dir, drive_config).expect("should open Drive successfully");

    drive
}

pub fn setup_drive_with_initial_state_structure() -> Drive {
    let drive = setup_drive(None);
    drive
        .create_initial_state_structure(None)
        .expect("should create root tree successfully");

    drive
}
