mod converter;
mod fee;

use std::num::ParseIntError;
use std::{option::Option::None, path::Path, sync::mpsc, thread};

use crate::fee::result::FeeResultWrapper;
use drive::dpp::identity::{KeyID, TimestampMillis};
use drive::dpp::prelude::Revision;
use drive::drive::batch::GroveDbOpBatch;
use drive::drive::config::DriveConfig;
use drive::drive::flags::StorageFlags;
use drive::error::Error;
use drive::fee::credits::Credits;
use drive::fee::epoch::CreditsPerEpoch;
use drive::fee_pools::epochs::Epoch;
use drive::grovedb::{PathQuery, Transaction};
use drive::query::TransactionArg;
use drive_abci::abci::handlers::TenderdashAbci;
use drive_abci::abci::messages::{
    AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees, InitChainRequest,
    Serializable,
};
use drive_abci::platform::Platform;
use fee::js_calculate_storage_fee_distribution_amount_and_leftovers;
use neon::prelude::*;

type PlatformCallback = Box<dyn for<'a> FnOnce(&'a Platform, TransactionArg, &Channel) + Send>;
type UnitCallback = Box<dyn FnOnce(&Channel) + Send>;
type ErrorCallback = Box<dyn FnOnce(&Channel, Result<(), String>) + Send>;
type TransactionCallback =
    Box<dyn FnOnce(mpsc::Sender<PlatformWrapperMessage>, Result<(), String>, &Channel) + Send>;

// Messages sent on the drive channel
enum PlatformWrapperMessage {
    // Callback to be executed
    Callback(PlatformCallback),
    // Indicates that the thread should be stopped and connection closed
    Close(UnitCallback),
    StartTransaction(TransactionCallback),
    CommitTransaction(ErrorCallback),
    RollbackTransaction(ErrorCallback),
    AbortTransaction(ErrorCallback),
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
            drive_config.get(cx, "dataContractsBlockCacheSize")?;
        let data_contracts_block_cache_size = u64::try_from(
            js_data_contracts_transactional_cache_size.value(cx) as i64,
        )
        .or_else(|_| cx.throw_range_error("`dataContractsBlockCacheSize` must fit in u64"))?;

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
                data_contracts_block_cache_size,
                ..Default::default()
            };

            // TODO: think how to pass this error to JS
            let platform: Platform = Platform::open(path, Some(drive_config)).unwrap();

            let mut maybe_transaction: Option<Transaction> = None;

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
                        callback(&platform, maybe_transaction.as_ref(), &channel);
                    }
                    // Immediately close the connection, even if there are pending messages
                    PlatformWrapperMessage::Close(callback) => {
                        drop(maybe_transaction);
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
                        let result = if maybe_transaction.is_some() {
                            Err("transaction is already started".to_string())
                        } else {
                            let transaction = platform.drive.grove.start_transaction();

                            maybe_transaction = Some(transaction);

                            Ok(())
                        };

                        callback(sender.clone(), result, &channel);
                    }
                    PlatformWrapperMessage::CommitTransaction(callback) => {
                        let result = if maybe_transaction.is_some() {
                            let mut drive_cache = platform.drive.cache.borrow_mut();

                            drive_cache.cached_contracts.merge_block_cache();

                            drive_cache.cached_contracts.clear_block_cache();

                            platform
                                .drive
                                .commit_transaction(maybe_transaction.take().unwrap())
                                .map_err(|err| err.to_string())
                        } else {
                            Err("transaction is not started".to_string())
                        };

                        callback(&channel, result);
                    }
                    PlatformWrapperMessage::RollbackTransaction(callback) => {
                        let result = if let Some(transaction) = &maybe_transaction {
                            let mut drive_cache = platform.drive.cache.borrow_mut();

                            drive_cache.cached_contracts.clear_block_cache();

                            platform
                                .drive
                                .rollback_transaction(transaction)
                                .map_err(|err| err.to_string())
                        } else {
                            Err("transaction is not started".to_string())
                        };

                        callback(&channel, result);
                    }
                    PlatformWrapperMessage::AbortTransaction(callback) => {
                        let result = if maybe_transaction.is_some() {
                            let mut drive_cache = platform.drive.cache.borrow_mut();

                            drive_cache.cached_contracts.clear_block_cache();

                            drop(maybe_transaction.take());

                            Ok(())
                        } else {
                            Err("transaction is not started".to_string())
                        };

                        callback(&channel, result);
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
        callback: impl for<'a> FnOnce(&'a Platform, TransactionArg, &Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::Callback(Box::new(callback)))
    }

    fn start_transaction(
        &self,
        callback: impl FnOnce(mpsc::Sender<PlatformWrapperMessage>, Result<(), String>, &Channel)
            + Send
            + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::StartTransaction(Box::new(callback)))
    }

    fn commit_transaction(
        &self,
        callback: impl FnOnce(&Channel, Result<(), String>) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::CommitTransaction(Box::new(
                callback,
            )))
    }

    fn rollback_transaction(
        &self,
        callback: impl FnOnce(&Channel, Result<(), String>) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::RollbackTransaction(Box::new(
                callback,
            )))
    }

    fn abort_transaction(
        &self,
        callback: impl FnOnce(&Channel, Result<(), String>) + Send + 'static,
    ) -> Result<(), mpsc::SendError<PlatformWrapperMessage>> {
        self.tx
            .send(PlatformWrapperMessage::AbortTransaction(Box::new(callback)))
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
        let js_using_transaction = cx.argument::<JsBoolean>(0)?;
        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let execution_result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .create_initial_state_structure(transaction_arg)
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> = match execution_result {
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

    fn js_fetch_contract(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_contract_id = cx.argument::<JsBuffer>(0)?;
        let js_maybe_epoch_index = cx.argument::<JsValue>(1)?;
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;
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

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .get_contract_with_fetch_info(
                            contract_id,
                            maybe_epoch.as_ref(),
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((maybe_fee_result, maybe_contract_fetch_info)) => {
                            let js_result = task_context.empty_array();

                            let js_contract: Handle<JsValue> = if let Some(contract_fetch_info) =
                                maybe_contract_fetch_info
                            {
                                let contract_cbor =
                                    contract_fetch_info.contract.to_buffer().or_else(|_| {
                                        task_context.throw_range_error("can't serialize contract")
                                    })?;

                                JsBuffer::external(&mut task_context, contract_cbor).upcast()
                            } else {
                                task_context.null().upcast()
                            };

                            js_result.set(&mut task_context, 0, js_contract)?;

                            if let Some(fee_result) = maybe_fee_result {
                                let js_fee_result =
                                    task_context.boxed(FeeResultWrapper::new(fee_result));

                                js_result.set(&mut task_context, 1, js_fee_result)?;
                            }

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let apply = js_apply.value(&mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .insert_contract_cbor(
                            contract_cbor,
                            None,
                            block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let apply = js_apply.value(&mut cx);

        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .update_contract_cbor(
                            contract_cbor,
                            None,
                            block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(7)?;
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
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let storage_flags =
                    StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .add_serialized_document_for_contract_id(
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
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(6)?;
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
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let storage_flags =
                    StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .update_document_for_contract_id(
                            &document_cbor,
                            contract_id,
                            &document_type_name,
                            Some(owner_id),
                            block_info,
                            apply,
                            Some(storage_flags).as_ref(),
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(5)?;
        let js_callback = cx.argument::<JsFunction>(6)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let document_id = converter::js_buffer_to_identifier(&mut cx, js_document_id)?;
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .delete_document_for_contract_id(
                            document_id,
                            contract_id,
                            &document_type_name,
                            None,
                            block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_cbor = converter::js_buffer_to_vec_u8(js_identity_cbor, &mut cx);
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .add_new_identity_from_cbor_encoded_bytes(
                            identity_cbor,
                            &block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_fetch_identity(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;
        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .fetch_full_identity(identity_id, transaction_arg)
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(maybe_identity) => {
                            if let Some(identity) = maybe_identity {
                                match identity.to_buffer() {
                                    Ok(serialized_identity) => {
                                        let js_serialized_identity = JsBuffer::external(
                                            &mut task_context,
                                            serialized_identity,
                                        );

                                        vec![
                                            task_context.null().upcast(),
                                            js_serialized_identity.upcast(),
                                        ]
                                    }
                                    Err(e) => {
                                        let err_message =
                                            format!("can't serialise identities: {}", e);

                                        vec![task_context.error(err_message)?.upcast()]
                                    }
                                }
                            } else {
                                vec![task_context.null().upcast(), task_context.null().upcast()]
                            }
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_fetch_identity_with_costs(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_epoch_index = cx.argument::<JsNumber>(1)?;
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;
        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;

        let epoch_index = u16::try_from(js_epoch_index.value(&mut cx) as i64)
            .or_else(|_| cx.throw_range_error("`epochs` must fit in u16"))?;

        let epoch = Epoch::new(epoch_index);

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .fetch_full_identity_with_costs(identity_id, &epoch, transaction_arg)
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((maybe_identity, fee_result)) => {
                            let js_result = task_context.empty_array();

                            let js_identity: Handle<JsValue> = if let Some(identity) =
                                maybe_identity
                            {
                                let serialized_identity = identity.to_buffer().or_else(|e| {
                                    task_context
                                        .throw_error(format!("can't serialize identity: {}", e))
                                })?;

                                JsBuffer::external(&mut task_context, serialized_identity).upcast()
                            } else {
                                task_context.null().upcast()
                            };

                            js_result.set(&mut task_context, 0, js_identity)?;

                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            js_result.set(&mut task_context, 1, js_fee_result)?;

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_add_to_identity_balance(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_balance_to_add = cx.argument::<JsNumber>(1)?;
        let js_block_info = cx.argument::<JsObject>(2)?;
        let js_apply = cx.argument::<JsBoolean>(3)?;
        let js_using_transaction = cx.argument::<JsBoolean>(4)?;
        let js_callback = cx.argument::<JsFunction>(5)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;
        let balance_to_add = js_balance_to_add.value(&mut cx) as u64;
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .add_to_identity_balance(
                            identity_id,
                            balance_to_add,
                            &block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_remove_from_identity_balance(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_required_balance_to_remove = cx.argument::<JsNumber>(1)?;
        let js_desired_balance_to_remove = cx.argument::<JsNumber>(2)?;
        let js_block_info = cx.argument::<JsObject>(3)?;
        let js_apply = cx.argument::<JsBoolean>(4)?;
        let js_using_transaction = cx.argument::<JsBoolean>(5)?;
        let js_callback = cx.argument::<JsFunction>(6)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;
        let required_balance_to_remove = js_required_balance_to_remove.value(&mut cx) as u64;
        let desired_balance_to_remove = js_desired_balance_to_remove.value(&mut cx) as u64;
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .remove_from_identity_balance(
                            identity_id,
                            required_balance_to_remove,
                            desired_balance_to_remove,
                            &block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_add_keys_to_identity(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_keys_to_add = cx.argument::<JsArray>(1)?;
        let js_block_info = cx.argument::<JsObject>(2)?;
        let js_apply = cx.argument::<JsBoolean>(3)?;
        let js_using_transaction = cx.argument::<JsBoolean>(4)?;
        let js_callback = cx.argument::<JsFunction>(5)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;
        let keys_to_add = converter::js_array_to_keys(js_keys_to_add, &mut cx)?;
        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .add_new_keys_to_identity(
                            identity_id,
                            keys_to_add,
                            &block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_disable_identity_keys(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_key_ids = cx.argument::<JsArray>(1)?;
        let js_disable_at = cx.argument::<JsNumber>(2)?;
        let js_block_info = cx.argument::<JsObject>(3)?;
        let js_apply = cx.argument::<JsBoolean>(4)?;
        let js_using_transaction = cx.argument::<JsBoolean>(5)?;
        let js_callback = cx.argument::<JsFunction>(6)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;

        let key_ids = js_key_ids
            .to_vec(&mut cx)?
            .into_iter()
            .map(|js_value| {
                let js_key = js_value.downcast_or_throw::<JsNumber, _>(&mut cx)?;
                let key = KeyID::try_from(js_key.value(&mut cx) as u64)
                    .or_else(|_| cx.throw_range_error("key id must be u32"))?;

                Ok(key)
            })
            .collect::<Result<Vec<KeyID>, _>>()?;

        let disabled_at = js_disable_at.value(&mut cx) as TimestampMillis;

        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .disable_identity_keys(
                            identity_id,
                            key_ids,
                            disabled_at,
                            &block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
                    };

                    callback.call(&mut task_context, this, callback_arguments)?;

                    Ok(())
                });
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_update_identity_revision(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_revision = cx.argument::<JsNumber>(1)?;
        let js_block_info = cx.argument::<JsObject>(2)?;
        let js_apply = cx.argument::<JsBoolean>(3)?;
        let js_using_transaction = cx.argument::<JsBoolean>(4)?;
        let js_callback = cx.argument::<JsFunction>(5)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_identifier(&mut cx, js_identity_id)?;

        let revision = js_revision.value(&mut cx) as Revision;

        let block_info = converter::js_object_to_block_info(js_block_info, &mut cx)?;
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .update_identity_revision(
                            identity_id,
                            revision,
                            &block_info,
                            apply,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
                });

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(fee_result) => {
                            let js_fee_result =
                                task_context.boxed(FeeResultWrapper::new(fee_result));

                            // First parameter of JS callbacks is error, which is null in this case
                            vec![task_context.null().upcast(), js_fee_result.upcast()]
                        }

                        // Convert the error to a JavaScript exception on failure
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(4)?;
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

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .query_documents(
                            &query_cbor,
                            contract_id,
                            document_type_name.as_str(),
                            maybe_epoch.as_ref(),
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
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
                        Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let query_cbor = converter::js_buffer_to_vec_u8(js_query_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_identifier(&mut cx, js_contract_id)?;
        let document_type_name = js_document_type_name.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |platform: &Platform, transaction, channel| {
                let transaction_result = if using_transaction {
                    if transaction.is_none() {
                        Err("transaction is not started".to_string())
                    } else {
                        Ok(transaction)
                    }
                } else {
                    Ok(None)
                };

                let result = transaction_result.and_then(|transaction_arg| {
                    platform
                        .drive
                        .query_documents_as_grove_proof(
                            &query_cbor,
                            contract_id,
                            document_type_name.as_str(),
                            None,
                            None,
                            transaction_arg,
                        )
                        .map_err(|err| err.to_string())
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
                        Err(err) => vec![task_context.error(err)?.upcast()],
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

        db.start_transaction(|_, result, channel| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(_) => {
                        vec![task_context.null().upcast()]
                    }
                    Err(err) => vec![task_context.error(err)?.upcast()],
                };

                callback.call(&mut task_context, this, callback_arguments)?;

                Ok(())
            });
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_grove_db_commit_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.commit_transaction(|channel, result| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
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
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.rollback_transaction(|channel, result| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
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
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.abort_transaction(|channel, result| {
            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
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

    fn js_grove_db_is_transaction_started(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |_platform: &Platform, transaction, channel| {
            let result = transaction.is_some();

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                // First parameter of JS callbacks is error, which is null in this case
                let callback_arguments: Vec<Handle<JsValue>> = vec![
                    task_context.null().upcast(),
                    task_context.boolean(result).upcast(),
                ];

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
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;

        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;
            let path_slice = path.iter().map(|fragment| fragment.as_slice());
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .get(path_slice, &key, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
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

    fn js_grove_db_insert(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;
        let js_element = cx.argument::<JsObject>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let element = converter::js_object_to_element(&mut cx, js_element)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;
            let path_slice = path.iter().map(|fragment| fragment.as_slice());
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .insert(path_slice, &key, element, None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
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

    fn js_grove_db_insert_if_not_exists(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;
        let js_element = cx.argument::<JsObject>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;

        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let element = converter::js_object_to_element(&mut cx, js_element)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let path_slice: Vec<&[u8]> = path.iter().map(|fragment| fragment.as_slice()).collect();
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .insert_if_not_exists(path_slice, key.as_slice(), element, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
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
                    Err(err) => vec![task_context.error(err)?.upcast()],
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
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;

        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let value = converter::js_buffer_to_vec_u8(js_value, &mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .put_aux(&key, &value, None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(()) => {
                        vec![task_context.null().upcast()]
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

    fn js_grove_db_delete_aux(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_key = cx.argument::<JsBuffer>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .delete_aux(&key, None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(()) => {
                        vec![task_context.null().upcast()]
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

    fn js_grove_db_get_aux(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_key = cx.argument::<JsBuffer>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .get_aux(&key, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
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

    fn js_grove_db_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_query = cx.argument::<JsObject>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let path_query = converter::js_path_query_to_path_query(js_path_query, &mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .query_item_value(&path_query, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok((values, skipped)) => {
                        let js_array: Handle<JsArray> = task_context.empty_array();
                        let js_vecs = converter::nested_vecs_to_js(&mut task_context, values)?;
                        let js_num = task_context.number(skipped).upcast::<JsValue>();

                        js_array.set(&mut task_context, 0, js_vecs)?;
                        js_array.set(&mut task_context, 1, js_num)?;

                        vec![task_context.null().upcast(), js_array.upcast()]
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

    fn js_grove_db_prove_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_query = cx.argument::<JsObject>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let path_query = converter::js_path_query_to_path_query(js_path_query, &mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .get_proved_path_query(&path_query, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
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

    fn js_grove_db_prove_query_many(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_queries = cx.argument::<JsArray>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        if js_using_transaction.value(&mut cx) {
            cx.throw_type_error("transaction is not supported")?;
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
        let js_using_transaction = cx.argument::<JsBoolean>(0)?;

        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .root_hash(transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(hash) => vec![
                        task_context.null().upcast(),
                        JsBuffer::external(&mut task_context, hash).upcast(),
                    ],
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

    fn js_grove_db_delete(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;

        let js_using_transaction = cx.argument::<JsBoolean>(2)?;

        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let grove_db = &platform.drive.grove;

            let path_slice: Vec<&[u8]> = path.iter().map(|fragment| fragment.as_slice()).collect();
            let result = transaction_result.and_then(|transaction_arg| {
                grove_db
                    .delete(path_slice, key.as_slice(), None, transaction_arg)
                    .unwrap()
                    .map_err(Error::GroveDB)
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(()) => {
                        vec![task_context.null().upcast()]
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

    fn js_abci_init_chain(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_request = cx.argument::<JsBuffer>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let request_bytes = converter::js_buffer_to_vec_u8(js_request, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let result = transaction_result.and_then(|transaction_arg| {
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
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let request_bytes = converter::js_buffer_to_vec_u8(js_request, &mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let result = transaction_result.and_then(|transaction_arg| {
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
        let js_request = cx.argument::<JsObject>(0)?;

        let js_using_transaction = cx.argument::<JsBoolean>(1)?;

        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let js_fees: Handle<JsObject> = js_request.get(&mut cx, "fees")?;

        let js_processing_fee: Handle<JsNumber> = js_fees.get(&mut cx, "processingFee")?;
        let processing_fee = js_processing_fee.value(&mut cx) as u64;

        let js_storage_fee: Handle<JsNumber> = js_fees.get(&mut cx, "storageFee")?;
        let storage_fee = js_storage_fee.value(&mut cx) as u64;

        let js_fee_refunds: Handle<JsObject> = js_fees.get(&mut cx, "feeRefunds")?;

        let mut fee_refunds: CreditsPerEpoch = Default::default();

        for js_epoch_index_value in js_fee_refunds
            .get_own_property_names(&mut cx)?
            .to_vec(&mut cx)?
        {
            let js_epoch_index = js_epoch_index_value.downcast_or_throw::<JsString, _>(&mut cx)?;

            let epoch_index = js_epoch_index
                .value(&mut cx)
                .parse()
                .or_else(|e: ParseIntError| cx.throw_error(e.to_string()))?;

            let js_credits: Handle<JsNumber> = js_fee_refunds.get(&mut cx, js_epoch_index)?;
            let credits = js_credits.value(&mut cx) as Credits;

            fee_refunds.insert(epoch_index, credits);
        }

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let result = transaction_result.and_then(|transaction_arg| {
                let request = BlockEndRequest {
                    fees: BlockFees {
                        processing_fee,
                        storage_fee,
                        fee_refunds,
                    },
                };

                platform
                    .block_end(request, transaction_arg)
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
        let js_using_transaction = cx.argument::<JsBoolean>(0)?;
        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let using_transaction = js_using_transaction.value(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
            };

            let result = transaction_result.and_then(|transaction_arg| {
                platform
                    .drive
                    .fetch_latest_withdrawal_transaction_index(transaction_arg)
                    .map_err(|err| err.to_string())
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

    fn js_enqueue_withdrawal_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_index = cx.argument::<JsNumber>(0)?;
        let js_core_transaction = cx.argument::<JsBuffer>(1)?;
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;
        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<PlatformWrapper>, _>(&mut cx)?;

        let index = js_index.value(&mut cx);
        let transaction_bytes = converter::js_buffer_to_vec_u8(js_core_transaction, &mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |platform: &Platform, transaction, channel| {
            let transaction_result = if using_transaction {
                if transaction.is_none() {
                    Err("transaction is not started".to_string())
                } else {
                    Ok(transaction)
                }
            } else {
                Ok(None)
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
                    .map_err(|err| err.to_string())
            });

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(_) => {
                        vec![task_context.null().upcast(), task_context.null().upcast()]
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
    cx.export_function("driveFetchIdentity", PlatformWrapper::js_fetch_identity)?;
    cx.export_function(
        "driveFetchIdentityWithCosts",
        PlatformWrapper::js_fetch_identity_with_costs,
    )?;
    cx.export_function(
        "driveAddToIdentityBalance",
        PlatformWrapper::js_add_to_identity_balance,
    )?;
    cx.export_function(
        "driveRemoveFromIdentityBalance",
        PlatformWrapper::js_remove_from_identity_balance,
    )?;
    cx.export_function(
        "driveAddKeysToIdentity",
        PlatformWrapper::js_add_keys_to_identity,
    )?;
    cx.export_function(
        "driveDisableIdentityKeys",
        PlatformWrapper::js_disable_identity_keys,
    )?;
    cx.export_function(
        "driveUpdateIdentityRevision",
        PlatformWrapper::js_update_identity_revision,
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
        "groveDbIsTransactionStarted",
        PlatformWrapper::js_grove_db_is_transaction_started,
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

    cx.export_function(
        "feeResultGetProcessingFee",
        FeeResultWrapper::get_processing_fee,
    )?;
    cx.export_function("feeResultGetStorageFee", FeeResultWrapper::get_storage_fee)?;
    cx.export_function("feeResultAdd", FeeResultWrapper::add)?;
    cx.export_function("feeResultAddFees", FeeResultWrapper::add_fees)?;
    cx.export_function("feeResultCreate", FeeResultWrapper::create)?;
    cx.export_function("feeResultGetRefunds", FeeResultWrapper::get_fee_refunds)?;

    cx.export_function(
        "calculateStorageFeeDistributionAmountAndLeftovers",
        js_calculate_storage_fee_distribution_amount_and_leftovers,
    )?;

    Ok(())
}
