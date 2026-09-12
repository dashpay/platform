mod fetch_platform_state_bytes;
mod fetch_reduced_platform_state_bytes;
mod store_platform_state_bytes;
mod store_reduced_platform_state_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";
pub(crate) const REDUCED_PLATFORM_STATE_KEY: &[u8; 19] = b"reduced_saved_state";

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_return_none_when_reduced_platform_state_is_absent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let fetched = drive
            .fetch_reduced_platform_state_bytes(None, platform_version)
            .expect("fetching an absent reduced platform state should not error");

        assert_eq!(fetched, None);
    }

    #[test]
    fn should_roundtrip_reduced_platform_state_bytes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let state_bytes = vec![1u8, 2, 3, 4, 5];

        drive
            .store_reduced_platform_state_bytes(&state_bytes, None, platform_version)
            .expect("should store reduced platform state");

        let fetched = drive
            .fetch_reduced_platform_state_bytes(None, platform_version)
            .expect("should fetch reduced platform state");

        assert_eq!(fetched, Some(state_bytes));
    }

    #[test]
    fn should_overwrite_reduced_platform_state_bytes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        drive
            .store_reduced_platform_state_bytes(&[1u8, 2, 3], None, platform_version)
            .expect("should store reduced platform state");

        let updated_bytes = vec![9u8, 8, 7];
        drive
            .store_reduced_platform_state_bytes(&updated_bytes, None, platform_version)
            .expect("should overwrite reduced platform state");

        let fetched = drive
            .fetch_reduced_platform_state_bytes(None, platform_version)
            .expect("should fetch reduced platform state");

        assert_eq!(fetched, Some(updated_bytes));
    }
}
