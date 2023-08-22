use crate::drive::object_size_info::key_value_info::KeyValueInfo;
use crate::drive::object_size_info::PathInfo;

/// Deletion Info
#[allow(dead_code)]
pub struct DeletionInfo<'a, const N: usize> {
    upper_path: PathInfo<'a, N>,
    lower_path: Vec<KeyValueInfo<'a>>,
}
