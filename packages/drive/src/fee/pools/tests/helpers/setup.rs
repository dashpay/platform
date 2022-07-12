use crate::drive::Drive;
use crate::fee::pools::fee_pools::FeePools;
use grovedb::Transaction;
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

pub fn setup_drive() -> Drive {
    let tmp_dir = TempDir::new().unwrap();
    let drive: Drive = Drive::open(tmp_dir, None).expect("should open Drive successfully");

    drive
}

pub fn setup_fee_pools<'a>(
    drive: &'a Drive,
    options: Option<SetupFeePoolsOptions>,
) -> (Transaction<'a>, FeePools) {
    let options = options.unwrap_or(SetupFeePoolsOptions::default());

    let transaction = drive.grove.start_transaction();

    let fee_pools = FeePools::new();

    if options.apply_fee_pool_structure {
        drive
            .create_initial_state_structure(None)
            .expect("should create root tree successfully");
    }

    (transaction, fee_pools)
}
