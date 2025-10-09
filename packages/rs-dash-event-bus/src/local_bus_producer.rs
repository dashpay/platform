use crate::event_bus::{EventBus, SubscriptionHandle};
use crate::event_mux::EventMux;
use dapi_grpc::platform::v0::platform_events_command::Version as CmdVersion;
use dapi_grpc::platform::v0::platform_events_command::platform_events_command_v0::Command as Cmd;
use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
// already imported below
use dapi_grpc::platform::v0::platform_events_response::{
    PlatformEventsResponseV0, Version as RespVersion,
};
// keep single RespVersion import
use dapi_grpc::platform::v0::{
    PlatformEventMessageV0, PlatformEventV0, PlatformEventsResponse, PlatformFilterV0,
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Runs a local producer that bridges EventMux commands to a local EventBus of Platform events.
///
/// - `mux`: the shared EventMux instance to attach as a producer
/// - `event_bus`: local bus emitting `PlatformEventV0` events
/// - `make_adapter`: function to convert incoming `PlatformFilterV0` into a bus filter type `F`
pub async fn run_local_platform_events_producer<F>(
    mux: EventMux,
    event_bus: EventBus<PlatformEventV0, F>,
    make_adapter: Arc<dyn Fn(PlatformFilterV0) -> F + Send + Sync>,
) where
    F: crate::event_bus::Filter<PlatformEventV0> + Send + Sync + Debug + 'static,
{
    let producer = mux.add_producer().await;
    let mut cmd_rx = producer.cmd_rx;
    let resp_tx = producer.resp_tx;

    let mut subs: HashMap<String, (SubscriptionHandle<PlatformEventV0, F>, JoinHandle<_>)> =
        HashMap::new();

    while let Some(cmd_res) = cmd_rx.recv().await {
        match cmd_res {
            Ok(cmd) => {
                let v0 = match cmd.version {
                    Some(CmdVersion::V0(v0)) => v0,
                    None => {
                        let err = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Error(
                                    dapi_grpc::platform::v0::PlatformErrorV0 {
                                        client_subscription_id: "".to_string(),
                                        code: 400,
                                        message: "missing version".to_string(),
                                    },
                                )),
                            })),
                        };
                        if resp_tx.send(Ok(err)).await.is_err() {
                            tracing::warn!("local producer failed to send missing version error");
                        }
                        continue;
                    }
                };
                match v0.command {
                    Some(Cmd::Add(add)) => {
                        let id = add.client_subscription_id;
                        let adapter = (make_adapter)(add.filter.unwrap_or_default());
                        let handle = event_bus.add_subscription(adapter).await;

                        // Start forwarding events for this subscription
                        let id_for = id.clone();
                        let handle_clone = handle.clone();
                        let resp_tx_clone = resp_tx.clone();
                        let worker = tokio::spawn(async move {
                            forward_local_events(handle_clone, &id_for, resp_tx_clone).await;
                        });

                        if let Some((old_handle, old_task)) =
                            subs.insert(id.clone(), (handle, worker))
                        {
                            tracing::debug!("replacing existing local subscription with id {}", id);
                            // Stop previous forwarder and drop old subscription
                            old_task.abort();
                            drop(old_handle);
                        }

                        // Ack
                        let ack = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Ack(dapi_grpc::platform::v0::AckV0 {
                                    client_subscription_id: id,
                                    op: "add".to_string(),
                                })),
                            })),
                        };
                        if resp_tx.send(Ok(ack)).await.is_err() {
                            tracing::warn!("local producer failed to send add ack");
                        }
                    }
                    Some(Cmd::Remove(rem)) => {
                        let id = rem.client_subscription_id;
                        if let Some((subscription, worker)) = subs.remove(&id) {
                            let ack = PlatformEventsResponse {
                                version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                    response: Some(Resp::Ack(dapi_grpc::platform::v0::AckV0 {
                                        client_subscription_id: id,
                                        op: "remove".to_string(),
                                    })),
                                })),
                            };
                            if resp_tx.send(Ok(ack)).await.is_err() {
                                tracing::warn!("local producer failed to send remove ack");
                            }

                            // TODO: add subscription close method
                            drop(subscription);
                            worker.abort();
                        }
                    }
                    None => {
                        let err = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Error(
                                    dapi_grpc::platform::v0::PlatformErrorV0 {
                                        client_subscription_id: "".to_string(),
                                        code: 400,
                                        message: "missing command".to_string(),
                                    },
                                )),
                            })),
                        };
                        if resp_tx.send(Ok(err)).await.is_err() {
                            tracing::warn!("local producer failed to send missing command error");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("local producer received error command: {}", e);
                let err = PlatformEventsResponse {
                    version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                        response: Some(Resp::Error(dapi_grpc::platform::v0::PlatformErrorV0 {
                            client_subscription_id: "".to_string(),
                            code: 500,
                            message: format!("{}", e),
                        })),
                    })),
                };
                if resp_tx.send(Ok(err)).await.is_err() {
                    tracing::warn!("local producer failed to send upstream error");
                }
            }
        }
    }
}

async fn forward_local_events<F>(
    subscription: SubscriptionHandle<PlatformEventV0, F>,
    client_subscription_id: &str,
    forward_tx: crate::event_mux::ResponseSender,
) where
    F: crate::event_bus::Filter<PlatformEventV0> + Send + Sync + 'static,
{
    while let Some(evt) = subscription.recv().await {
        let resp = PlatformEventsResponse {
            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                response: Some(Resp::Event(PlatformEventMessageV0 {
                    client_subscription_id: client_subscription_id.to_string(),
                    event: Some(evt),
                })),
            })),
        };
        if forward_tx.send(Ok(resp)).await.is_err() {
            tracing::warn!("client disconnected, stopping local event forwarding");
            break;
        }
    }
}
