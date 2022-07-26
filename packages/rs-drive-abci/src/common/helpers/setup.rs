use crate::platform::Platform;
use tempfile::TempDir;

pub fn setup_platform() -> Platform {
    let tmp_dir = TempDir::new().unwrap();
    let drive: Platform = Platform::open(tmp_dir, None).expect("should open Platform successfully");

    drive
}

pub fn setup_platform_with_initial_state_structure() -> Platform {
    let platform = setup_platform();
    platform
        .drive
        .create_initial_state_structure(None)
        .expect("should create root tree successfully");

    platform
}
