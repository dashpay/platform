use dpp::version::PlatformVersion;

pub fn patch_1_10_test(mut platform_version: PlatformVersion) -> PlatformVersion {
    platform_version.drive_abci.query.document_query.max_version = 10;

    platform_version
}
