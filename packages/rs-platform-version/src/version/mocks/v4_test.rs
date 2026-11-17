use crate::version::fee::storage::FeeStorageVersion;
use crate::version::fee::v4::FEE_VERSION4;
use crate::version::fee::FeeVersion;
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v15::PLATFORM_V15;

pub const TEST_PROTOCOL_VERSION_4: u32 = (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 4;

/// Protocol version 15 with twice its storage rate, so a test can tell which epoch's schedule
/// priced a refund. No network runs it.
pub const TEST_PLATFORM_V4: PlatformVersion = PlatformVersion {
    protocol_version: TEST_PROTOCOL_VERSION_4,
    fee_version: FeeVersion {
        storage: FeeStorageVersion {
            storage_disk_usage_credit_per_byte: 2 * FEE_VERSION4
                .storage
                .storage_disk_usage_credit_per_byte,
            ..FEE_VERSION4.storage
        },
        ..FEE_VERSION4
    },
    ..PLATFORM_V15
};
