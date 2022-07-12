use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::{KeyInfo, PathKeyElementInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use grovedb::TransactionArg;

pub struct Batch<'d> {
    drive: &'d Drive,
    pub operations: Vec<DriveOperation>,
}

// TODO: Move batch_* methods from grove operations to this structure so we can get rid of drive here
impl<'d> Batch<'d> {
    pub fn new(drive: &'d Drive) -> Self {
        Batch {
            drive,
            operations: Vec::new(),
        }
    }

    pub fn insert_empty_tree<'c, P>(
        &mut self,
        path: P,
        key_info: KeyInfo<'c>,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        self.drive
            .batch_insert_empty_tree(path, key_info, storage_flags, &mut self.operations)
    }

    pub fn insert<const N: usize>(
        &mut self,
        path_key_element_info: PathKeyElementInfo<N>,
    ) -> Result<(), Error> {
        self.drive
            .batch_insert(path_key_element_info, &mut self.operations)
    }

    pub fn delete<'c, P>(
        &mut self,
        // TODO: Pass drive (storage eventually) here when we remove drive from the struct
        path: P,
        key: &'c [u8],
        only_delete_tree_if_empty: bool,
        transaction: TransactionArg,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        self.drive.batch_delete(
            path,
            key,
            only_delete_tree_if_empty,
            transaction,
            &mut self.operations,
        )
    }
}
