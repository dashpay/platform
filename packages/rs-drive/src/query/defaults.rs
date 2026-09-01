/// Max index difference constant
pub(crate) const MAX_INDEX_DIFFERENCE: u16 = 2;

/// Maximum product of `In` clause list sizes in a single document query.
/// Each `In` value opens one subtree of the compound index, so this caps the
/// branch enumeration of a multi-`In` query at the same worst case as a
/// single maximal `In` clause (100 values).
pub const MAX_IN_CROSS_PRODUCT_SIZE: usize = 100;
