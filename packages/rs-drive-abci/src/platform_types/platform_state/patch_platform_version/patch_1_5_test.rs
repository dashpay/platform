use dpp::version::PlatformVersion;

pub fn patch_1_5_test(mut platform_version: PlatformVersion) -> PlatformVersion {
    platform_version
        .drive_abci
        .query
        .document_query
        .default_current_version = 5;

    // We want to speed up tests
    platform_version
        .drive_abci
        .methods
        .protocol_upgrade
        .protocol_version_upgrade_percentage_needed = 1;

    platform_version
}
