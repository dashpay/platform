mod converter;

use std::{option::Option::None, path::Path, sync::mpsc, thread};

use grovedb::{PrefixedRocksDbStorage, Storage};
use neon::prelude::*;
use rs_drive::contract::{Contract, DocumentType};
use rs_drive::drive::Drive;
use rs_drive::query::DriveQuery;

type DriveCallback = Box<
    dyn for<'a> FnOnce(
        &'a mut Drive,
        Option<&<PrefixedRocksDbStorage as Storage>::DBTransaction<'a>>,
        &Channel,
    ) + Send,
>;
type UnitCallback = Box<dyn FnOnce(&Channel) + Send>;

// Messages sent on the drive channel
enum DriveMessage {
    // Callback to be executed
    Callback(DriveCallback),
    // Indicates that the thread should be stopped and connection closed
    Close(UnitCallback),
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
            let mut drive = Drive::open(path).unwrap();

            let mut transaction: Option<<PrefixedRocksDbStorage as Storage>::DBTransaction<'_>> =
                None;

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
                        callback(&mut drive, transaction.as_ref(), &channel);
                    }
                    // Immediately close the connection, even if there are pending messages
                    DriveMessage::Close(callback) => {
                        drop(transaction);
                        drop(drive);
                        callback(&channel);
                        break;
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
        callback: impl for<'a> FnOnce(
            &'a mut Drive,
            Option<&<PrefixedRocksDbStorage as Storage>::DBTransaction<'a>>,
            &Channel,
        ) + Send
        + 'static,
    ) -> Result<(), mpsc::SendError<DriveMessage>> {
        self.tx.send(DriveMessage::Callback(Box::new(callback)))
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

        drive.close(|channel| {
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

    fn js_create_root_tree(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_using_transaction = cx.argument::<JsBoolean>(0)?;
        let js_callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let using_transaction = js_using_transaction.value(&mut cx);

        drive.send_to_drive_thread(move |drive: &mut Drive, transaction, channel| {
            drive.create_root_tree(using_transaction.then(|| transaction).flatten());

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

    fn js_apply_contract(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_contract_cbor = cx.argument::<JsBuffer>(0)?;
        let js_using_transaction = cx.argument::<JsBoolean>(1)?;
        let js_callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive.send_to_drive_thread(move |drive: &mut Drive, transaction, channel| {
            let result = drive.apply_contract(contract_cbor, using_transaction.then(|| transaction).flatten());

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();


                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(score) => {
                        // First parameter of JS callbacks is error, which is null in this case
                        vec![
                            task_context.null().upcast(),
                            task_context.number(score as f64).upcast()
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

        Ok(cx.undefined())
    }

    fn js_add_document_for_contract_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_cbor = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_owner_id = cx.argument::<JsBuffer>(3)?;
        let js_override_document = cx.argument::<JsBoolean>(4)?;
        let js_using_transaction = cx.argument::<JsBoolean>(5)?;
        let js_callback = cx.argument::<JsFunction>(6)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let document_cbor = converter::js_buffer_to_vec_u8(js_document_cbor, &mut cx);
        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let owner_id = converter::js_buffer_to_vec_u8(js_owner_id, &mut cx);
        let js_override_document = js_override_document.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive.send_to_drive_thread(move |drive: &mut Drive, transaction, channel| {
            let result = drive.add_document_for_contract_cbor(
                &document_cbor,
                &contract_cbor,
                &document_type_name,
                Some(&owner_id),
                js_override_document,
                using_transaction.then(|| transaction).flatten());

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(score) => {
                        // First parameter of JS callbacks is error, which is null in this case
                        vec![
                            task_context.null().upcast(),
                            task_context.number(score as f64).upcast()
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

        Ok(cx.undefined())
    }

    fn js_update_document_for_contract_cbor(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_document_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_cbor = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_owner_id = cx.argument::<JsBuffer>(3)?;
        let js_using_transaction = cx.argument::<JsBoolean>(4)?;
        let js_callback = cx.argument::<JsFunction>(5)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let document_cbor = converter::js_buffer_to_vec_u8(js_document_cbor, &mut cx);
        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let owner_id = converter::js_buffer_to_vec_u8(js_owner_id, &mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive.send_to_drive_thread(move |drive: &mut Drive, transaction, channel| {
            let result = drive.update_document_for_contract_cbor(
                &document_cbor,
                &contract_cbor,
                &document_type_name,
                Some(&owner_id),
                using_transaction.then(|| transaction).flatten());

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(score) => {
                        // First parameter of JS callbacks is error, which is null in this case
                        vec![
                            task_context.null().upcast(),
                            task_context.number(score as f64).upcast()
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

        drive.send_to_drive_thread(move |drive: &mut Drive, transaction, channel| {
            let result = drive.delete_document_for_contract_cbor(
                &document_id,
                &contract_cbor,
                &document_type_name,
                None,
                using_transaction.then(|| transaction).flatten());

            channel.send(move |mut task_context| {
                let callback = js_callback.into_inner(&mut task_context);
                let this = task_context.undefined();

                let callback_arguments: Vec<Handle<JsValue>> = match result {
                    Ok(score) => {
                        // First parameter of JS callbacks is error, which is null in this case
                        vec![
                            task_context.null().upcast(),
                            task_context.number(score as f64).upcast()
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

        Ok(cx.undefined())
    }

    fn js_create_and_execute_query(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let js_query_cbor = cx.argument::<JsBuffer>(0)?;
        let js_contract_cbor = cx.argument::<JsBuffer>(1)?;
        let js_document_type_name = cx.argument::<JsString>(2)?;
        let js_using_transaction = cx.argument::<JsBoolean>(3)?;
        let js_callback = cx.argument::<JsFunction>(4)?.root(&mut cx);

        let drive = cx
            .this()
            .downcast_or_throw::<JsBox<DriveWrapper>, _>(&mut cx)?;

        let query_cbor = converter::js_buffer_to_vec_u8(js_query_cbor, &mut cx);
        let contract_cbor = converter::js_buffer_to_vec_u8(js_contract_cbor, &mut cx);
        let document_type_name = js_document_type_name.value(&mut cx);
        let using_transaction = js_using_transaction.value(&mut cx);

        drive.send_to_drive_thread(move |drive: &mut Drive, transaction, channel| {
            let result = drive.query_documents_from_cbor(
                &contract_cbor,
                document_type_name,
                &query_cbor,
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

        Ok(cx.undefined())
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("driveOpen", DriveWrapper::js_open)?;
    cx.export_function("driveClose", DriveWrapper::js_close)?;
    cx.export_function("driveCreateRootTree", DriveWrapper::js_create_root_tree)?;
    cx.export_function("driveApplyContract", DriveWrapper::js_apply_contract)?;
    cx.export_function("driveCreateDocument", DriveWrapper::js_add_document_for_contract_cbor)?;
    cx.export_function("driveUpdateDocument", DriveWrapper::js_update_document_for_contract_cbor)?;
    cx.export_function("driveDeleteDocument", DriveWrapper::js_delete_document_for_contract_cbor)?;
    cx.export_function("driveQueryDocuments", DriveWrapper::js_create_and_execute_query)?;

    Ok(())
}
