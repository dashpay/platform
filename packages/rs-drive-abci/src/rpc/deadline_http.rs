use base64::Engine;
use dpp::dashcore_rpc::jsonrpc::{self, Request, Response, Transport};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// The largest answer read. `quorum info` for a 100-member quorum is tens of kilobytes.
const MAX_ANSWER_BYTES: usize = 4 * 1024 * 1024;

/// The most written in one call. Some systems bound only each wait for buffer space by the
/// socket's write timeout, not a whole call, so a large write to a peer that keeps taking a little
/// could outlast the deadline; a write this small cannot by much.
const WRITE_CHUNK: usize = 16 * 1024;

/// The port a URL without a scheme or port is assumed to use, as the other Core RPC clients
/// assume; with `http://` it is 80.
const FALLBACK_PORT: u16 = 8332;

/// A JSON-RPC transport to Core that sends each request once, over a connection of its own, and
/// gives up when the whole exchange has not finished by its deadline: a Core that is slow, sends
/// its answer a byte at a time, never answers or closes the connection midway costs the caller
/// at most the deadline. The request asks Core to close the connection after answering, so an
/// answer ends at its `Content-Length` or, without one, where Core closes the connection.
pub(crate) struct DeadlineHttpTransport {
    address: SocketAddr,
    /// The URL's authority, sent as the `Host` header.
    host: String,
    path: String,
    authorization: String,
    deadline: Duration,
}

impl DeadlineHttpTransport {
    /// A transport to the Core RPC server at `url`, read by the rules the other Core RPC clients
    /// use: `host:port`, optionally with `http://` (port 80 by default) in front, user
    /// information, which is ignored, and a path. Resolved now.
    pub(crate) fn new(
        url: &str,
        username: &str,
        password: &str,
        deadline: Duration,
    ) -> io::Result<Self> {
        let (after_scheme, fallback_port) = match url.split_once("://") {
            None => (url, FALLBACK_PORT),
            Some(("http", after_scheme)) => (after_scheme, 80),
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Core RPC url {url} must be plain http"),
                ))
            }
        };
        let (authority, path) = match after_scheme.find('/') {
            Some(slash) => after_scheme.split_at(slash),
            None => (after_scheme, "/"),
        };
        let host = authority
            .split_once('@')
            .map_or(authority, |(_, host)| host);
        let address = host
            .to_socket_addrs()
            .or_else(|_| format!("{host}:{fallback_port}").to_socket_addrs())?
            .next()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Core RPC url {url} names no address"),
                )
            })?;
        let credentials = base64::engine::general_purpose::STANDARD
            .encode(format!("{username}:{password}").as_bytes());
        Ok(Self {
            address,
            host: host.to_string(),
            path: path.to_string(),
            authorization: format!("Basic {credentials}"),
            deadline,
        })
    }

    /// Sends `request` and reads Core's answer, all by the deadline.
    fn exchange(&self, request: &Request) -> io::Result<Response> {
        let deadline = Deadline::after(self.deadline);

        let body = serde_json::to_vec(request)?;
        let mut message = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.host,
            self.authorization,
            body.len()
        )
        .into_bytes();
        message.extend_from_slice(&body);
        let mut stream = TcpStream::connect_timeout(&self.address, deadline.remaining()?)?;
        write_by(&mut stream, &message, &deadline)?;

        let mut answer = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            stream.set_read_timeout(Some(deadline.remaining()?))?;
            let read = match stream.read(&mut buffer) {
                Ok(read) => read,
                // A signal cut the wait short; the deadline still decides how long is left.
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(deadline.explain(error)),
            };
            if read == 0 {
                break;
            }
            answer.extend_from_slice(&buffer[..read]);
            if answer.len() > MAX_ANSWER_BYTES {
                return Err(invalid_answer("Core's answer is too large"));
            }
            if let Some(length) = answer_length(&answer)? {
                if answer.len() >= length {
                    answer.truncate(length);
                    break;
                }
            }
        }
        read_answer(&answer)
    }
}

/// The time an exchange with Core must have finished by.
struct Deadline {
    started: Instant,
    budget: Duration,
}

impl Deadline {
    fn after(budget: Duration) -> Self {
        Self {
            started: Instant::now(),
            budget,
        }
    }

    /// What is left of the deadline, or a timeout error once it has passed.
    fn remaining(&self) -> io::Result<Duration> {
        self.budget
            .checked_sub(self.started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| self.passed())
    }

    /// The error for an exchange the deadline cut off.
    fn passed(&self) -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "the exchange with Core did not finish within {:?}",
                self.budget
            ),
        )
    }

    /// `error`, or the deadline's own error when it is a socket timeout, which the deadline set.
    fn explain(&self, error: io::Error) -> io::Error {
        match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => self.passed(),
            _ => error,
        }
    }
}

/// Writes all of `bytes` by `deadline`, at most [`WRITE_CHUNK`] at a time. The socket's write
/// timeout is set to what is left of the deadline before every write, so a peer that takes the
/// bytes a little at a time cannot stretch the request past it.
fn write_by(stream: &mut TcpStream, bytes: &[u8], deadline: &Deadline) -> io::Result<()> {
    let mut written = 0;
    while written < bytes.len() {
        stream.set_write_timeout(Some(deadline.remaining()?))?;
        let end = bytes.len().min(written + WRITE_CHUNK);
        match stream.write(&bytes[written..end]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "Core stopped taking the request",
                ))
            }
            Ok(count) => written += count,
            // A signal cut the wait short; the deadline still decides how long is left.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(deadline.explain(error)),
        }
    }
    Ok(())
}

impl Transport for DeadlineHttpTransport {
    fn send_request(&self, request: Request) -> Result<Response, jsonrpc::Error> {
        self.exchange(&request)
            .map_err(|error| jsonrpc::Error::Transport(Box::new(error)))
    }

    fn send_batch(&self, _: &[Request]) -> Result<Vec<Response>, jsonrpc::Error> {
        Err(jsonrpc::Error::Transport(Box::new(io::Error::new(
            io::ErrorKind::Unsupported,
            "this transport sends single requests only",
        ))))
    }

    fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http://{}{}", self.host, self.path)
    }
}

fn invalid_answer(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.to_string())
}

/// Where the headers of `answer` end, if they have.
fn headers_end(answer: &[u8]) -> Option<usize> {
    answer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

/// The value of header `name` in `headers`, matched without regard to case.
fn header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.split("\r\n").skip(1).find_map(|line| {
        let (field, value) = line.split_once(':')?;
        field
            .trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim())
    })
}

/// The full length of `answer` once its headers are in and give a `Content-Length`; `None`
/// while the headers are incomplete or when the answer ends where Core closes the connection.
fn answer_length(answer: &[u8]) -> io::Result<Option<usize>> {
    let Some(end) = headers_end(answer) else {
        return Ok(None);
    };
    let headers = std::str::from_utf8(&answer[..end])
        .map_err(|_| invalid_answer("Core's answer headers are not text"))?;
    if header(headers, "transfer-encoding").is_some() {
        return Err(invalid_answer("Core's answer uses a transfer encoding"));
    }
    header(headers, "content-length")
        .map(|length| {
            length
                .parse::<usize>()
                .ok()
                .and_then(|length| end.checked_add(length))
                .ok_or_else(|| invalid_answer("Core's answer has an invalid Content-Length"))
        })
        .transpose()
}

/// The JSON-RPC response in Core's complete HTTP `answer`. Core sends an RPC error in the body
/// of a non-200 answer, so the body is read first and the status only explains a body that is
/// not a response.
fn read_answer(answer: &[u8]) -> io::Result<Response> {
    let end = headers_end(answer).ok_or_else(|| {
        invalid_answer("Core closed the connection before the end of its answer's headers")
    })?;
    let headers = std::str::from_utf8(&answer[..end])
        .map_err(|_| invalid_answer("Core's answer headers are not text"))?;
    let status = headers
        .strip_prefix("HTTP/1.1 ")
        .or_else(|| headers.strip_prefix("HTTP/1.0 "))
        .and_then(|rest| rest.get(..3))
        .ok_or_else(|| invalid_answer("Core's answer has no HTTP status line"))?;
    if let Some(length) = header(headers, "content-length") {
        let length: usize = length
            .parse()
            .map_err(|_| invalid_answer("Core's answer has an invalid Content-Length"))?;
        if answer.len() - end < length {
            return Err(invalid_answer(
                "Core closed the connection before the end of its answer",
            ));
        }
    }
    let body = &answer[end..];
    serde_json::from_slice::<Response>(body).map_err(|error| {
        if status == "200" {
            invalid_answer(&format!(
                "Core's answer is not a JSON-RPC response: {error}"
            ))
        } else {
            invalid_answer(&format!("Core answered with HTTP status {status}"))
        }
    })
}

/// A fake Core RPC server for tests: it reads each request and answers as it is told, and stops
/// when asked, closing any connection it holds.
#[cfg(test)]
pub(crate) mod fake_core {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc::{self, Sender, TryRecvError};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    /// How the fake Core answers a request.
    pub(crate) struct Reply {
        /// The bytes sent back.
        pub(crate) bytes: Vec<u8>,
        /// Sent a byte at a time with this pause in between, when set.
        pub(crate) pace: Option<Duration>,
        /// Whether the connection stays open after the bytes, until the server stops.
        pub(crate) hold_open: bool,
    }

    impl Reply {
        /// `body` as a complete HTTP answer with `status`, after which the connection closes.
        pub(crate) fn http(status: &str, body: &str) -> Self {
            Self {
                bytes: format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .into_bytes(),
                pace: None,
                hold_open: false,
            }
        }

        /// `bytes` as they are, after which the connection closes.
        pub(crate) fn raw(bytes: &str) -> Self {
            Self {
                bytes: bytes.as_bytes().to_vec(),
                pace: None,
                hold_open: false,
            }
        }

        /// No answer; the connection stays open.
        pub(crate) fn silence() -> Self {
            Self {
                bytes: Vec::new(),
                pace: None,
                hold_open: true,
            }
        }
    }

    /// A running fake Core.
    pub(crate) struct FakeCore {
        address: SocketAddr,
        stop: Sender<()>,
        server: JoinHandle<Vec<String>>,
    }

    impl FakeCore {
        /// Starts a fake Core that answers each request body with `reply`.
        pub(crate) fn start(reply: impl Fn(&serde_json::Value) -> Reply + Send + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("expected a local port");
            listener
                .set_nonblocking(true)
                .expect("expected a non-blocking listener");
            let address = listener.local_addr().expect("expected the local address");
            let (stop, stopped) = mpsc::channel();
            let server = thread::spawn(move || {
                let mut requests = Vec::new();
                let mut held = Vec::new();
                while let Err(TryRecvError::Empty) = stopped.try_recv() {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let request = read_request(&mut stream);
                            let body = request
                                .split_once("\r\n\r\n")
                                .map(|(_, body)| body)
                                .unwrap_or_default();
                            let body = serde_json::from_str(body).unwrap_or_default();
                            let Reply {
                                bytes,
                                pace,
                                hold_open,
                            } = reply(&body);
                            requests.push(request);
                            match pace {
                                Some(pace) => {
                                    for byte in bytes {
                                        if stream.write_all(&[byte]).is_err() {
                                            break;
                                        }
                                        thread::sleep(pace);
                                    }
                                }
                                None => {
                                    let _ = stream.write_all(&bytes);
                                }
                            }
                            if hold_open {
                                held.push(stream);
                            }
                        }
                        Err(_) => thread::sleep(Duration::from_millis(5)),
                    }
                }
                requests
            });
            Self {
                address,
                stop,
                server,
            }
        }

        /// The fake Core's RPC url, as Drive's config gives it.
        pub(crate) fn url(&self) -> String {
            self.address.to_string()
        }

        /// Stops the fake Core, closing the connections it held, and returns the requests it
        /// read.
        pub(crate) fn stop(self) -> Vec<String> {
            let _ = self.stop.send(());
            self.server
                .join()
                .expect("expected the fake Core to stop cleanly")
        }
    }

    /// Reads one HTTP request: its headers and the `Content-Length` bytes of body after them.
    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_nonblocking(false)
            .expect("expected a blocking stream");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("expected a read timeout");
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(read @ 1..) = stream.read(&mut buffer) {
            request.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&request);
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (field, value) = line.split_once(':')?;
                        field
                            .eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                if body.len() >= length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&request).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::fake_core::{FakeCore, Reply};
    use super::*;
    use serde_json::json;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;

    /// Short, so a test that waits it out stays quick.
    const TEST_DEADLINE: Duration = Duration::from_secs(1);
    /// What a test allows beyond the deadline for a busy machine.
    const MARGIN: Duration = Duration::from_secs(2);

    fn client(core: &FakeCore) -> jsonrpc::Client {
        let transport = DeadlineHttpTransport::new(&core.url(), "user", "pass", TEST_DEADLINE)
            .expect("expected a transport to the fake Core");
        jsonrpc::Client::with_transport(transport)
    }

    fn ask(client: &jsonrpc::Client) -> Result<Response, jsonrpc::Error> {
        let request = client.build_request("getblockcount", None);
        client.send_request(request)
    }

    /// A complete answer is read to its `Content-Length` and returned.
    #[test]
    fn should_return_the_response_core_answers_with() {
        let core = FakeCore::start(|request| {
            Reply::http(
                "200 OK",
                &json!({"result": 7, "error": null, "id": request["id"]}).to_string(),
            )
        });

        let response = ask(&client(&core)).expect("expected Core's answer");

        assert_eq!(response.result::<u64>().expect("expected a number"), 7);
        let requests = core.stop();
        assert_eq!(requests.len(), 1, "one request");
        assert!(requests[0].starts_with("POST / HTTP/1.1\r\n"));
        // "user:pass" in base64.
        assert!(requests[0].contains("Authorization: Basic dXNlcjpwYXNz\r\n"));
        assert!(requests[0].contains("\"method\":\"getblockcount\""));
    }

    /// Core reports an RPC error in the body of an HTTP 500; the caller gets the RPC error.
    #[test]
    fn should_return_the_rpc_error_core_answers_with() {
        let core = FakeCore::start(|request| {
            Reply::http(
                "500 Internal Server Error",
                &json!({"result": null, "error": {"code": -8, "message": "quorum not found"}, "id": request["id"]})
                    .to_string(),
            )
        });

        let response = ask(&client(&core)).expect("expected Core's answer");

        assert_eq!(response.error.expect("expected the RPC error").code, -8);
        core.stop();
    }

    /// A connection Core closes before the end of the headers ends the request at once with an
    /// error, rather than waiting for headers that cannot come.
    #[test]
    fn should_fail_at_once_when_core_closes_the_connection_in_the_headers() {
        let core = FakeCore::start(|_| {
            Reply::raw("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n")
        });

        let started = Instant::now();
        let answer = ask(&client(&core));
        let waited = started.elapsed();

        assert!(answer.is_err());
        assert!(waited < TEST_DEADLINE, "waited {waited:?}");
        assert_eq!(core.stop().len(), 1);
    }

    /// A connection Core closes before the body is complete is an error, not a short body.
    #[test]
    fn should_fail_when_core_closes_the_connection_in_the_body() {
        let core = FakeCore::start(|_| {
            Reply::raw("HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{\"result\":")
        });

        let answer = ask(&client(&core));

        assert!(answer.is_err());
        assert_eq!(core.stop().len(), 1);
    }

    /// A `Content-Length` too large to add to the headers' length is refused, not wrapped.
    #[test]
    fn should_refuse_a_content_length_that_overflows() {
        let core = FakeCore::start(|_| {
            Reply::raw("HTTP/1.1 200 OK\r\nContent-Length: 18446744073709551615\r\n\r\n{}")
        });

        let answer = ask(&client(&core));

        assert!(answer.is_err());
        assert_eq!(core.stop().len(), 1);
    }

    /// A Core that never answers costs the deadline, and the request is sent once.
    #[test]
    fn should_give_up_on_a_core_that_never_answers_by_the_deadline() {
        let core = FakeCore::start(|_| Reply::silence());

        let started = Instant::now();
        let answer = ask(&client(&core));
        let waited = started.elapsed();

        assert!(answer.is_err());
        assert!(waited >= TEST_DEADLINE, "gave up after {waited:?}");
        assert!(waited < TEST_DEADLINE + MARGIN, "waited {waited:?}");
        assert_eq!(core.stop().len(), 1, "one request, never sent again");
    }

    /// The deadline covers the whole answer: a Core sending it a byte at a time, each well within
    /// any single read's wait, still costs no more than the deadline.
    #[test]
    fn should_give_up_on_an_answer_that_trickles_past_the_deadline() {
        let core = FakeCore::start(|request| {
            let mut reply = Reply::http(
                "200 OK",
                &json!({"result": 7, "error": null, "id": request["id"]}).to_string(),
            );
            reply.pace = Some(Duration::from_millis(20));
            reply
        });

        let started = Instant::now();
        let answer = ask(&client(&core));
        let waited = started.elapsed();

        assert!(answer.is_err());
        assert!(waited < TEST_DEADLINE + MARGIN, "waited {waited:?}");
        assert_eq!(core.stop().len(), 1);
    }

    /// The deadline bounds the whole request, not each write: a Core that takes the request a
    /// little at a time cannot stretch it past the deadline.
    #[test]
    fn should_give_up_writing_to_a_core_that_takes_the_request_too_slowly() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("expected a local port");
        let address = listener.local_addr().expect("expected the local address");
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        // Takes about 5 KB a second.
        let server = thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_millis(10)));
            let mut buffer = [0u8; 256];
            while !stopped.load(Ordering::SeqCst) {
                let _ = stream.read(&mut buffer);
                thread::sleep(Duration::from_millis(50));
            }
        });
        let mut stream = TcpStream::connect(address).expect("expected to connect");
        let request = vec![1u8; 64 * 1024 * 1024];

        let started = Instant::now();
        let written = write_by(&mut stream, &request, &Deadline::after(TEST_DEADLINE));
        let waited = started.elapsed();
        stop.store(true, Ordering::SeqCst);
        drop(stream);
        server.join().expect("expected the slow reader to stop");

        let error = written.expect_err("expected the deadline to stop the request");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut, "{error}");
        assert!(
            waited + Duration::from_millis(50) >= TEST_DEADLINE,
            "gave up after {waited:?}"
        );
        assert!(waited < TEST_DEADLINE + MARGIN, "waited {waited:?}");
    }

    /// Drive's config gives Core's address as `host:port`; a scheme and a path are accepted too.
    #[test]
    fn should_read_the_address_and_path_of_the_url() {
        let plain = DeadlineHttpTransport::new("127.0.0.1:9998", "u", "p", TEST_DEADLINE)
            .expect("expected host:port");
        assert_eq!(plain.address, "127.0.0.1:9998".parse().unwrap());
        assert_eq!(plain.path, "/");
        assert_eq!(plain.host, "127.0.0.1:9998");

        let full =
            DeadlineHttpTransport::new("http://127.0.0.1:9998/wallet", "u", "p", TEST_DEADLINE)
                .expect("expected a full url");
        assert_eq!(full.address, "127.0.0.1:9998".parse().unwrap());
        assert_eq!(full.path, "/wallet");

        let with_user =
            DeadlineHttpTransport::new("http://user:pw@127.0.0.1:9998", "u", "p", TEST_DEADLINE)
                .expect("expected user information to be ignored");
        assert_eq!(with_user.address, "127.0.0.1:9998".parse().unwrap());
        assert_eq!(with_user.host, "127.0.0.1:9998");

        let http_default = DeadlineHttpTransport::new("http://127.0.0.1", "u", "p", TEST_DEADLINE)
            .expect("expected http without a port");
        assert_eq!(http_default.address, "127.0.0.1:80".parse().unwrap());

        assert!(
            DeadlineHttpTransport::new("https://127.0.0.1:9998", "u", "p", TEST_DEADLINE).is_err()
        );
    }
}
