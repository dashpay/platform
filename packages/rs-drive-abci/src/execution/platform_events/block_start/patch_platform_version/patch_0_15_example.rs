use dpp::version::PlatformVersion;

pub fn patch_0_15_example(mut platform_version: PlatformVersion) -> PlatformVersion {
    platform_version.drive_abci.methods.engine.check_tx = 0;

    platform_version
}
