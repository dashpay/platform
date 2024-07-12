use dpp::version::PlatformVersion;

pub fn patch_2_30_test(mut platform_version: PlatformVersion) -> PlatformVersion {
    platform_version.drive_abci.query.document_query.min_version = 30;

    platform_version
}
