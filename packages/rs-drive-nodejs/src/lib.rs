mod converter;

use std::collections::HashMap;
use std::ops::Deref;
use std::{option::Option::None, path::Path, sync::mpsc, thread};

use drive_abci::abci::handlers::TenderdashAbci;
use drive_abci::abci::messages::{
    AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, InitChainRequest, Serializable,
};
use drive_abci::platform::Platform;
use neon::prelude::*;
use drive::dpp::identity::Identity;
use drive::drive::batch::GroveDbOpBatch;
use drive::drive::config::DriveConfig;
use drive::drive::flags::StorageFlags;
use drive::error::drive::DriveError;
use drive::error::Error;
use drive::fee_pools::epochs::Epoch;
use drive::grovedb::{PathQuery, Transaction};

type TransactionPointerAddress = usize;

struct PlatformWrapperTransactionAddress {
    address: TransactionPointerAddress,
    tx: mpsc::Sender<PlatformWrapperMessage>,
}

impl Finalize for PlatformWrapperTransactionAddress {
    fn finalize<'a, C: Context<'a>>(self, _: &mut C) {
        // Ignoring the result of the `send` function as
        // it only fails if other side closed a connection
        // that would mean there is no reason using `cx.throw`
        // as thread is probably closed already
        self.tx
            .send(PlatformWrapperMessage::AbortTransaction(
                self.address,
                Box::new(|_, _| {}),
            ))
            .ok();
    }
}

impl Deref for PlatformWrapperTransactionAddress {
    type Target = TransactionPointerAddress;

    fn deref(&self) -> &Self::Target {
        &self.address
    }
}

type PlatformCallback = Box<
    dyn for<'a> FnOnce(&'a Platform, &HashMap<TransactionPointerAddress, Transaction>, &Channel)
        + Send,
>;
type UnitCallback = Box<dyn FnOnce(&Channel) + Send>;
type ErrorCallback = Box<dyn FnOnce(&Channel, Result<(), String>) + Send>;
type TransactionCallback =
    Box<dyn FnOnce(mpsc::Sender<PlatformWrapperMessage>, usize, &Channel) + Send>;

// Messages sent on the drive channel
enum PlatformWrapperMessage {
    // Callback to be executed
    Callback(PlatformCallback),
    // Indicates that the thread should be stopped and connection closed
    Close(UnitCallback),
    StartTransaction(TransactionCallback),
    CommitTransaction(TransactionPointerAddress, ErrorCallback),
    RollbackTransaction(TransactionPointerAddress, ErrorCallback),
    AbortTransaction(TransactionPointerAddress, ErrorCallback),
    Flush(UnitCallback),
}

struct PlatformWrapper {
    tx: mpsc::Sender<PlatformWrapperMessage>,
}

// Internal wrapper logic. Needed to avoid issues with passing threads to
// node.js. Avoiding thread conflicts by having a dedicated thread for the
// groveDB instance and uses events to communicate with it
impl PlatformWrapper {
    // Creates a new instance of `DriveWrapper`
    //
    // 1. Creates a connection and a channel
    // 2. Spawns a thread and moves the channel receiver and connection to it
    // 3. On a separate thread, read closures off the channel and execute with
    // access    to the connection.
    fn new(cx: &mut FunctionContext) -> NeonResult<Self> {
        // Drive's configuration
        let path_string = cx.argument::<JsString>(0)?.value(cx);
        let drive_config = cx.argument::<JsObject>(1)?;

        let js_data_contracts_cache_size: Handle<JsNumber> =
            drive_config.get(cx, "dataContractsGlobalCacheSize")?;
        let data_contracts_global_cache_size =
            u64::try_from(js_data_contracts_cache_size.value(cx) as i64).or_else(|_| {
                cx.throw_range_error("`dataContractsGlobalCacheSize` must fit in u64")
            })?;

        let js_data_contracts_transactional_cache_size: Handle<JsNumber> =
            drive_config.get(cx, "dataContractsTransactionalCacheSize")?;
        let data_contracts_transactional_cache_size =
            u64::try_from(js_data_contracts_transactional_cache_size.value(cx) as i64).or_else(
                |_| cx.throw_range_error("`dataContractsTransactionalCacheSize` must fit in u64"),
            )?;

        // Channel for sending callbacks to execute on the Drive connection thread
        let (tx, rx) = mpsc::channel::<PlatformWrapperMessage>();

        // Create an `Channel` for calling back to JavaScript. It is more efficient
        // to create a single channel and re-use it for all database callbacks.
        // The JavaScript process will not exit as long as this channel has not been
        // dropped.
        let channel = cx.channel();

        let sender = tx.clone();

        // Spawn a thread for processing database queries
        // This will not block the JavaScript main thread and will continue executing
        // concurrently.
        thread::spawn(move || {
            let path = Path::new(&path_string);
            // Open a connection to groveDb, this will be moved to a separate thread

            let drive_config = DriveConfig {
                data_contracts_global_cache_size,
                data_contracts_transactional_cache_size,
                ..Default::default()
            };

            // TODO: think how to pass this error to JS
            let platform: Platform = Platform::open(path, Some(drive_config)).unwrap();

            let mut transactions: HashMap<TransactionPointerAddress, Transaction> = HashMap::new();

            // Blocks until a callback is available
            // When the instance of `Database` is dropped, the channel will be closed
            // and `rx.recv()` will return an `Err`, ending the loop and terminating
            // the thread.
            while let Ok(message) = rx.recv() {
                match message {
                    PlatformWrapperMessage::Callback(callback) => {
                        // The connection and channel are owned by the thread, but _lent_ to
                        // the callback. The callback has exclusive access to the connection
                        // for the duration of the callback.
                        callback(&platform, &transactions, &channel);
                    }
                    // Immediately close the connection, even if there are pending messages
                    PlatformWrapperMessage::Close(callback) => {
                        drop(transactions);
                        drop(platform);

                        callback(&channel);
                        break;
                    }
                    // Flush message
                    PlatformWrapperMessage::Flush(callback) => {
                        platform.drive.grove.flush().unwrap();
                        callback(&channel);
                    }
                    PlatformWrapperMessage::StartTransaction(callback) => {
                        let transaction = platform.drive.grove.start_transaction();

                        let transaction_ref = &transaction;
                        let transaction_raw_pointer = transaction_ref as *const Transaction;
                        let transaction_raw_pointer_address =
                            transaction_raw_pointer as TransactionPointerAddress;

                        transactions.insert(transaction_raw_pointer_address, transaction);

                        callback(sender.clone(), transaction_raw_pointer_address, &channel);
                    }
                    PlatformWrapperMessage::CommitTransaction(
                        transaction_raw_pointer_address,
                        callback,
                    ) => {
                        let error = if let Some(transaction) =
                            transactions.remove(&transaction_raw_pointer_address)
                        {
                            let mut drive_cache = platform.drive.cache.borrow_mut();

                            drive_cache
                                .cached_contracts
                                .merge_transactional_cache(&transaction);

                            drive_cache
                                .cached_contracts
                                .clear_transactional_cache(&transaction);

                            platform.drive.commit_transaction(transaction).unwrap();

                            Ok(())
                        } else {
                            Err("invalid transaction_raw_pointer_address, transaction was not found".to_string())
                        };

                        callback(&channel, error);
                    }
                    PlatformWrapperMessage::RollbackTransaction(
                        transaction_raw_pointer_address,
                        callback,
                    ) => {
                        let error = if let Some(transaction) =
                            transactions.remove(&transaction_raw_pointer_address)
                        {
                            let mut drive_cache = platform.drive.cache.borrow_mut();

                            drive_cache
                                .cached_contracts
                                .clear_transactional_cache(&transaction);

                            platform.drive.rollback_transaction(&transaction).unwrap();

                            Ok(())
                        } else {
                            Err("invalid transaction_raw_pointer_address, transaction was not found".to_string())
                        };

                        callback(&channel, error);
                    }
                    PlatformWrapperMessage::AbortTransaction(
                        transaction_raw_pointer_address,
                        callback,
                    ) => {
                        let error = if let Some(transaction) =
                            transactions.remove(&transaction_raw_pointer_address)
                        {
                            let mut drive_cache = platform.drive.cache.borrow_mut();

                            drive_cache
                                .cached_contracts
                                .clear_transactional_cache(&transaction);

                            drop(transaction);

                            Ok(())
                        } else {
                            Err("invalid transaction_raw_pointer_address, transaction was not found".to_string())
                        };

                        callback(&channel, error);
                    }
                }
            }
        });

        Ok(Self { tx })
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a
    // closed database
    fn close(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::Close(Box::new(callback)))
    }

    fn send_to_drive_thread(
        &self,
        callback: impl for<'a> FnOnce(&'a Platform, &HashMap<TransactionPointerAddress, Transaction>, &Channel)
            + Send
            + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::Callback(Box::new(callback)))
    }

    fn start_transaction(
        &self,
        callback: impl FnOnce(mpsc::Sender<PlatformWrapperMessage>, usize, &Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::StartTransaction(Box::new(callback)))
    }

    fn commit_transaction(
        &self,
        transaction_raw_pointer_address: TransactionPointerAddress,
        callback: impl FnOnce(&Channel, Result<(), String>) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx.send(PlatformWrapperMessage::CommitTransaction(
            transaction_raw_pointer_address,
            Box::new(callback),
        ))
    }

    fn rollback_transaction(
        &self,
        transaction_raw_pointer_address: TransactionPointerAddress,
        callback: impl FnOnce(&Channel, Result<(), String>) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx.send(PlatformWrapperMessage::RollbackTransaction(
            transaction_raw_pointer_address,
            Box::new(callback),
        ))
    }

    fn abort_transaction(
        &self,
        transaction_raw_pointer_address: TransactionPointerAddress,
        callback: impl FnOnce(&Channel, Result<(), String>) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx.send(PlatformWrapperMessage::AbortTransaction(
            transaction_raw_pointer_address,
            Box::new(callback),
        ))
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a
    // closed database
    fn flush(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::Flush(Box::new(callback)))
    }
}

// Ensures that DriveWrapper is properly disposed when the corresponding JS
// object gets garbage collected
impl Finalize for PlatformWrapper {}

// External wrapper logic
impl PlatformWrapper {
    // Create a new instance of `Drive` and place it inside a `JsBox`
    // JavaScript can hold a reference to a `JsBox`, but the contents are opaque
    fn js_open(mut cx: FunctionContext) -> JsResult<JsBox<PlatformWrapper>> {
        let drive_wrapper =
            PlatformWrapper::new(&mut cx).or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.boxed(drive_wrapper))
    }

    /// Sends a message to the DB thread to stop the thread and dispose the
    /// groveDb instance owned by it, then calls js callback passed as a first
    /// argument to the function
    fn js_close(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        drive
            .close(|channel| {
                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> =
                        vec![task_context.null().upcast()];

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_create_initial_state_structure(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_transaction = cx.argument::<JsValue>(0)?;

        let maybe_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let execution_result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .create_initial_state_structure(transaction_arg)
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> = match execution_result {
                        Ok(_) => vec![task_context.null().upcast()],
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_fetch_contract(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_contract_id = cx.argument::<JsBuffer>(0)?;
        let js_maybe_epoch_index = cx.argument::<JsValue>(1)?;
        let js_transaction = cx.argument::<JsValue>(2)?;
        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;

        let maybe_epoch: Option<Epoch> = if !js_maybe_epoch_index.is_a::<JsUndefined, _>(&mut cx) {
            let js_epoch_index = js_maybe_epoch_index.downcast_or_throw::<JsNumber, _>(&mut cx)?;

            let epoch_index = u16::try_from(js_epoch_index.value(&mut cx) as i64)
                .or_else(|_| cx.throw_range_error("`epochs` must fit in u16"))?;

            let epoch = Epoch::new(epoch_index);

            Some(epoch)
        } else {
            None
        };

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.get_contract_with_fetch_info(
                        contract_id,
                        maybe_epoch.as_ref(),
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(maybe_contract_fetch_info) => {
                            let js_result = task_context.empty_array();

                            if let Some(contract_fetch_info) = maybe_contract_fetch_info {
                                let contract_cbor =
                                    contract_fetch_info.contract.to_buffer().or_else(|_| {
                                        task_context.throw_range_error("can't serialize contract")
                                    })?;

                                let contract_buffer =
                                    JsBuffer::external(&mut task_context, contract_cbor);

                                js_result.set(&mut task_context, 0, contract_buffer)?;

                                if let Some(fee_result) = &contract_fetch_info.fee {
                                    let js_fee_result = converter::fee_result_to_js_object(
                                        &mut task_context,
                                        fee_result.clone(),
                                    )?;

                                    js_result.set(&mut task_context, 1, js_fee_result)?;
                                }
                            }

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_create_contract(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_contract_cbor = cx.argument::<JsBuffer>(0)?;
        let js_block_info = cx.argument::<JsObject>(1)?;
        let js_apply = cx.argument::<JsBoolean>(2)?;
        let js_transaction = cx.argument::<JsValue>(3)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let apply = js_apply.value(&mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.insert_contract_cbor(
                        contract_cbor,
                        None,
                        block_info,
                        apply,
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                converter::fee_result_to_js_object(&mut task_context, fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_update_contract(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_contract_cbor = cx.argument::<JsBuffer>(0)?;
        let js_block_info = cx.argument::<JsObject>(1)?;
        let js_apply = cx.argument::<JsBoolean>(2)?;
        let js_transaction = cx.argument::<JsValue>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let apply = js_apply.value(&mut cx);

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.update_contract_cbor(
                        contract_cbor,
                        None,
                        block_info,
                        apply,
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                converter::fee_result_to_js_object(&mut task_context, fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_create_document(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_owner_id = cx.argument::<JsBuffer>(3)?;
        let js_override_document = cx.argument::<JsBoolean>(4)?;
        let js_block_info = cx.argument::<JsObject>(5)?;
        let js_apply = cx.argument::<JsBoolean>(6)?;
        let js_transaction = cx.argument::<JsValue>(7)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(8)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let document_cbor = converter::js_buffer_to_vec_u8(js_document_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);
        let owner_id = converter::js_buffer_to_identifier(&mut cx, js_owner_id)?;
        let override_document = js_override_document.value(&mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let storage_flags =
                    StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.add_serialized_document_for_contract_id(
                        &document_cbor,
                        contract_id,
                        &document_type_name,
                        Some(owner_id),
                        override_document,
                        block_info,
                        apply,
                        Some(storage_flags).as_ref(),
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                converter::fee_result_to_js_object(&mut task_context, fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_update_document(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_owner_id = cx.argument::<JsBuffer>(3)?;
        let js_block_info = cx.argument::<JsObject>(4)?;
        let js_apply = cx.argument::<JsBoolean>(5)?;
        let js_transaction = cx.argument::<JsValue>(6)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(7)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let document_cbor = converter::js_buffer_to_vec_u8(js_document_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);
        let owner_id = converter::js_buffer_to_identifier(&mut cx, js_owner_id)?;
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let storage_flags =
                    StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.update_document_for_contract_id(
                        &document_cbor,
                        contract_id,
                        &document_type_name,
                        Some(owner_id),
                        block_info,
                        apply,
                        Some(storage_flags).as_ref(),
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                converter::fee_result_to_js_object(&mut task_context, fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_delete_document(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_id = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_block_info = cx.argument::<JsObject>(3)?;
        let js_apply = cx.argument::<JsBoolean>(4)?;
        let js_transaction = cx.argument::<JsValue>(5)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(6)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let document_id = converter::js_buffer_to_identifier(&mut cx, js_document_id)?;
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.delete_document_for_contract_id(
                        document_id,
                        contract_id,
                        &document_type_name,
                        None,
                        block_info,
                        apply,
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                converter::fee_result_to_js_object(&mut task_context, fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_insert_identity_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_cbor = cx.argument::<JsBuffer>(0)?;
        let js_block_info = cx.argument::<JsObject>(1)?;
        let js_apply = cx.argument::<JsBoolean>(2)?;
        let js_transaction = cx.argument::<JsValue>(3)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_cbor = converter::js_buffer_to_vec_u8(js_identity_cbor, &mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);

        let identity =
            Identity::from_buffer(identity_cbor).or_else(|e| cx.throw_error(e.to_string()))?;

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let storage_flags = StorageFlags::new_single_epoch(
                    block_info.epoch.index,
                    Some(identity.id.to_buffer()),
                );

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.insert_identity(
                        identity,
                        block_info,
                        apply,
                        Some(storage_flags).as_ref(),
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                converter::fee_result_to_js_object(&mut task_context, fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_query_documents(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_query_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_maybe_epoch_index = cx.argument::<JsValue>(3)?;
        // TODO We need dry run for validation
        let js_transaction = cx.argument::<JsValue>(4)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(5)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let query_cbor = converter::js_buffer_to_vec_u8(js_query_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);

        let maybe_epoch: Option<Epoch> = if !js_maybe_epoch_index.is_a::<JsUndefined, _>(&mut cx) {
            let js_epoch_index = js_maybe_epoch_index.downcast_or_throw::<JsNumber, _>(&mut cx)?;

            let epoch_index = u16::try_from(js_epoch_index.value(&mut cx) as i64)
                .or_else(|_| cx.throw_range_error("`epochs` must fit in u16"))?;

            let epoch = Epoch::new(epoch_index);

            Some(epoch)
        } else {
            None
        };

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.query_documents(
                        &query_cbor,
                        contract_id,
                        document_type_name.as_str(),
                        maybe_epoch.as_ref(),
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((value, skipped, cost)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();
                            let js_vecs = converter::nested_vecs_to_js(&mut task_context, value)?;
                            let js_num = task_context.number(skipped).upcast::<JsValue>();
                            let js_cost = task_context.number(cost as f64).upcast::<JsValue>();

                            js_array.set(&mut task_context, 0, js_vecs)?;
                            js_array.set(&mut task_context, 1, js_num)?;
                            js_array.set(&mut task_context, 2, js_cost)?;

                            vec![task_context.null().upcast(), js_array.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_prove_documents_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_query_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_transaction = cx.argument::<JsValue>(3)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let query_cbor = converter::js_buffer_to_vec_u8(js_query_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transactions, channel| {
                let transaction_result: Result<Option<&Transaction>, Error> =
                    match maybe_boxed_transaction_address {
                        Some(address) => transactions
                            .get(&address)
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "invalid transaction pointer address",
                            )))
                            .map(Some),
                        None => Ok(None),
                    };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform.drive.query_documents_as_grove_proof(
                        &query_cbor,
                        contract_id,
                        document_type_name.as_str(),
                        None,
                        None,
                        transaction_arg,
                    )
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((proof, processing_cost)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();
                            let js_buffer = JsBuffer::external(&mut task_context, proof);
                            let js_processing_cost = task_context.number(processing_cost as f64);

                            js_array.set(&mut task_context, 0, js_buffer)?;
                            js_array.set(&mut task_context, 1, js_processing_cost)?;

                            vec![task_context.null().upcast(), js_array.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_start_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.start_transaction(|tx, transaction_raw_pointer_address, channel| {
            let transaction_address = PlatformWrapperTransactionAddress {
                address: transaction_raw_pointer_address,
                tx,
            };

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = vec![
                    task_context.null().upcast(),
                    task_context.boxed(transaction_address).upcast(),
                ];

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_commit_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_transaction = cx.argument::<JsValue>(0)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let transaction_address =
            maybe_boxed_transaction_address.expect("transaction address should be available");

        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.commit_transaction(transaction_address, |channel, maybe_error| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match maybe_error {
                    Ok(_) => vec![task_context.null().upcast()],
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_rollback_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_transaction = cx.argument::<JsValue>(0)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let transaction_address =
            maybe_boxed_transaction_address.expect("transaction address should be available");

        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.rollback_transaction(transaction_address, |channel, maybe_error| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match maybe_error {
                    Ok(_) => vec![task_context.null().upcast()],
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_abort_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_transaction = cx.argument::<JsValue>(0)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let transaction_address =
            maybe_boxed_transaction_address.expect("transaction address should be available");

        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.abort_transaction(transaction_address, |channel, maybe_error| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match maybe_error {
                    Ok(_) => vec![task_context.null().upcast()],
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_get(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;
        let js_transaction = cx.argument::<JsValue>(2)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;
            let path_slice = path.iter().map(|fragment| fragment.as_slice());
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .get(path_slice, &key, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(element) => {
                        // First parameter of JS callbacks is error, which is null in this case
                        vec![
                            task_context.null().upcast(),
                            converter::element_to_js_object(&mut task_context, element)?,
                        ]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_insert(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;
        let js_element = cx.argument::<JsObject>(2)?;
        let js_transaction = cx.argument::<JsValue>(3)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let element = converter::js_object_to_element(&mut cx, js_element)?;

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;
            let path_slice = path.iter().map(|fragment| fragment.as_slice());
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .insert(path_slice, &key, element, None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(_) => vec![task_context.null().upcast()],
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;
                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_insert_if_not_exists(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;
        let js_element = cx.argument::<JsObject>(2)?;
        let js_transaction = cx.argument::<JsValue>(3)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let element = converter::js_object_to_element(&mut cx, js_element)?;

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let path_slice: Vec<&[u8]> = path.iter().map(|fragment| fragment.as_slice()).collect();
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .insert_if_not_exists(path_slice, key.as_slice(), element, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(is_inserted) => vec![
                        task_context.null().upcast(),
                        task_context
                            .boolean(is_inserted)
                            .as_value(&mut task_context),
                    ],
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_put_aux(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_key = cx.argument::<JsBuffer>(0)?;
        let js_value = cx.argument::<JsBuffer>(1)?;
        let js_transaction = cx.argument::<JsValue>(2)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let value = converter::js_buffer_to_vec_u8(js_value, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .put_aux(&key, &value, None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(()) => {
                        vec![task_context.null().upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_delete_aux(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_key = cx.argument::<JsBuffer>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .delete_aux(&key, None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(()) => {
                        vec![task_context.null().upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_get_aux(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_key = cx.argument::<JsBuffer>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .get_aux(&key, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(value) => {
                        if let Some(value) = value {
                            vec![
                                task_context.null().upcast(),
                                JsBuffer::external(&mut task_context, value).upcast(),
                            ]
                        } else {
                            vec![task_context.null().upcast(), task_context.null().upcast()]
                        }
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_query = cx.argument::<JsObject>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let path_query = converter::js_path_query_to_path_query(js_path_query, &mut cx)?;

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .query(&path_query, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok((value, skipped)) => {
                        let js_array: Handle<JsArray> = task_context.empty_array();
                        let js_vecs = converter::nested_vecs_to_js(&mut task_context, value)?;
                        let js_num = task_context.number(skipped).upcast::<JsValue>();
                        js_array.set(&mut task_context, 0, js_vecs)?;
                        js_array.set(&mut task_context, 1, js_num)?;

                        vec![task_context.null().upcast(), js_array.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_prove_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_query = cx.argument::<JsObject>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let path_query = converter::js_path_query_to_path_query(js_path_query, &mut cx)?;

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .get_proved_path_query(&path_query, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(proof) => {
                        let js_buffer = JsBuffer::external(&mut task_context, proof);
                        let js_value = js_buffer.as_value(&mut task_context);

                        vec![task_context.null().upcast(), js_value.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_prove_query_many(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_queries = cx.argument::<JsArray>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        if maybe_boxed_transaction_address.is_none() {
            cx.throw_type_error("transaction address is undefined")?;
        }

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let js_path_queries = js_path_queries.to_vec(&mut cx)?;
        let mut path_queries: Vec<PathQuery> = Vec::with_capacity(js_path_queries.len());

        for js_path_query in js_path_queries {
            let js_path_query = js_path_query.downcast_or_throw::<JsObject, _>(&mut cx)?;
            path_queries.push(converter::js_path_query_to_path_query(
                js_path_query,
                &mut cx,
            )?);
        }

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, _, channel| {
            let grove_db = &platform.drive.grove;

            let path_queries = path_queries.iter().collect();

            let result = grove_db.prove_query_many(path_queries).unwrap();

            channel.send(move |mut task_context| {
                let this = task_context.undefined();
                let callback = js_callback.into_inner(&mut task_context);

                let callback_arguments = match result {
                    Ok(proof) => {
                        let js_buffer = JsBuffer::external(&mut task_context, proof);
                        let js_value = js_buffer.as_value(&mut task_context);

                        vec![task_context.null().upcast(), js_value.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    /// Flush data on disc and then calls js callback passed as a first
    /// argument to the function
    fn js_grove_db_flush(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.flush(|channel| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = vec![task_context.null().upcast()];

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    /// Returns root hash or empty buffer
    fn js_grove_db_root_hash(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_transaction = cx.argument::<JsValue>(0)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .root_hash(transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(hash) => vec![
                        task_context.null().upcast(),
                        JsBuffer::external(&mut task_context, hash).upcast(),
                    ],
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_grove_db_delete(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;

        let js_transaction = cx.argument::<JsValue>(2)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let grove_db = &platform.drive.grove;

            let path_slice: Vec<&[u8]> = path.iter().map(|fragment| fragment.as_slice()).collect();
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .delete(path_slice, key.as_slice(), None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(()) => {
                        vec![task_context.null().upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_abci_init_chain(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_request = cx.argument::<JsBuffer>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let request_bytes = converter::js_buffer_to_vec_u8(js_request, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let result =
                transaction_result
                    .map_err(|e| e.to_string())
                    .and_then(|transaction_arg| {
                        InitChainRequest::from_bytes(&request_bytes)
                            .and_then(|request| platform.init_chain(request, transaction_arg))
                            .and_then(|response| response.to_bytes())
                            .map_err(|e| e.to_string())
                    });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(response_bytes) => {
                        let value = JsBuffer::external(&mut task_context, response_bytes);

                        vec![task_context.null().upcast(), value.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_abci_block_begin(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_request = cx.argument::<JsBuffer>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let request_bytes = converter::js_buffer_to_vec_u8(js_request, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let result =
                transaction_result
                    .map_err(|e| e.to_string())
                    .and_then(|transaction_arg| {
                        BlockBeginRequest::from_bytes(&request_bytes)
                            .and_then(|request| platform.block_begin(request, transaction_arg))
                            .and_then(|response| response.to_bytes())
                            .map_err(|e| e.to_string())
                    });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(response_bytes) => {
                        let value = JsBuffer::external(&mut task_context, response_bytes);

                        vec![task_context.null().upcast(), value.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_abci_block_end(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_request = cx.argument::<JsBuffer>(0)?;
        let js_transaction = cx.argument::<JsValue>(1)?;

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let request_bytes = converter::js_buffer_to_vec_u8(js_request, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let result =
                transaction_result
                    .map_err(|e| e.to_string())
                    .and_then(|transaction_arg| {
                        BlockEndRequest::from_bytes(&request_bytes)
                            .and_then(|request| platform.block_end(request, transaction_arg))
                            .and_then(|response| response.to_bytes())
                            .map_err(|e| e.to_string())
                    });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(response_bytes) => {
                        let value = JsBuffer::external(&mut task_context, response_bytes);

                        vec![task_context.null().upcast(), value.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_abci_after_finalize_block(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_request = cx.argument::<JsBuffer>(0)?;
        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let request_bytes = converter::js_buffer_to_vec_u8(js_request, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, _, channel| {
            let result = AfterFinalizeBlockRequest::from_bytes(&request_bytes)
                .and_then(|request| platform.after_finalize_block(request))
                .and_then(|response| response.to_bytes());

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(response_bytes) => {
                        let value = JsBuffer::external(&mut task_context, response_bytes);

                        vec![task_context.null().upcast(), value.upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_fetch_latest_withdrawal_transaction_index(
        mut cx: FunctionContext,
    ) -> JsResult<JsUndefined> {
        let js_transaction = cx.argument::<JsValue>(0)?;
        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let maybe_boxed_transaction_address = if !js_transaction.is_a::<JsUndefined, _>(&mut cx) {
            let handle = js_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let result = transaction_result.and_then(|transaction_arg| {
                platform
                    .drive
                    .fetch_latest_withdrawal_transaction_index(transaction_arg)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(index) => {
                        let index_f64 = index as f64;
                        if index_f64 as u64 != index {
                            vec![task_context
                                .error("could not convert withdrawal transaction index to f64")?
                                .upcast()]
                        } else {
                            let value = JsNumber::new(&mut task_context, index_f64);

                            vec![task_context.null().upcast(), value.upcast()]
                        }
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }

    fn js_enqueue_withdrawal_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_index = cx.argument::<JsNumber>(0)?;
        let js_core_transaction = cx.argument::<JsBuffer>(1)?;
        let js_db_transaction = cx.argument::<JsValue>(2)?;
        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let maybe_boxed_transaction_address = if !js_db_transaction.is_a::<JsUndefined, _>(&mut cx)
        {
            let handle = js_db_transaction
                .downcast_or_throw::<JsBox<PlatformWrapperTransactionAddress>, _>(&mut cx)?;

            Some(***handle)
        } else {
            None
        };

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let index = js_index.value(&mut cx);
        let transaction_bytes = converter::js_buffer_to_vec_u8(js_core_transaction, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transactions, channel| {
            let transaction_result: Result<Option<&Transaction>, Error> =
                match maybe_boxed_transaction_address {
                    Some(address) => transactions
                        .get(&address)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "invalid transaction pointer address",
                        )))
                        .map(Some),
                    None => Ok(None),
                };

            let mut batch = GroveDbOpBatch::new();

            let index_bytes = (index as u64).to_be_bytes().to_vec();

            let withdrawals = vec![(index_bytes.clone(), transaction_bytes)];

            platform
                .drive
                .add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawals);

            platform
                .drive
                .add_update_withdrawal_index_counter_operation(&mut batch, index_bytes);

            let result = transaction_result.and_then(|transaction_arg| {
                platform
                    .drive
                    .grove_apply_batch(batch, false, transaction_arg)
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(_) => {
                        vec![task_context.null().upcast(), task_context.null().upcast()]
                    }

                    // Convert the error to a JavaScript exception on failure
                    Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("driveOpen", PlatformWrapper::js_open)?;
    cx.export_function("driveClose", PlatformWrapper::js_close)?;
    cx.export_function(
        "driveCreateInitialStateStructure",
        PlatformWrapper::js_create_initial_state_structure,
    )?;
    cx.export_function("driveFetchContract", PlatformWrapper::js_fetch_contract)?;
    cx.export_function("driveCreateContract", PlatformWrapper::js_create_contract)?;
    cx.export_function("driveUpdateContract", PlatformWrapper::js_update_contract)?;
    cx.export_function("driveCreateDocument", PlatformWrapper::js_create_document)?;
    cx.export_function("driveUpdateDocument", PlatformWrapper::js_update_document)?;
    cx.export_function("driveDeleteDocument", PlatformWrapper::js_delete_document)?;
    cx.export_function(
        "driveInsertIdentity",
        PlatformWrapper::js_insert_identity_cbor,
    )?;
    cx.export_function("driveQueryDocuments", PlatformWrapper::js_query_documents)?;

    cx.export_function(
        "driveProveDocumentsQuery",
        PlatformWrapper::js_prove_documents_query,
    )?;

    cx.export_function(
        "driveFetchLatestWithdrawalTransactionIndex",
        PlatformWrapper::js_fetch_latest_withdrawal_transaction_index,
    )?;
    cx.export_function(
        "driveEnqueueWithdrawalTransaction",
        PlatformWrapper::js_enqueue_withdrawal_transaction,
    )?;

    cx.export_function("groveDbInsert", PlatformWrapper::js_grove_db_insert)?;
    cx.export_function(
        "groveDbInsertIfNotExists",
        PlatformWrapper::js_grove_db_insert_if_not_exists,
    )?;
    cx.export_function("groveDbGet", PlatformWrapper::js_grove_db_get)?;
    cx.export_function("groveDbDelete", PlatformWrapper::js_grove_db_delete)?;
    cx.export_function("groveDbFlush", PlatformWrapper::js_grove_db_flush)?;
    cx.export_function(
        "groveDbStartTransaction",
        PlatformWrapper::js_grove_db_start_transaction,
    )?;
    cx.export_function(
        "groveDbCommitTransaction",
        PlatformWrapper::js_grove_db_commit_transaction,
    )?;
    cx.export_function(
        "groveDbRollbackTransaction",
        PlatformWrapper::js_grove_db_rollback_transaction,
    )?;
    cx.export_function(
        "groveDbAbortTransaction",
        PlatformWrapper::js_grove_db_abort_transaction,
    )?;
    cx.export_function("groveDbPutAux", PlatformWrapper::js_grove_db_put_aux)?;
    cx.export_function("groveDbDeleteAux", PlatformWrapper::js_grove_db_delete_aux)?;
    cx.export_function("groveDbGetAux", PlatformWrapper::js_grove_db_get_aux)?;
    cx.export_function("groveDbQuery", PlatformWrapper::js_grove_db_query)?;
    cx.export_function(
        "groveDbProveQuery",
        PlatformWrapper::js_grove_db_prove_query,
    )?;
    cx.export_function(
        "groveDbProveQueryMany",
        PlatformWrapper::js_grove_db_prove_query_many,
    )?;
    cx.export_function("groveDbRootHash", PlatformWrapper::js_grove_db_root_hash)?;

    cx.export_function("abciInitChain", PlatformWrapper::js_abci_init_chain)?;
    cx.export_function("abciBlockBegin", PlatformWrapper::js_abci_block_begin)?;
    cx.export_function("abciBlockEnd", PlatformWrapper::js_abci_block_end)?;
    cx.export_function(
        "abciAfterFinalizeBlock",
        PlatformWrapper::js_abci_after_finalize_block,
    )?;

    Ok(())
}
