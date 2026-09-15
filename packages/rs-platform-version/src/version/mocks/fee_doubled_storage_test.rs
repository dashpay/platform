use crate::version::fee::storage::v1::FEE_STORAGE_VERSION1;
use crate::version::fee::storage::FeeStorageVersion;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::fee::{FeeVersion, FeeVersionNumber};
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;

/// The fee generation number of the doubled-storage test schedule.
///
/// Test fee generations live in the same shifted range as the mock protocol
/// versions (a set high bit that no production number can carry), so a
/// persisted fee history that names this number is rejected by a node built
/// without `mock-versions`, exactly as a mock protocol version would be.
pub const TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE: FeeVersionNumber =
    (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 1;

/// Storage rates of the doubled-storage test schedule: the disk usage rate is
/// exactly twice `FEE_STORAGE_VERSION1`, every other storage rate is unchanged.
pub const TEST_FEE_STORAGE_VERSION_DOUBLED: FeeStorageVersion = FeeStorageVersion {
    storage_disk_usage_credit_per_byte: 2 * FEE_STORAGE_VERSION1.storage_disk_usage_credit_per_byte,
    storage_processing_credit_per_byte: FEE_STORAGE_VERSION1.storage_processing_credit_per_byte,
    storage_load_credit_per_byte: FEE_STORAGE_VERSION1.storage_load_credit_per_byte,
    non_storage_load_credit_per_byte: FEE_STORAGE_VERSION1.non_storage_load_credit_per_byte,
    storage_seek_cost: FEE_STORAGE_VERSION1.storage_seek_cost,
};

/// A fee generation that exists only under `mock-versions`.
///
/// No shipped schedule carries a `fee_version_number` other than 1, so the
/// registry lookup, the epoch-change hook, the saved-state round trip and the
/// history-driven refund path have no production input that exercises a
/// second generation. This schedule is `FEE_VERSION2` with a new generation
/// number and a doubled storage disk usage rate: processing, hashing and
/// signature rates are identical, so any fee difference across a boundary
/// into this generation is storage, and a refund priced at the wrong
/// generation is off by a factor of two.
///
/// It is unreachable in production builds: the constant is compiled only
/// with `mock-versions`, and `FeeVersion::get` resolves the shifted number
/// range only under that feature.
pub const TEST_FEE_VERSION_DOUBLED_STORAGE: FeeVersion = FeeVersion {
    fee_version_number: TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE,
    storage: TEST_FEE_STORAGE_VERSION_DOUBLED,
    ..FEE_VERSION2
};

/// The test fee generations, resolved by number through `FeeVersion::get`
/// when the number is in the shifted test range.
pub const TEST_FEE_VERSIONS: &[FeeVersion] = &[TEST_FEE_VERSION_DOUBLED_STORAGE];
