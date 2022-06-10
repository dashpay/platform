mod converter;

use std::{option::Option::None, path::Path, sync::mpsc, thread};

use grovedb::{Transaction, TransactionArg};
use neon::prelude::*;
use neon::types::JsDate;
use rs_drive::drive::Drive;

const READONLY_MSG: &str =
    "db is in readonly mode due to the active transaction. Please provide transaction or commit it";

type DriveCallback = Box<dyn for<'a> FnOnce(&'a Drive, TransactionArg, &Channel) + Send>;
type UnitCallback = Box<dyn FnOnce(&Channel) + Send>;

// Messages sent on the drive channel
enum DriveMessage {
    // Callback to be executed
    Callback(DriveCallback),
    // Indicates that the thread should be stopped and connection closed
    Close(UnitCallback),
    StartTransaction(UnitCallback),
    CommitTransaction(UnitCallback),
    RollbackTransaction(UnitCallback),
    AbortTransaction(UnitCallback),
    Flush(UnitCallback),
}

struct DriveWrapper {
    tx: mpsc::Sender<DriveMessage>,
}

// Internal wrapper logic. Needed to avoid issues with passing threads to
// node.js. Avoiding thread conflicts by having a dedicated thread for the
// groveDB instance and uses events to communicate with it
impl DriveWrapper {
    // Creates a new instance of `DriveWrapper`
    //
    // 1. Creates a connection and a channel
    // 2. Spawns a thread and moves the channel receiver and connection to it
    // 3. On a separate thread, read closures off the channel and execute with
    // access    to the connection.
    fn new(cx: &mut FunctionContext) -> NeonResult<Self> {
        let path_string = cx.argument::<JsString>(0)?.value(cx);

        // Channel for sending callbacks to execute on the Drive connection thread
        let (tx, rx) = mpsc::channel::<DriveMessage>();

        // Create an `Channel` for calling back to JavaScript. It is more efficient
        // to create a single channel and re-use it for all database callbacks.
        // The JavaScript process will not exit as long as this channel has not been
        // dropped.
        let channel = cx.channel();

        // Spawn a thread for processing database queries
        // This will not block the JavaScript main thread and will continue executing
        // concurrently.
        thread::spawn(move || {
            let path = Path::new(&path_string);
            // Open a connection to groveDb, this will be moved to a separate thread
            // TODO: think how to pass this error to JS
            let drive = Drive::open(path).unwrap();

            let mut transaction: Option<Transaction> = None;

            // Blocks until a callback is available
            // When the instance of `Database` is dropped, the channel will be closed
            // and `rx.recv()` will return an `Err`, ending the loop and terminating
            // the thread.
            while let Ok(message) = rx.recv() {
                match message {
                    DriveMessage::Callback(callback) => {
                        // The connection and channel are owned by the thread, but _lent_ to
                        // the callback. The callback has exclusive access to the connection
                        // for the duration of the callback.
                        callback(&drive, transaction.as_ref(), &channel);
                    }
                    // Immediately close the connection, even if there are pending messages
                    DriveMessage::Close(callback) => {
                        drop(transaction);
                        drop(drive);
                        callback(&channel);
                        break;
                    }
                    // Flush message
                    DriveMessage::Flush(callback) => {
                        drive.grove.flush().unwrap();
                        callback(&channel);
                    }
                    DriveMessage::StartTransaction(callback) => {
                        transaction = Some(drive.grove.start_transaction());
                        callback(&channel);
                    }
                    DriveMessage::CommitTransaction(callback) => {
                        drive
                            .commit_transaction(transaction.take().unwrap())
                            .unwrap();
                        callback(&channel);
                    }
                    DriveMessage::RollbackTransaction(callback) => {
                        drive
                            .rollback_transaction(&transaction.take().unwrap())
                            .unwrap();
                        callback(&channel);
                    }
                    DriveMessage::AbortTransaction(callback) => {
                        drop(transaction.take());
                        callback(&channel);
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
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx.send(DriveMessage::Close(Box::new(callback)))
    }

    fn send_to_drive_thread(
        &self,
        callback: impl for<'a> FnOnce(&'a Drive, TransactionArg, &Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx.send(DriveMessage::Callback(Box::new(callback)))
    }

    fn start_transaction(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx
            .send(DriveMessage::StartTransaction(Box::new(callback)))
    }

    fn commit_transaction(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx
            .send(DriveMessage::CommitTransaction(Box::new(callback)))
    }

    fn rollback_transaction(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx
            .send(DriveMessage::RollbackTransaction(Box::new(callback)))
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a
    // closed database
    fn flush(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx.send(DriveMessage::Flush(Box::new(callback)))
    }

    fn abort_transaction(
        &self,
        callback: impl FnOnce(&Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx
            .send(DriveMessage::AbortTransaction(Box::new(callback)))
    }
}

// Ensures that DriveWrapper is properly disposed when the corresponding JS
// object gets garbage collected
impl Finalize for DriveWrapper {}

// External wrapper logic
impl DriveWrapper {
    // Create a new instance of `Drive` and place it inside a `JsBox`
    // JavaScript can hold a reference to a `JsBox`, but the contents are opaque
    fn js_open(mut cx: FunctionContext) -> JsResult<JsBox<DriveWrapper>> {
        let drive_wrapper =
            DriveWrapper::new(&mut cx).or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.boxed(drive_wrapper))
    }

    /// Sends a message to the DB thread to stop the thread and dispose the
    /// groveDb instance owned by it, then calls js callback passed as a first
    /// argument to the function
    fn js_close(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

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

    fn js_create_root_tree(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_using_transaction = cx.argument::<JsBoolean>(0)?;
        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                drive
                    .create_root_tree(using_transaction.then(|| transaction).flatten())
                    .expect("create_root_tree should not fail");

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

    fn js_apply_contract(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_contract_cbor = cx.argument::<JsBuffer>(0)?;
        let js_block_time = cx.argument::<JsDate>(1)?;
        let js_apply = cx.argument::<JsBoolean>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);
        let block_time = js_block_time.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                let result = drive.apply_contract_cbor(
                    contract_cbor,
                    None,
                    block_time,
                    apply,
                    using_transaction.then(|| transaction).flatten(),
                );

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((storage_fee, processing_fee)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();

                            let storage_fee_value =
                                task_context.number(storage_fee as f64).upcast::<JsValue>();
                            let processing_fee_value = task_context
                                .number(processing_fee as f64)
                                .upcast::<JsValue>();

                            js_array.set(&mut task_context, 0, storage_fee_value)?;
                            js_array.set(&mut task_context, 1, processing_fee_value)?;

                            // First parameter of JS callbacks is error, which is null in this case
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

    fn js_add_document_for_contract_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_cbor = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_owner_id = cx.argument::<JsBuffer>(3)?;
        let js_override_document = cx.argument::<JsBoolean>(4)?;
        let js_block_time = cx.argument::<JsDate>(5)?;
        let js_apply = cx.argument::<JsBoolean>(6)?;
        let js_using_transaction = cx.argument::<JsBoolean>(7)?;
        let js_callback = cx.argument::<JsFunction>(8)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let document_cbor = converter::js_buffer_to_vec_u8(js_document_cbor, &mut cx);
        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let owner_id = converter::js_buffer_to_vec_u8(js_owner_id, &mut cx);
        let override_document = js_override_document.value(&mut cx);
        let block_time = js_block_time.value(&mut cx);
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                let result = drive.add_document_for_contract_cbor(
                    &document_cbor,
                    &contract_cbor,
                    &document_type_name,
                    Some(&owner_id),
                    override_document,
                    block_time,
                    apply,
                    using_transaction.then(|| transaction).flatten(),
                );

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((storage_fee, processing_fee)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();

                            let storage_fee_value =
                                task_context.number(storage_fee as f64).upcast::<JsValue>();
                            let processing_fee_value = task_context
                                .number(processing_fee as f64)
                                .upcast::<JsValue>();

                            js_array.set(&mut task_context, 0, storage_fee_value)?;
                            js_array.set(&mut task_context, 1, processing_fee_value)?;

                            // First parameter of JS callbacks is error, which is null in this case
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

    fn js_update_document_for_contract_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_cbor = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_owner_id = cx.argument::<JsBuffer>(3)?;
        let js_block_time = cx.argument::<JsDate>(4)?;
        let js_apply = cx.argument::<JsBoolean>(6)?;
        let js_using_transaction = cx.argument::<JsBoolean>(6)?;
        let js_callback = cx.argument::<JsFunction>(7)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let document_cbor = converter::js_buffer_to_vec_u8(js_document_cbor, &mut cx);
        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let owner_id = converter::js_buffer_to_vec_u8(js_owner_id, &mut cx);
        let block_time = js_block_time.value(&mut cx);
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                let result = drive.update_document_for_contract_cbor(
                    &document_cbor,
                    &contract_cbor,
                    &document_type_name,
                    Some(&owner_id),
                    block_time,
                    apply,
                    using_transaction.then(|| transaction).flatten(),
                );

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((storage_fee, processing_fee)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();

                            let storage_fee_value =
                                task_context.number(storage_fee as f64).upcast::<JsValue>();
                            let processing_fee_value = task_context
                                .number(processing_fee as f64)
                                .upcast::<JsValue>();

                            js_array.set(&mut task_context, 0, storage_fee_value)?;
                            js_array.set(&mut task_context, 1, processing_fee_value)?;

                            // First parameter of JS callbacks is error, which is null in this case
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

    fn js_delete_document_for_contract_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_id = cx.argument::<JsBuffer>(0)?;
        let js_contract_cbor = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let document_id = converter::js_buffer_to_vec_u8(js_document_id, &mut cx);
        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                if transaction.is_some() && !using_transaction {
                    channel.send(move |mut task_context| {
                        let callback = js_callback.into_inner(&mut task_context);
                        let this = task_context.undefined();
                        let callback_arguments: Vec<Handle<JsValue>> =
                            vec![task_context.error(READONLY_MSG)?.upcast()];

                        callback.call(&mut task_context, this, callback_arguments)?;
                        Ok(())
                    });
                } else {
                    let result = drive.delete_document_for_contract_cbor(
                        &document_id,
                        &contract_cbor,
                        &document_type_name,
                        None,
                        using_transaction.then(|| transaction).flatten(),
                    );

                    channel.send(move |mut task_context| {
                        let callback = js_callback.into_inner(&mut task_context);
                        let this = task_context.undefined();

                        let callback_arguments: Vec<Handle<JsValue>> = match result {
                            Ok((storage_fee, processing_fee)) => {
                                let js_array: Handle<JsArray> = task_context.empty_array();

                                let storage_fee_value =
                                    task_context.number(storage_fee as f64).upcast::<JsValue>();
                                let processing_fee_value = task_context
                                    .number(processing_fee as f64)
                                    .upcast::<JsValue>();

                                js_array.set(&mut task_context, 0, storage_fee_value)?;
                                js_array.set(&mut task_context, 1, processing_fee_value)?;

                                // First parameter of JS callbacks is error, which is null in this case
                                vec![task_context.null().upcast(), js_array.upcast()]
                            }

                            // Convert the error to a JavaScript exception on failure
                            Err(err) => vec![task_context.error(err.to_string())?.upcast()],
                        };

                        callback.call(&mut task_context, this, callback_arguments)?;

                        Ok(())
                    });
                }
            })
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_insert_identity_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_identity_id = cx.argument::<JsBuffer>(0)?;
        let js_identity_cbor = cx.argument::<JsBuffer>(1)?;
        let js_apply = cx.argument::<JsBoolean>(3)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let identity_id = converter::js_buffer_to_vec_u8(js_identity_id, &mut cx);
        let identity_cbor = converter::js_buffer_to_vec_u8(js_identity_cbor, &mut cx);
        let apply = js_apply.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                let result = drive.insert_identity_cbor(
                    Some(&identity_id),
                    identity_cbor,
                    apply,
                    using_transaction.then(|| transaction).flatten(),
                );

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();

                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((storage_fee, processing_fee)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();

                            let storage_fee_value =
                                task_context.number(storage_fee as f64).upcast::<JsValue>();
                            let processing_fee_value = task_context
                                .number(processing_fee as f64)
                                .upcast::<JsValue>();

                            js_array.set(&mut task_context, 0, storage_fee_value)?;
                            js_array.set(&mut task_context, 1, processing_fee_value)?;

                            // First parameter of JS callbacks is error, which is null in this case
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

    fn js_create_and_execute_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_query_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let query_cbor = converter::js_buffer_to_vec_u8(js_query_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_vec_u8(js_contract_id, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                let result = drive.query_documents(
                    &query_cbor,
                    <[u8; 32]>::try_from(contract_id).unwrap(),
                    document_type_name.as_str(),
                    using_transaction.then(|| transaction).flatten(),
                );

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok((value, skipped, cost)) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();
                            let js_vecs = converter::nested_vecs_to_js(value, &mut task_context)?;
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

    fn js_create_and_execute_query_as_grove_proof(
        mut cx: FunctionContext,
    ) -> JsResult<JsUndefined> {
        let js_query_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_id = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let query_cbor = converter::js_buffer_to_vec_u8(js_query_cbor, &mut cx);
        let contract_id = converter::js_buffer_to_vec_u8(js_contract_id, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive
            .send_to_drive_thread(move |drive: &Drive, transaction, channel| {
                let result = drive.query_documents_as_grove_proof(
                    &query_cbor,
                    <[u8; 32]>::try_from(contract_id).unwrap(),
                    document_type_name.as_str(),
                    using_transaction.then(|| transaction).flatten(),
                );

                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> = match result {
                        Ok(value) => {
                            let js_array: Handle<JsArray> = task_context.empty_array();
                            let js_buffer = JsBuffer::external(&mut task_context, value);
                            js_array.set(&mut task_context, 0, js_buffer)?;

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
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.start_transaction(|channel| {
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

    fn js_grove_db_commit_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.commit_transaction(|channel| {
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

    fn js_grove_db_rollback_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.rollback_transaction(|channel| {
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

    fn js_grove_db_is_transaction_started(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |_drive: &Drive, transaction, channel| {
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

    fn js_grove_db_abort_transaction(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.abort_transaction(|channel| {
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

    fn js_grove_db_get(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path = cx.argument::<JsArray>(0)?;
        let js_key = cx.argument::<JsBuffer>(1)?;
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;
        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            let grove_db = &drive.grove;
            let path_slice = path.iter().map(|fragment| fragment.as_slice());
            let result = grove_db.get(
                path_slice,
                &key,
                using_transaction.then(|| transaction).flatten(),
            );

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(element) => {
                        // First parameter of JS callbacks is error, which is null in this case
                        vec![
                            task_context.null().upcast(),
                            converter::element_to_js_object(element, &mut task_context)?,
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
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);
        let element = converter::js_object_to_element(js_element, &mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            if transaction.is_some() && !using_transaction {
                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> =
                        vec![task_context.error(READONLY_MSG)?.upcast()];

                    callback.call(&mut task_context, this, callback_arguments)?;
                    Ok(())
                });
            } else {
                let grove_db = &drive.grove;
                let path_slice = path.iter().map(|fragment| fragment.as_slice());
                let result = grove_db.insert(
                    path_slice,
                    &key,
                    element,
                    using_transaction.then(|| transaction).flatten(),
                );

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
            }
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
        let element = converter::js_object_to_element(js_element, &mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        // Get the `this` value as a `JsBox<Database>`
        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            if transaction.is_some() && !using_transaction {
                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> =
                        vec![task_context.error(READONLY_MSG)?.upcast()];

                    callback.call(&mut task_context, this, callback_arguments)?;
                    Ok(())
                });
            } else {
                let grove_db = &drive.grove;

                let path_slice: Vec<&[u8]> =
                    path.iter().map(|fragment| fragment.as_slice()).collect();
                let result = grove_db.insert_if_not_exists(
                    path_slice,
                    key.as_slice(),
                    element,
                    using_transaction.then(|| transaction).flatten(),
                );

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
            }
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

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            let grove_db = &drive.grove;

            let result = grove_db.put_aux(
                &key,
                &value,
                using_transaction.then(|| transaction).flatten(),
            );

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
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;
        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            if transaction.is_some() && !using_transaction {
                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> =
                        vec![task_context.error(READONLY_MSG)?.upcast()];

                    callback.call(&mut task_context, this, callback_arguments)?;
                    Ok(())
                });
            } else {
                let grove_db = &drive.grove;

                let result =
                    grove_db.delete_aux(&key, using_transaction.then(|| transaction).flatten());

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
            }
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
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            let grove_db = &drive.grove;

            let result = grove_db.get_aux(&key, using_transaction.then(|| transaction).flatten());

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

    fn js_grove_db_get_path_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_path_query = cx.argument::<JsObject>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;
        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let path_query = converter::js_path_query_to_path_query(js_path_query, &mut cx)?;

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            let grove_db = &drive.grove;

            let result = grove_db.get_path_query(
                &path_query,
                using_transaction.then(|| transaction).flatten(),
            );

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();
                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok((value, skipped)) => {
                        let js_array: Handle<JsArray> = task_context.empty_array();
                        let js_vecs = converter::nested_vecs_to_js(value, &mut task_context)?;
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

    /// Not implemented
    fn js_grove_db_proof(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        Ok(cx.undefined())
    }

    /// Flush data on disc and then calls js callback passed as a first
    /// argument to the function
    fn js_grove_db_flush(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

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

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            let grove_db = &drive.grove;

            let result = grove_db.root_hash(using_transaction.then(|| transaction).flatten());

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(Some(hash)) => vec![
                        task_context.null().upcast(),
                        JsBuffer::external(&mut task_context, hash).upcast(),
                    ],
                    Ok(None) => vec![
                        task_context.null().upcast(),
                        task_context.buffer(32)?.upcast(),
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
        let js_using_transaction = cx.argument::<JsBoolean>(2)?;
        let js_callback = cx.argument::<JsFunction>(3)?.root(&mut cx);

        let path = converter::js_array_of_buffers_to_vec(js_path, &mut cx)?;
        let key = converter::js_buffer_to_vec_u8(js_key, &mut cx);

        let db = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;
        let using_transaction = js_using_transaction.value(&mut cx);

        db.send_to_drive_thread(move |drive: &Drive, transaction, channel| {
            if transaction.is_some() && !using_transaction {
                channel.send(move |mut task_context| {
                    let callback = js_callback.into_inner(&mut task_context);
                    let this = task_context.undefined();
                    let callback_arguments: Vec<Handle<JsValue>> =
                        vec![task_context.error(READONLY_MSG)?.upcast()];

                    callback.call(&mut task_context, this, callback_arguments)?;
                    Ok(())
                });
            } else {
                let grove_db = &drive.grove;

                let path_slice: Vec<&[u8]> =
                    path.iter().map(|fragment| fragment.as_slice()).collect();
                let result = grove_db.delete(
                    path_slice,
                    key.as_slice(),
                    using_transaction.then(|| transaction).flatten(),
                );

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
            }
        })
        .or_else(|err| cx.throw_error(err.to_string()))?;

        // The result is returned through the callback, not through direct return
        Ok(cx.undefined())
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("driveOpen", DriveWrapper::js_open)?;
    cx.export_function("driveClose", DriveWrapper::js_close)?;
    cx.export_function("driveCreateRootTree", DriveWrapper::js_create_root_tree)?;
    cx.export_function("driveApplyContract", DriveWrapper::js_apply_contract)?;
    cx.export_function(
        "driveCreateDocument",
        DriveWrapper::js_add_document_for_contract_cbor,
    )?;
    cx.export_function(
        "driveUpdateDocument",
        DriveWrapper::js_update_document_for_contract_cbor,
    )?;
    cx.export_function(
        "driveDeleteDocument",
        DriveWrapper::js_delete_document_for_contract_cbor,
    )?;
    cx.export_function("driveInsertIdentity", DriveWrapper::js_insert_identity_cbor)?;
    cx.export_function(
        "driveQueryDocuments",
        DriveWrapper::js_create_and_execute_query,
    )?;
    cx.export_function("groveDbInsert", DriveWrapper::js_grove_db_insert)?;
    cx.export_function(
        "groveDbInsertIfNotExists",
        DriveWrapper::js_grove_db_insert_if_not_exists,
    )?;
    cx.export_function("groveDbGet", DriveWrapper::js_grove_db_get)?;
    cx.export_function("groveDbDelete", DriveWrapper::js_grove_db_delete)?;
    cx.export_function("groveDbProof", DriveWrapper::js_grove_db_proof)?;
    cx.export_function("groveDbFlush", DriveWrapper::js_grove_db_flush)?;
    cx.export_function(
        "groveDbStartTransaction",
        DriveWrapper::js_grove_db_start_transaction,
    )?;
    cx.export_function(
        "groveDbCommitTransaction",
        DriveWrapper::js_grove_db_commit_transaction,
    )?;
    cx.export_function(
        "groveDbRollbackTransaction",
        DriveWrapper::js_grove_db_rollback_transaction,
    )?;
    cx.export_function(
        "groveDbIsTransactionStarted",
        DriveWrapper::js_grove_db_is_transaction_started,
    )?;
    cx.export_function(
        "groveDbAbortTransaction",
        DriveWrapper::js_grove_db_abort_transaction,
    )?;
    cx.export_function("groveDbPutAux", DriveWrapper::js_grove_db_put_aux)?;
    cx.export_function("groveDbDeleteAux", DriveWrapper::js_grove_db_delete_aux)?;
    cx.export_function("groveDbGetAux", DriveWrapper::js_grove_db_get_aux)?;
    cx.export_function(
        "groveDbGetPathQuery",
        DriveWrapper::js_grove_db_get_path_query,
    )?;
    cx.export_function("groveDbRootHash", DriveWrapper::js_grove_db_root_hash)?;

    Ok(())
}
