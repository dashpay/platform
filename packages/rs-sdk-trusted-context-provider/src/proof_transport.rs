//! HTTP POSTs to proof relays.
//!
//! A relay is an untrusted byte source: whatever it returns is verified
//! against a checkpoint before anything is believed. The transport only has
//! to keep a relay from wasting the caller's time or memory, so every request
//! has a deadline, redirects are refused, and a body is read only up to
//! [`MAX_PROOF_BYTES`].
//!
//! Native builds use one `reqwest` client. Browser builds call `fetch`
//! directly, because `reqwest` on wasm32 can neither refuse redirects nor
//! bound a body without buffering it whole; those futures are not `Send`, so
//! the caller runs them on the local executor.

use dash_core_proof::MAX_PROOF_BYTES;
use futures::{Stream, StreamExt};
use std::pin::pin;
use std::time::Duration;
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use crate::error::with_causes;
#[cfg(not(target_arch = "wasm32"))]
use futures::stream::try_unfold;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::header::CONTENT_TYPE;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::redirect::Policy;

#[cfg(target_arch = "wasm32")]
use futures::future::{select, Either};
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::sleep;
#[cfg(target_arch = "wasm32")]
use js_sys::{Error as JsError, Function, Promise, Reflect, Uint8Array};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{
    AbortController, Headers, Request, RequestCache, RequestCredentials, RequestInit,
    RequestRedirect, Response,
};

/// How long connecting to a relay may take.
#[cfg(not(target_arch = "wasm32"))]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// A relay's answer: its status and at most [`MAX_PROOF_BYTES`] of body.
#[derive(Debug)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

/// Why a POST got no usable answer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PostError {
    /// The request failed or timed out before a complete answer arrived.
    Failed(String),
    /// The body is larger than a proof can be.
    TooLarge,
}

/// Read a body chunk by chunk, refusing one larger than [`MAX_PROOF_BYTES`]
/// before buffering past the limit. `declared` is the body length the
/// response announced, if any.
async fn read_capped<S, C>(declared: Option<u64>, chunks: S) -> Result<Vec<u8>, PostError>
where
    S: Stream<Item = Result<C, PostError>>,
    C: AsRef<[u8]>,
{
    if declared.is_some_and(|length| length > MAX_PROOF_BYTES as u64) {
        return Err(PostError::TooLarge);
    }
    let mut chunks = pin!(chunks);
    let mut body = Vec::new();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk?;
        let chunk = chunk.as_ref();
        if chunk.len() > MAX_PROOF_BYTES - body.len() {
            return Err(PostError::TooLarge);
        }
        body.extend_from_slice(chunk);
    }
    Ok(body)
}

/// Sends proof requests to relays.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct ProofTransport {
    client: reqwest::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProofTransport {
    /// Build the client every request of a provider shares.
    pub(crate) fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(concat!("DashSDK-verified/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client })
    }

    /// POST a JSON `body` to `url`, giving up after `timeout`.
    pub(crate) async fn post(
        &self,
        url: &Url,
        body: String,
        timeout: Duration,
    ) -> Result<Reply, PostError> {
        let failed = |error: reqwest::Error| PostError::Failed(with_causes(&error));
        let response = self
            .client
            .post(url.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .timeout(timeout)
            .send()
            .await
            .map_err(failed)?;
        let status = response.status().as_u16();
        let declared = response.content_length();
        let chunks = try_unfold(response, move |mut response| async move {
            Ok(response
                .chunk()
                .await
                .map_err(failed)?
                .map(|chunk| (chunk, response)))
        });
        let body = read_capped(declared, chunks).await?;
        Ok(Reply { status, body })
    }
}

/// Sends proof requests to relays with the browser's `fetch`.
#[cfg(target_arch = "wasm32")]
pub(crate) struct ProofTransport;

#[cfg(target_arch = "wasm32")]
impl ProofTransport {
    /// Nothing to build: every request calls the global `fetch`.
    pub(crate) fn new() -> Result<Self, reqwest::Error> {
        Ok(Self)
    }

    /// POST a JSON `body` to `url`, giving up after `timeout`.
    ///
    /// The request refuses redirects, bypasses the HTTP cache and sends no
    /// credentials. It is aborted when the deadline passes or the returned
    /// future is dropped. Must run on the local executor.
    pub(crate) async fn post(
        &self,
        url: &Url,
        body: String,
        timeout: Duration,
    ) -> Result<Reply, PostError> {
        let controller = AbortController::new().map_err(js_failure)?;
        let _abort = AbortOnDrop(controller.clone());
        let exchange = pin!(browser_post(url, body, &controller));
        let deadline = pin!(sleep(timeout));
        match select(exchange, deadline).await {
            Either::Left((reply, _)) => reply,
            Either::Right(_) => Err(PostError::Failed(format!(
                "no answer within {} ms",
                timeout.as_millis()
            ))),
        }
    }
}

/// Aborts a browser request when dropped, so an abandoned request does not
/// keep downloading.
#[cfg(target_arch = "wasm32")]
struct AbortOnDrop(AbortController);

#[cfg(target_arch = "wasm32")]
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(target_arch = "wasm32")]
fn js_failure(error: JsValue) -> PostError {
    PostError::Failed(match error.dyn_ref::<JsError>() {
        Some(error) => format!(
            "{}: {}",
            String::from(error.name()),
            String::from(error.message())
        ),
        None => error
            .as_string()
            .unwrap_or_else(|| "fetch failed".to_string()),
    })
}

/// The browser half of [`ProofTransport::post`], without the deadline.
///
/// `fetch` is looked up on the global object so the same code runs in a
/// window, a worker and Node.js.
#[cfg(target_arch = "wasm32")]
async fn browser_post(
    url: &Url,
    body: String,
    controller: &AbortController,
) -> Result<Reply, PostError> {
    let headers = Headers::new().map_err(js_failure)?;
    headers
        .set("Content-Type", "application/json")
        .map_err(js_failure)?;
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_body(&JsValue::from_str(&body));
    init.set_headers(&headers);
    init.set_redirect(RequestRedirect::Error);
    init.set_cache(RequestCache::NoStore);
    init.set_credentials(RequestCredentials::Omit);
    init.set_signal(Some(&controller.signal()));
    let request = Request::new_with_str_and_init(url.as_str(), &init).map_err(js_failure)?;

    let global = js_sys::global();
    let fetch: Function = Reflect::get(&global, &JsValue::from_str("fetch"))
        .map_err(js_failure)?
        .dyn_into()
        .map_err(|_| PostError::Failed("no global fetch".to_string()))?;
    let promise: Promise = fetch
        .call1(&global, &request)
        .map_err(js_failure)?
        .dyn_into()
        .map_err(|_| PostError::Failed("fetch did not return a promise".to_string()))?;
    let response: Response = JsFuture::from(promise)
        .await
        .map_err(js_failure)?
        .dyn_into()
        .map_err(|_| PostError::Failed("fetch did not return a response".to_string()))?;

    let status = response.status();
    let declared = response
        .headers()
        .get("Content-Length")
        .ok()
        .flatten()
        .and_then(|length| length.trim().parse::<u64>().ok());
    let Some(raw) = response.body() else {
        return Ok(Reply {
            status,
            body: Vec::new(),
        });
    };
    let chunks = wasm_streams::ReadableStream::from_raw(raw)
        .try_into_stream()
        .map_err(|_| PostError::Failed("response body is locked".to_string()))?
        .map(|chunk| {
            let chunk: Uint8Array = chunk
                .map_err(js_failure)?
                .dyn_into()
                .map_err(|_| PostError::Failed("a body chunk is not bytes".to_string()))?;
            // Refuse an oversized chunk before copying it out of JavaScript.
            if chunk.length() as usize > MAX_PROOF_BYTES {
                return Err(PostError::TooLarge);
            }
            Ok(chunk.to_vec())
        });
    let body = read_capped(declared, chunks).await?;
    Ok(Reply { status, body })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use futures::stream;

    fn chunks(sizes: &[usize]) -> impl Stream<Item = Result<Vec<u8>, PostError>> {
        stream::iter(
            sizes
                .iter()
                .map(|size| Ok(vec![7; *size]))
                .collect::<Vec<_>>(),
        )
    }

    #[tokio::test]
    async fn should_read_a_body_up_to_the_proof_limit() {
        let body = read_capped(None, chunks(&[MAX_PROOF_BYTES - 1, 1]))
            .await
            .expect("a body at the limit");
        assert_eq!(body.len(), MAX_PROOF_BYTES);
    }

    #[tokio::test]
    async fn should_refuse_a_body_past_the_proof_limit_without_buffering_it() {
        let endless = stream::repeat_with(|| Ok::<_, PostError>(vec![0; 4096]));
        assert_eq!(read_capped(None, endless).await, Err(PostError::TooLarge));
        assert_eq!(
            read_capped(None, chunks(&[MAX_PROOF_BYTES + 1])).await,
            Err(PostError::TooLarge)
        );
    }

    #[tokio::test]
    async fn should_refuse_a_declared_length_past_the_proof_limit_before_reading() {
        let unreadable =
            stream::once(async { Err::<Vec<u8>, _>(PostError::Failed("read".to_string())) });
        assert_eq!(
            read_capped(Some(MAX_PROOF_BYTES as u64 + 1), unreadable).await,
            Err(PostError::TooLarge)
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Replace the global `fetch` with a function of `request`.
    fn mock_fetch(body: &str) {
        let fetch = Function::new_with_args("request", body);
        Reflect::set(&js_sys::global(), &JsValue::from_str("fetch"), &fetch)
            .expect("set the global fetch");
    }

    fn seen(field: &str) -> JsValue {
        let seen = Reflect::get(&js_sys::global(), &JsValue::from_str("__seen")).expect("seen");
        Reflect::get(&seen, &JsValue::from_str(field)).expect(field)
    }

    fn relay() -> Url {
        Url::parse("https://relay.test/proofs").expect("a URL")
    }

    #[wasm_bindgen_test]
    async fn should_post_json_without_redirects_cache_or_credentials_and_stream_the_body() {
        mock_fetch(
            r#"
            globalThis.__seen = {
                method: request.method,
                redirect: request.redirect,
                cache: request.cache,
                credentials: request.credentials,
                type: request.headers.get("content-type"),
            };
            return request.text().then((text) => {
                globalThis.__seen.body = text;
                const chunks = [new Uint8Array([1, 2]), new Uint8Array([3])];
                const stream = new ReadableStream({
                    pull(controller) {
                        const next = chunks.shift();
                        if (next) controller.enqueue(next); else controller.close();
                    },
                });
                return new Response(stream, { status: 200 });
            });
            "#,
        );
        let reply = ProofTransport
            .post(
                &relay(),
                r#"{"height":1}"#.to_string(),
                Duration::from_secs(5),
            )
            .await
            .expect("a reply");
        assert_eq!((reply.status, reply.body), (200, vec![1, 2, 3]));
        for (field, expected) in [
            ("method", "POST"),
            ("redirect", "error"),
            ("cache", "no-store"),
            ("credentials", "omit"),
            ("type", "application/json"),
            ("body", r#"{"height":1}"#),
        ] {
            assert_eq!(
                seen(field).as_string().as_deref(),
                Some(expected),
                "{field}"
            );
        }
    }

    #[wasm_bindgen_test]
    async fn should_refuse_an_endless_body_without_a_declared_length() {
        mock_fetch(
            r#"
            const stream = new ReadableStream({
                pull(controller) { controller.enqueue(new Uint8Array(65536)); },
            });
            return Promise.resolve(new Response(stream, { status: 200 }));
            "#,
        );
        let reply = ProofTransport
            .post(&relay(), "{}".to_string(), Duration::from_secs(5))
            .await;
        assert_eq!(reply.err(), Some(PostError::TooLarge));
    }

    #[wasm_bindgen_test]
    async fn should_refuse_a_declared_length_past_the_limit() {
        mock_fetch(
            r#"
            return Promise.resolve(new Response("", {
                status: 200,
                headers: { "content-length": "99999999" },
            }));
            "#,
        );
        let reply = ProofTransport
            .post(&relay(), "{}".to_string(), Duration::from_secs(5))
            .await;
        assert_eq!(reply.err(), Some(PostError::TooLarge));
    }

    #[wasm_bindgen_test]
    async fn should_abort_the_request_at_the_deadline() {
        mock_fetch(
            r#"
            globalThis.__aborted = false;
            return new Promise((_, reject) => {
                request.signal.addEventListener("abort", () => {
                    globalThis.__aborted = true;
                    reject(new Error("aborted"));
                });
            });
            "#,
        );
        let reply = ProofTransport
            .post(&relay(), "{}".to_string(), Duration::from_millis(50))
            .await;
        assert!(
            matches!(&reply, Err(PostError::Failed(reason)) if reason.contains("no answer within")),
            "{reply:?}"
        );
        let aborted =
            Reflect::get(&js_sys::global(), &JsValue::from_str("__aborted")).expect("the flag");
        assert_eq!(aborted.as_bool(), Some(true));
    }
}
