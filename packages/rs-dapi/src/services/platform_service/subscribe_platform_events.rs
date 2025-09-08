use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_events_command::platform_events_command_v0::Command as Cmd;
use dapi_grpc::platform::v0::platform_events_command::Version as CmdVersion;
use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
use dapi_grpc::platform::v0::platform_events_response::PlatformEventsResponseV0;
use dapi_grpc::platform::v0::{
    PlatformEventMessageV0, PlatformEventsCommand, PlatformEventsResponse, PlatformFilterV0,
};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::clients::drive_client::{DriveChannel, DriveClient};
use crate::metrics;

use super::PlatformServiceImpl;

/// Number of upstream connections to drive‑abci used by the proxy.
const UPSTREAM_CONN_COUNT: usize = 2;

/// Multiplexer that manages a pool of bi‑di upstream connections to Drive ABCI.
#[derive(Clone)]
struct PlatformEventsMux {
    /// Drive gRPC client used to open upstream connections.
    drive_client: DriveClient,
    /// Per‑upstream sender for commands into each bi‑di stream.
    upstream_txs: Vec<mpsc::UnboundedSender<PlatformEventsCommand>>,
    /// Routing map: upstream_id -> (downstream session sender, public_id).
    routes: Arc<
        RwLock<
            BTreeMap<
                String,
                (
                    mpsc::UnboundedSender<Result<PlatformEventsResponse, Status>>,
                    String,
                ),
            >,
        >,
    >,
    /// Monotonic counter to create per‑session ID prefixes.
    session_counter: Arc<AtomicU64>,
    /// Round‑robin counter for choosing an upstream connection.
    rr_counter: Arc<AtomicUsize>,
}

impl PlatformEventsMux {
    /// Create a new mux and spawn the upstream connection tasks.
    async fn new(drive_client: DriveClient) -> Result<Self, Status> {
        let routes = Arc::new(RwLock::new(BTreeMap::new()));

        // Start a small pool of upstream connection tasks
        let mut upstream_txs = Vec::with_capacity(UPSTREAM_CONN_COUNT);
        for _ in 0..UPSTREAM_CONN_COUNT {
            let (up_tx, up_rx) = mpsc::unbounded_channel::<PlatformEventsCommand>();
            let client = drive_client.get_client();
            Self::spawn_upstream(client, up_rx, routes.clone());
            upstream_txs.push(up_tx);
        }

        Ok(Self {
            drive_client,
            upstream_txs,
            routes,
            session_counter: Arc::new(AtomicU64::new(1)),
            rr_counter: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Spawn a single upstream bi‑di stream task to Drive ABCI.
    fn spawn_upstream(
        mut client: PlatformClient<DriveChannel>,
        up_rx: mpsc::UnboundedReceiver<PlatformEventsCommand>,
        routes: Arc<
            RwLock<
                BTreeMap<
                    String,
                    (
                        mpsc::UnboundedSender<Result<PlatformEventsResponse, Status>>,
                        String,
                    ),
                >,
            >,
        >,
    ) {
        tokio::spawn(async move {
            use tokio_stream::StreamExt;
            let cmd_stream = UnboundedReceiverStream::new(up_rx);

            let res = client.subscribe_platform_events(cmd_stream).await;
            if let Ok(mut resp_stream) = res.map(|r| r.into_inner()) {
                metrics::platform_events_upstream_stream_started();
                loop {
                    match resp_stream.message().await {
                        Ok(Some(PlatformEventsResponse { version: Some(v) })) => {
                            let dapi_grpc::platform::v0::platform_events_response::Version::V0(v0) =
                                v;
                            match v0.response {
                                Some(Resp::Event(PlatformEventMessageV0 {
                                    client_subscription_id,
                                    event,
                                })) => {
                                    let entry = {
                                        routes.read().await.get(&client_subscription_id).cloned()
                                    };
                                    if let Some((tx, public_id)) = entry {
                                        let rewired = PlatformEventsResponse{
                                                version: Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(PlatformEventsResponseV0{
                                                    response: Some(Resp::Event(PlatformEventMessageV0{ client_subscription_id: public_id, event }))
                                                }))
                                            };
                                        let _ = tx.send(Ok(rewired));
                                        metrics::platform_events_forwarded_event();
                                    }
                                }
                                Some(Resp::Ack(mut ack)) => {
                                    let entry = {
                                        routes
                                            .read()
                                            .await
                                            .get(&ack.client_subscription_id)
                                            .cloned()
                                    };
                                    if let Some((tx, public_id)) = entry {
                                        ack.client_subscription_id = public_id;
                                        let rewired = PlatformEventsResponse{
                                                version: Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(PlatformEventsResponseV0{ response: Some(Resp::Ack(ack)) }))
                                            };
                                        let _ = tx.send(Ok(rewired));
                                        metrics::platform_events_forwarded_ack();
                                    }
                                }
                                Some(Resp::Error(mut err)) => {
                                    let entry = {
                                        routes
                                            .read()
                                            .await
                                            .get(&err.client_subscription_id)
                                            .cloned()
                                    };
                                    if let Some((tx, public_id)) = entry {
                                        err.client_subscription_id = public_id;
                                        let rewired = PlatformEventsResponse{
                                                version: Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(PlatformEventsResponseV0{ response: Some(Resp::Error(err)) }))
                                            };
                                        let _ = tx.send(Ok(rewired));
                                        metrics::platform_events_forwarded_error();
                                    }
                                }
                                None => {}
                            }
                        }
                        Ok(None) => break,
                        Ok(Some(PlatformEventsResponse { version: None })) => {}
                        Err(_) => break,
                    }
                }
            }
        });
    }

    /// Generate a unique per‑session prefix for upstream IDs.
    fn next_session_prefix(&self) -> String {
        let n = self.session_counter.fetch_add(1, Ordering::Relaxed);
        format!("s{}", n)
    }

    /// Pick an upstream connection in round‑robin fashion.
    fn choose_upstream(&self) -> (usize, mpsc::UnboundedSender<PlatformEventsCommand>) {
        let idx = self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.upstream_txs.len();
        (idx, self.upstream_txs[idx].clone())
    }

    /// Register a new client session and bind it to an upstream.
    async fn register_session_with_tx(
        &self,
        downstream_tx: mpsc::UnboundedSender<Result<PlatformEventsResponse, Status>>,
    ) -> PlatformEventsSession {
        let (up_idx, upstream_tx) = self.choose_upstream();
        PlatformEventsSession {
            mux: self.clone(),
            session_prefix: self.next_session_prefix(),
            downstream_tx,
            upstream_tx,
            upstream_idx: up_idx,
            public_to_upstream: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

/// Per‑client session that rewrites IDs and routes events.
struct PlatformEventsSession {
    /// Shared upstream multiplexer used by this session.
    mux: PlatformEventsMux,
    /// Unique per‑session prefix used in upstream IDs.
    session_prefix: String,
    /// Sender for responses to the public client stream.
    downstream_tx: mpsc::UnboundedSender<Result<PlatformEventsResponse, Status>>,
    /// Sender for commands to the chosen upstream connection.
    upstream_tx: mpsc::UnboundedSender<PlatformEventsCommand>,
    /// Index of the upstream connection chosen for this session.
    upstream_idx: usize,
    /// Per‑session map of public_id -> upstream_id.
    public_to_upstream: Arc<Mutex<BTreeMap<String, String>>>,
}

impl PlatformEventsSession {
    /// Build an upstream subscription ID from the public ID.
    fn upstream_id(&self, public_id: &str) -> String {
        // include upstream index for uniqueness across pool and easier debugging
        format!(
            "u{}:{}:{}",
            self.upstream_idx, self.session_prefix, public_id
        )
    }

    /// Add a subscription: register routing and forward upstream.
    async fn add(&self, public_id: String, filter: PlatformFilterV0) {
        let up_id = self.upstream_id(&public_id);
        // register route
        {
            let mut map = self.public_to_upstream.lock().await;
            map.insert(public_id.clone(), up_id.clone());
        }
        {
            let mut routes = self.mux.routes.write().await;
            routes.insert(up_id.clone(), (self.downstream_tx.clone(), public_id));
        }
        // send upstream add
        let cmd = PlatformEventsCommand {
            version: Some(CmdVersion::V0(
                dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                    command: Some(Cmd::Add(dapi_grpc::platform::v0::AddSubscriptionV0 {
                        client_subscription_id: up_id,
                        filter: Some(filter),
                    })),
                },
            )),
        };
        let _ = self.upstream_tx.send(cmd);
    }

    /// Remove a subscription: drop routing and forward upstream.
    async fn remove(&self, public_id: String) {
        let up_id_opt = {
            self.public_to_upstream
                .lock()
                .await
                .get(&public_id)
                .cloned()
        };
        if let Some(up_id) = up_id_opt {
            // remove route
            {
                let mut routes = self.mux.routes.write().await;
                routes.remove(&up_id);
            }
            // send upstream remove
            let cmd = PlatformEventsCommand {
                version: Some(CmdVersion::V0(
                    dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                        command: Some(Cmd::Remove(dapi_grpc::platform::v0::RemoveSubscriptionV0 {
                            client_subscription_id: up_id,
                        })),
                    },
                )),
            };
            let _ = self.upstream_tx.send(cmd);
        }
    }
}

impl Drop for PlatformEventsSession {
    fn drop(&mut self) {
        let upstream_tx = self.upstream_tx.clone();
        let map = self.public_to_upstream.clone();
        tokio::spawn(async move {
            let ids: Vec<(String, String)> = {
                let m = map.lock().await;
                m.iter()
                    .map(|(pub_id, up_id)| (pub_id.clone(), up_id.clone()))
                    .collect()
            };
            for (_pub_id, up_id) in ids {
                let cmd = PlatformEventsCommand {
                    version: Some(CmdVersion::V0(
                        dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                            command: Some(Cmd::Remove(
                                dapi_grpc::platform::v0::RemoveSubscriptionV0 {
                                    client_subscription_id: up_id,
                                },
                            )),
                        },
                    )),
                };
                let _ = upstream_tx.send(cmd);
            }
        });
        metrics::platform_events_active_sessions_dec();
    }
}

impl PlatformServiceImpl {
    /// Proxy implementation of Platform::subscribePlatformEvents with upstream muxing.
    pub async fn subscribe_platform_events_impl(
        &self,
        request: Request<dapi_grpc::tonic::Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<UnboundedReceiverStream<Result<PlatformEventsResponse, Status>>>, Status>
    {
        use tokio_stream::StreamExt;

        // Ensure single upstream mux exists (lazy init stored in self via once_cell)
        let mux = {
            use once_cell::sync::OnceCell;
            static MUX: OnceCell<PlatformEventsMux> = OnceCell::new();
            if let Some(m) = MUX.get() {
                m.clone()
            } else {
                let m = PlatformEventsMux::new(self.drive_client.clone())
                    .await
                    .map_err(|e| Status::internal(format!("failed to init upstream mux: {}", e)))?;
                MUX.set(m.clone()).ok();
                m
            }
        };

        let (out_tx, out_rx) = mpsc::unbounded_channel::<Result<PlatformEventsResponse, Status>>();
        let session = mux.register_session_with_tx(out_tx.clone()).await;
        metrics::platform_events_active_sessions_inc();

        let mut inbound = request.into_inner();
        // Process client commands
        tokio::spawn(async move {
            loop {
                match inbound.message().await {
                    Ok(Some(PlatformEventsCommand {
                        version: Some(CmdVersion::V0(v0)),
                    })) => {
                        match v0.command {
                            Some(Cmd::Add(add)) => {
                                let filter = add.filter.unwrap_or(PlatformFilterV0 { kind: None });
                                session.add(add.client_subscription_id, filter).await;
                                metrics::platform_events_command("add");
                            }
                            Some(Cmd::Remove(rem)) => {
                                session.remove(rem.client_subscription_id).await;
                                metrics::platform_events_command("remove");
                            }
                            Some(Cmd::Ping(p)) => {
                                // Local ack (do not forward upstream)
                                let resp = PlatformEventsResponse{
                                    version: Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(PlatformEventsResponseV0{
                                        response: Some(Resp::Ack(dapi_grpc::platform::v0::AckV0{ client_subscription_id: p.nonce.to_string(), op: "ping".to_string() }))
                                    }))
                                };
                                let _ = out_tx.send(Ok(resp));
                                metrics::platform_events_command("ping");
                            }
                            None => {
                                let resp = PlatformEventsResponse{
                                    version: Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(PlatformEventsResponseV0{
                                        response: Some(Resp::Error(dapi_grpc::platform::v0::PlatformErrorV0{ client_subscription_id: "".to_string(), code: 400, message: "missing command".to_string() }))
                                    }))
                                };
                                let _ = out_tx.send(Ok(resp));
                                metrics::platform_events_command("invalid");
                            }
                        }
                    }
                    Ok(Some(PlatformEventsCommand { version: None })) => {
                        let resp = PlatformEventsResponse {
                            version: Some(
                                dapi_grpc::platform::v0::platform_events_response::Version::V0(
                                    PlatformEventsResponseV0 {
                                        response: Some(Resp::Error(
                                            dapi_grpc::platform::v0::PlatformErrorV0 {
                                                client_subscription_id: "".to_string(),
                                                code: 400,
                                                message: "missing version".to_string(),
                                            },
                                        )),
                                    },
                                ),
                            ),
                        };
                        let _ = out_tx.send(Ok(resp));
                        metrics::platform_events_command("invalid_version");
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let resp = PlatformEventsResponse {
                            version: Some(
                                dapi_grpc::platform::v0::platform_events_response::Version::V0(
                                    PlatformEventsResponseV0 {
                                        response: Some(Resp::Error(
                                            dapi_grpc::platform::v0::PlatformErrorV0 {
                                                client_subscription_id: "".to_string(),
                                                code: 500,
                                                message: format!("{}", e),
                                            },
                                        )),
                                    },
                                ),
                            ),
                        };
                        let _ = out_tx.send(Ok(resp));
                        metrics::platform_events_command("stream_error");
                        break;
                    }
                }
            }
        });

        Ok(Response::new(UnboundedReceiverStream::new(out_rx)))
    }
}
