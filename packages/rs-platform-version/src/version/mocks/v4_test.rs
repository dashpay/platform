use crate::version::mocks::fee_doubled_storage_test::TEST_FEE_VERSION_DOUBLED_STORAGE;
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v15::PLATFORM_V15;

pub const TEST_PROTOCOL_VERSION_4: u32 = (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 4;

/// Protocol version 15 with twice its storage rate, so a test can tell which epoch's schedule
/// priced a refund. No network runs it.
///
/// Every dispatch table is the latest protocol version's, so an upgrade from
/// the latest protocol version to this mock changes nothing but the fee
/// schedule: `perform_events_on_first_block_of_protocol_change` fires no
/// migration (its gates compare against real protocol version numbers, all
/// below the mock). After activation the observable differences are the
/// storage rate, which a refund of bytes stored in an epoch that ran this mock
/// is priced at (read from that epoch's recorded protocol version), and the fee
/// history entry the epoch-change hook records for the schedule's own
/// `fee_version_number`.
///
/// When a new protocol version is introduced, move the base to its table (and
/// the doubled-storage schedule to its fee schedule) so the mock keeps
/// tracking the latest behaviour.
pub const TEST_PLATFORM_V4: PlatformVersion = PlatformVersion {
    protocol_version: TEST_PROTOCOL_VERSION_4,
    fee_version: TEST_FEE_VERSION_DOUBLED_STORAGE,
    ..PLATFORM_V15
};
