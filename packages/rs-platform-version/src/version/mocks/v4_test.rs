use crate::version::mocks::fee_doubled_storage_test::TEST_FEE_VERSION_DOUBLED_STORAGE;
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v14::PLATFORM_V14;

pub const TEST_PROTOCOL_VERSION_4: u32 = (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 4;

/// A mock protocol version that is the latest shipped protocol version with
/// the doubled-storage test fee generation.
///
/// Every dispatch table is the shipped one, so an upgrade from the latest
/// protocol version to this mock changes nothing but the fee generation:
/// `perform_events_on_first_block_of_protocol_change` fires no migration
/// (its gates compare against shipped numbers, all below the mock), and the
/// only observable difference after activation is the storage rate and the
/// fee history entry the epoch-change hook records.
///
/// When a new protocol version is introduced, move the base to its table so
/// the mock keeps tracking the latest shipped behaviour.
pub const TEST_PLATFORM_V4: PlatformVersion = PlatformVersion {
    protocol_version: TEST_PROTOCOL_VERSION_4,
    fee_version: TEST_FEE_VERSION_DOUBLED_STORAGE,
    ..PLATFORM_V14
};
