use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::KeyInfo::{Key, KeyRef, KeySize};
use crate::drive::object_size_info::KeyValueInfo::{KeyRefRequest, KeyValueMaxSize};
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyElement, PathKeyElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::drive::object_size_info::{KeyInfo, KeyValueInfo, PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::{DriveOperation, QueryOperation};
use crate::query::GroveError;
use grovedb::batch::{GroveDbOp, Op};
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(crate) fn grove_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: KeyInfo<'c>,
        storage_flags: &StorageFlags,
        transaction: TransactionArg,
        apply: bool,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyRef(key) => {
                let (path_items, path): (Vec<Vec<u8>>, Vec<&[u8]>) =
                    path.into_iter().map(|x| (Vec::from(x), x)).unzip();
                if let Some(drive_operations) = drive_operations {
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                if apply {
                    self.grove
                        .insert(
                            path,
                            key,
                            Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                            transaction,
                        )
                        .map_err(Error::GroveDB)?
                }
                Ok(())
            }
            KeySize(key_max_length) => {
                if let Some(drive_operations) = drive_operations {
                    let path_size: u32 = path.into_iter().map(|p| p.len() as u32).sum();
                    drive_operations.push(DriveOperation::for_path_key_value_size(
                        path_size,
                        key_max_length as u16,
                        0,
                    ));
                }
                Ok(())
            }
            Key(_) => Err(Error::Drive(DriveError::GroveDBInsertion(
                "only a key ref can be inserted into groveDB",
            ))),
        }
    }

    pub(crate) fn grove_insert_empty_tree_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_info: PathKeyInfo<'c, N>,
        storage_flags: &StorageFlags,
        transaction: TransactionArg,
        apply: bool,
        query_operations: Option<&mut Vec<QueryOperation>>,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_clone = path.clone();
                let path_iter: Vec<&[u8]> = path_clone.iter().map(|x| x.as_slice()).collect();
                let inserted = if apply {
                    self.grove.insert_if_not_exists(
                        path_iter.clone(),
                        key,
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    )?
                } else {
                    true
                };
                if inserted {
                    if let Some(drive_operations) = drive_operations {
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path,
                            key.to_vec(),
                            storage_flags,
                        ));
                    }
                }
                if let Some(query_operations) = query_operations {
                    query_operations
                        .push(QueryOperation::for_key_check_in_path(key.len(), path_iter));
                }
                Ok(inserted)
            }
            PathKeySize((path_length, key_length)) => {
                if let Some(drive_operations) = drive_operations {
                    drive_operations.push(DriveOperation::for_path_key_value_size(
                        path_length as u32,
                        key_length as u16,
                        0,
                    ));
                }
                if let Some(query_operations) = query_operations {
                    query_operations.push(QueryOperation::for_key_check_with_path_length(
                        key_length,
                        path_length,
                    ));
                }
                Ok(true)
            }
            PathKey((path, key)) => {
                let path_clone = path.clone();
                let path_iter: Vec<&[u8]> = path_clone.iter().map(|x| x.as_slice()).collect();
                let inserted = if apply {
                    self.grove.insert_if_not_exists(
                        path_iter.clone(),
                        key.as_slice(),
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    )?
                } else {
                    true
                };
                if inserted {
                    if let Some(drive_operations) = drive_operations {
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path,
                            key.to_vec(),
                            storage_flags,
                        ));
                    }
                }
                if let Some(query_operations) = query_operations {
                    query_operations
                        .push(QueryOperation::for_key_check_in_path(key.len(), path_iter));
                }
                Ok(inserted)
            }
            PathFixedSizeKey((path, key)) => {
                let inserted = if apply {
                    self.grove.insert_if_not_exists(
                        path.clone(),
                        key.as_slice(),
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    )?
                } else {
                    true
                };
                if inserted {
                    if let Some(drive_operations) = drive_operations {
                        let path_clone = path.clone();
                        let path_items: Vec<Vec<u8>> =
                            path_clone.into_iter().map(Vec::from).collect();
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path_items,
                            key.to_vec(),
                            storage_flags,
                        ));
                    }
                }
                if let Some(query_operations) = query_operations {
                    query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                }
                Ok(inserted)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let inserted = if apply {
                    self.grove.insert_if_not_exists(
                        path.clone(),
                        key,
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    )?
                } else {
                    true
                };
                if inserted {
                    if let Some(drive_operations) = drive_operations {
                        let path_clone = path.clone();
                        let path_items: Vec<Vec<u8>> =
                            path_clone.into_iter().map(Vec::from).collect();
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path_items,
                            key.to_vec(),
                            storage_flags,
                        ));
                    }
                }
                if let Some(query_operations) = query_operations {
                    query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                }
                Ok(inserted)
            }
        }
    }

    pub(crate) fn grove_insert<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        apply: bool,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                if let Some(drive_operations) = drive_operations {
                    let path_size = path.iter().map(|x| x.len() as u32).sum();
                    let key_len = key.len();
                    drive_operations.push(DriveOperation::for_path_key_value_size(
                        path_size,
                        key_len as u16,
                        element.node_byte_size(key_len) as u32,
                    ));
                }
                if apply {
                    self.grove
                        .insert(path_iter, key, element, transaction)
                        .map_err(Error::GroveDB)
                } else {
                    Ok(())
                }
            }
            PathKeyElementSize((path_max_length, key_max_length, element_max_size)) => {
                if let Some(drive_operations) = drive_operations {
                    drive_operations.push(DriveOperation::for_path_key_value_size(
                        path_max_length as u32,
                        key_max_length as u16,
                        element_max_size as u32,
                    ));
                }
                Ok(())
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                if let Some(drive_operations) = drive_operations {
                    let path_size = path.into_iter().map(|a| a.len() as u32).sum();
                    let key_len = key.len();
                    drive_operations.push(DriveOperation::for_path_key_value_size(
                        path_size,
                        key_len as u16,
                        element.node_byte_size(key_len) as u32,
                    ));
                }
                if apply {
                    self.grove
                        .insert(path, key, element, transaction)
                        .map_err(Error::GroveDB)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub(crate) fn grove_insert_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        apply: bool,
        query_operations: Option<&mut Vec<QueryOperation>>,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let (path_iter, path_lengths): (Vec<&[u8]>, Vec<u32>) =
                    path.iter().map(|x| (x.as_slice(), x.len() as u32)).unzip();
                let element_node_byte_size = if drive_operations.is_some() {
                    element.node_byte_size(key.len()) as u32
                } else {
                    0 //doesn't matter
                };
                let inserted = if apply {
                    self.grove
                        .insert_if_not_exists(path_iter.clone(), key, element, transaction)?
                } else {
                    true
                };
                if inserted {
                    if let Some(drive_operations) = drive_operations {
                        let insert_operation = DriveOperation::for_path_key_value_size(
                            path_lengths.iter().sum(),
                            key.len() as u16,
                            element_node_byte_size,
                        );
                        drive_operations.push(insert_operation);
                    }
                }
                if let Some(query_operations) = query_operations {
                    let query_operation =
                        QueryOperation::for_key_check_in_path(key.len(), path_iter);
                    query_operations.push(query_operation);
                }
                Ok(inserted)
            }
            PathKeyElementSize((path_size, key_max_length, element_max_size)) => {
                let insert_operation = DriveOperation::for_path_key_value_size(
                    path_size as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                );
                let query_operation =
                    QueryOperation::for_key_check_with_path_length(key_max_length, path_size);
                if let Some(drive_operations) = drive_operations {
                    drive_operations.push(insert_operation);
                }
                if let Some(query_operations) = query_operations {
                    query_operations.push(query_operation);
                }
                Ok(true)
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                let path_iter = path.into_iter();
                let element_node_byte_size = if drive_operations.is_some() {
                    element.node_byte_size(key.len()) as u32
                } else {
                    0 //doesn't matter
                };
                let inserted = if apply {
                    self.grove
                        .insert_if_not_exists(path_iter.clone(), key, element, transaction)?
                } else {
                    true
                };
                if inserted {
                    if let Some(drive_operations) = drive_operations {
                        let path_size = path_iter.clone().map(|a| a.len() as u32).sum();
                        let key_len = key.len();
                        let insert_operation = DriveOperation::for_path_key_value_size(
                            path_size,
                            key_len as u16,
                            element_node_byte_size,
                        );
                        drive_operations.push(insert_operation);
                    }
                }
                if let Some(query_operations) = query_operations {
                    let query_operation =
                        QueryOperation::for_key_check_in_path(key.len(), path_iter.clone());
                    query_operations.push(query_operation);
                }
                Ok(inserted)
            }
        }
    }

    pub(crate) fn batch_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: KeyInfo<'c>,
        storage_flags: &StorageFlags,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyRef(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(DriveOperation::for_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                ));
                Ok(())
            }
            KeySize(key_max_length) => {
                let path_size = path.into_iter().map(|x| x.len() as u32).sum();
                drive_operations.push(DriveOperation::for_path_key_value_size(
                    path_size,
                    key_max_length as u16,
                    0,
                ));
                Ok(())
            }
            Key(_) => Err(Error::Drive(DriveError::GroveDBInsertion(
                "only a key ref can be inserted into groveDB",
            ))),
        }
    }

    pub(crate) fn grove_has_raw<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        transaction: TransactionArg,
    ) -> Result<bool, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let query_result = self.grove.has_raw(path, key, transaction);
        match query_result {
            Err(GroveError::PathKeyNotFound(_)) | Err(GroveError::PathNotFound(_)) => Ok(false),
            _ => Ok(query_result?),
        }
    }

    pub(crate) fn batch_insert_empty_tree_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_info: PathKeyInfo<'c, N>,
        storage_flags: &StorageFlags,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(path_iter.clone(), key, transaction)?;
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path_iter));
                if has_raw == false {
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                Ok(!has_raw)
            }
            PathKeySize((path_length, key_length)) => {
                drive_operations.push(DriveOperation::for_path_key_value_size(
                    path_length as u32,
                    key_length as u16,
                    0,
                ));

                query_operations.push(QueryOperation::for_key_check_with_path_length(
                    key_length,
                    path_length,
                ));
                Ok(true)
            }
            PathKey((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(path_iter.clone(), key.as_slice(), transaction)?;
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path_iter));
                if has_raw == false {
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKey((path, key)) => {
                let has_raw = self.grove_has_raw(path.clone(), key.as_slice(), transaction)?;
                if has_raw == false {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(!has_raw)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let has_raw = self.grove_has_raw(path.clone(), key, transaction)?;
                if has_raw == false {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(!has_raw)
            }
        }
    }

    pub(crate) fn batch_insert<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                drive_operations.push(DriveOperation::for_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
            PathKeyElementSize((path_max_length, key_max_length, element_max_size)) => {
                drive_operations.push(DriveOperation::for_path_key_value_size(
                    path_max_length as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                ));
                Ok(())
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(DriveOperation::for_path_key_element(
                    path_items,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
        }
    }

    pub(crate) fn batch_insert_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(path_iter.clone(), key, transaction)?;
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path_iter));
                if has_raw == false {
                    drive_operations.push(DriveOperation::for_path_key_element(
                        path,
                        key.to_vec(),
                        element,
                    ));
                }
                Ok(!has_raw)
            }
            PathKeyElementSize((path_size, key_max_length, element_max_size)) => {
                let insert_operation = DriveOperation::for_path_key_value_size(
                    path_size as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                );
                let query_operation =
                    QueryOperation::for_key_check_with_path_length(key_max_length, path_size);
                drive_operations.push(insert_operation);
                query_operations.push(query_operation);
                Ok(true)
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(path, key, transaction)?;
                if has_raw == false {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_path_key_element(
                        path_items,
                        key.to_vec(),
                        element,
                    ));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(!has_raw)
            }
        }
    }

    pub(crate) fn batch_delete<'a, 'c, P>(
        &'a self,
        path: P,
        key: &'c [u8],
        only_delete_tree_if_empty: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let current_batch_operations = DriveOperation::grovedb_operations(drive_operations);
        if let Some(delete_operation) = self
            .grove
            .delete_operation_for_delete_internal(
                path,
                key,
                only_delete_tree_if_empty,
                true,
                &current_batch_operations,
                transaction,
            )
            .map_err(Error::GroveDB)?
        {
            drive_operations.push(DriveOperation::GroveOperation(delete_operation))
        }
        Ok(())
    }

    pub(crate) fn batch_delete_up_tree_while_empty<'a, 'c, P>(
        &'a self,
        path: P,
        key: &'c [u8],
        stop_path_height: Option<u16>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let current_batch_operations = DriveOperation::grovedb_operations(drive_operations);
        if let Some(delete_operations) = self
            .grove
            .delete_operations_for_delete_up_tree_while_empty(
                path,
                key,
                stop_path_height,
                true,
                &current_batch_operations,
                transaction,
            )
            .map_err(Error::GroveDB)?
        {
            delete_operations
                .into_iter()
                .for_each(|op| drive_operations.push(DriveOperation::GroveOperation(op)))
        }
        Ok(())
    }

    pub(crate) fn grove_get<'a, 'c, P>(
        &'a self,
        path: P,
        key_value_info: KeyValueInfo<'c>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        match key_value_info {
            KeyRefRequest(key) => {
                let item = self.grove.get(path_iter.clone(), key, transaction)?;
                query_operations.push(QueryOperation::for_value_retrieval_in_path(
                    key.len(),
                    path_iter,
                    item.serialized_byte_size(),
                ));
                Ok(Some(item))
            }
            KeyValueMaxSize((key_size, value_size)) => {
                query_operations.push(QueryOperation::for_value_retrieval_in_path(
                    key_size, path_iter, value_size,
                ));
                Ok(None)
            }
        }
    }

    pub(crate) fn grove_apply_batch(
        &self,
        ops: Vec<GroveDbOp>,
        validate: bool,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        if self.config.batching_enabled {
            self.grove
                .apply_batch(ops, validate, transaction)
                .map_err(Error::GroveDB)
        } else {
            //println!("changes {} {:#?}", ops.len(), ops);
            for op in ops.into_iter() {
                //println!("on {:#?}", op);
                match op.op {
                    Op::Insert { element } => {
                        self.grove_insert(
                            PathKeyElementInfo::<0>::PathKeyElement((
                                op.path.clone(),
                                op.key.as_slice(),
                                element,
                            )),
                            transaction,
                            true,
                            None,
                        )?;
                    }
                    Op::Delete => {
                        let path_iter: Vec<&[u8]> = op.path.iter().map(|x| x.as_slice()).collect();
                        self.grove
                            .delete(path_iter, op.key.as_slice(), transaction)
                            .map_err(Error::GroveDB)?;
                    }
                }
            }
            Ok(())
        }
    }
}
