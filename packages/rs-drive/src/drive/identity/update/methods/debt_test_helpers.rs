//! Test helpers shared by the identity debt repayment tests of `add_to_identity_balance` and
//! `apply_balance_change_from_fee_to_identity`.

use crate::drive::Drive;
use crate::error::Error;
use crate::util::test_helpers::test_utils::identities::create_test_identity;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::version::PlatformVersion;

/// The epoch the debt repayment tests write in
pub(crate) const DEBT_TEST_EPOCH_INDEX: u16 = 2;

/// A block of [`DEBT_TEST_EPOCH_INDEX`]
pub(crate) fn debt_test_block_info() -> BlockInfo {
    BlockInfo::default_with_epoch(Epoch::new(DEBT_TEST_EPOCH_INDEX).expect("a valid epoch index"))
}

/// The processing fees of [`DEBT_TEST_EPOCH_INDEX`], 0 before anything was written there
pub(crate) fn processing_pool(drive: &Drive, platform_version: &PlatformVersion) -> Credits {
    match drive.get_epoch_processing_credits_for_distribution(
        &Epoch::new(DEBT_TEST_EPOCH_INDEX).expect("a valid epoch index"),
        None,
        platform_version,
    ) {
        Ok(credits) => credits,
        Err(Error::GroveDB(error))
            if matches!(error.as_ref(), grovedb::Error::PathKeyNotFound(_)) =>
        {
            0
        }
        Err(error) => panic!("expected to read the processing fee pool: {error}"),
    }
}

/// The identity's balance
pub(crate) fn balance(
    drive: &Drive,
    identity_id: [u8; 32],
    platform_version: &PlatformVersion,
) -> Credits {
    drive
        .fetch_identity_balance(identity_id, None, platform_version)
        .expect("expected to fetch the balance")
        .expect("expected the identity to have a balance")
}

/// The identity's debt, its negative credit balance
pub(crate) fn debt(
    drive: &Drive,
    identity_id: [u8; 32],
    platform_version: &PlatformVersion,
) -> Credits {
    drive
        .fetch_identity_negative_balance_operations(
            identity_id,
            true,
            None,
            &mut vec![],
            platform_version,
        )
        .expect("expected to fetch the debt")
        .expect("expected a stored debt")
}

/// Whether the platform's credit total equals the sum of every tree it counts
pub(crate) fn credits_are_balanced(drive: &Drive, platform_version: &PlatformVersion) -> bool {
    drive
        .calculate_total_credits_balance(None, &platform_version.drive)
        .expect("expected to calculate the credit sum")
        .ok()
        .expect("expected no overflow")
}

/// An identity with an empty balance that owes `owed` credits, the way an unpaid part of a fee
/// leaves it
pub(crate) fn indebted_identity(
    drive: &Drive,
    id: [u8; 32],
    owed: Credits,
    platform_version: &PlatformVersion,
) -> [u8; 32] {
    let identity = create_test_identity(drive, id, Some(u64::from(id[0])), None, platform_version)
        .expect("expected an identity");
    let debt_operation = drive
        .update_identity_negative_credit_operation(
            identity.id().to_buffer(),
            owed,
            platform_version,
        )
        .expect("expected a debt operation");
    drive
        .apply_batch_low_level_drive_operations(
            None,
            None,
            vec![debt_operation],
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to store the debt");
    identity.id().to_buffer()
}
