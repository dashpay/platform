//! The Dash Core forks Platform has seen active, each with the chain locked Core height of the
//! first Platform block that saw it.

mod fetch_core_fork_activation_height;
mod store_core_fork_activation_height;

/// The first byte of every Core fork activation key in `Misc`; the fork's id follows it.
pub const CORE_FORK_ACTIVATION_KEY_PREFIX: u8 = b'F';

/// The `Misc` key holding the Core height at which Platform first saw Core's V24 active
pub const CORE_FORK_V24_ACTIVATION_KEY: &[u8; 2] =
    &[CORE_FORK_ACTIVATION_KEY_PREFIX, CoreFork::V24 as u8];

/// A Dash Core fork whose activation Platform records.
///
/// The discriminant is the fork's id in its `Misc` key, so it never changes.
// @append_only
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreFork {
    /// Dash Core's `DEPLOYMENT_V24`
    V24 = 0,
}

impl CoreFork {
    /// The `Misc` key holding the fork's activation height
    pub const fn activation_key(self) -> &'static [u8; 2] {
        match self {
            CoreFork::V24 => CORE_FORK_V24_ACTIVATION_KEY,
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::{CoreFork, CORE_FORK_V24_ACTIVATION_KEY};
    use crate::drive::system::misc_path;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn should_return_none_for_a_core_fork_never_recorded() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        assert_eq!(
            drive
                .fetch_core_fork_activation_height(
                    CoreFork::V24,
                    Some(&transaction),
                    platform_version
                )
                .expect("expected to read an absent record"),
            None
        );
    }

    /// The record is a big endian `u32` item without flags at `[b'F', 0]` in `Misc`, which
    /// clients can prove through `getPathElements`. It is visible in the transaction that
    /// wrote it before that transaction commits, and nowhere else.
    #[test]
    fn should_read_back_the_observed_height() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        drive
            .store_core_fork_activation_height(
                CoreFork::V24,
                1_234_567,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record V24");

        assert_eq!(
            drive
                .fetch_core_fork_activation_height(
                    CoreFork::V24,
                    Some(&transaction),
                    platform_version
                )
                .expect("expected to read the record"),
            Some(1_234_567)
        );
        assert_eq!(CORE_FORK_V24_ACTIVATION_KEY, &[b'F', 0]);
        assert_eq!(
            drive
                .grove_get_raw_optional(
                    (&misc_path()).into(),
                    CORE_FORK_V24_ACTIVATION_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    Some(&transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to read the element"),
            Some(Element::Item(1_234_567u32.to_be_bytes().to_vec(), None))
        );
        assert_eq!(
            drive
                .fetch_core_fork_activation_height(CoreFork::V24, None, platform_version)
                .expect("expected to read outside the transaction"),
            None,
            "an uncommitted record must not be visible outside its transaction"
        );
    }

    /// A recorded height is what every later block compares with, so recording the fork again
    /// is a bug and leaves the first height in place.
    #[test]
    fn should_refuse_to_record_a_core_fork_twice() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        drive
            .store_core_fork_activation_height(
                CoreFork::V24,
                1_000,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record V24");

        for core_height in [1_000, 2_000] {
            let result = drive.store_core_fork_activation_height(
                CoreFork::V24,
                core_height,
                Some(&transaction),
                platform_version,
            );
            assert!(
                matches!(
                    result,
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
                ),
                "recording V24 again at {core_height} must fail, got {result:?}"
            );
        }
        assert_eq!(
            drive
                .fetch_core_fork_activation_height(
                    CoreFork::V24,
                    Some(&transaction),
                    platform_version
                )
                .expect("expected to read the record"),
            Some(1_000)
        );
    }

    #[test]
    fn should_refuse_a_stored_height_that_is_not_four_bytes() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        drive
            .grove_insert(
                (&misc_path()).into(),
                CORE_FORK_V24_ACTIVATION_KEY,
                Element::Item(vec![0, 0, 1], None),
                None,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to write a malformed record");

        assert!(matches!(
            drive.fetch_core_fork_activation_height(CoreFork::V24, None, platform_version),
            Err(Error::Drive(DriveError::CorruptedSerialization(_)))
        ));
    }

    #[test]
    fn should_neither_read_nor_record_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        assert!(matches!(
            drive.fetch_core_fork_activation_height(CoreFork::V24, None, platform_version),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
        assert!(matches!(
            drive.store_core_fork_activation_height(CoreFork::V24, 1_000, None, platform_version),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}
