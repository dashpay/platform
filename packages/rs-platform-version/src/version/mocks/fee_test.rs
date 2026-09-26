use crate::version::fee::storage::FeeStorageVersion;
use crate::version::fee::v1::FEE_VERSION1;
use crate::version::fee::{FeeVersion, FeeVersionNumber};
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;

/// Number of the mock fee-history generation.
///
/// Test identifiers live above the same shift as test protocol versions, so a mock number can
/// never collide with a number a released network persisted, and it is never a position in the
/// shipped registry.
pub const TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE_RATE: FeeVersionNumber =
    (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 1;

/// A fee-history generation whose disk usage rate is twice the shipped one.
///
/// It exists so tests can put a storage-rate boundary into the fee history and observe it
/// through every consumer: the registry lookup, Drive's refund pricing and the saved-state
/// round trip. It is registered only when the `mock-versions` feature is on.
pub const TEST_FEE_VERSION_DOUBLED_STORAGE_RATE: FeeVersion = FeeVersion {
    fee_version_number: TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE_RATE,
    storage: FeeStorageVersion {
        storage_disk_usage_credit_per_byte: FEE_VERSION1.storage.storage_disk_usage_credit_per_byte
            * 2,
        ..FEE_VERSION1.storage
    },
    ..FEE_VERSION1
};

/// Mock generations consulted by `FeeVersion::get` after the shipped registry.
///
/// Never part of a release build: `mock-versions` is a development-only feature.
pub const FEE_TEST_VERSIONS: &[FeeVersion] = &[TEST_FEE_VERSION_DOUBLED_STORAGE_RATE];

const _: () = assert!(
    every_number_is_above_the_test_shift(FEE_TEST_VERSIONS),
    "mock fee version numbers must live above the test shift so they cannot collide with shipped numbers"
);

/// Returns true when every mock entry carries a number above the test shift.
const fn every_number_is_above_the_test_shift(registry: &[FeeVersion]) -> bool {
    let mut index = 0;
    while index < registry.len() {
        if registry[index].fee_version_number >> TEST_PROTOCOL_VERSION_SHIFT_BYTES == 0 {
            return false;
        }
        index += 1;
    }
    true
}
