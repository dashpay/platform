use std::ffi::CStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_COMMIT: &str = env!("PLATFORM_WALLET_GIT_COMMIT");
const GIT_DIRTY: &str = env!("PLATFORM_WALLET_GIT_DIRTY");

/// Install the global `tracing` subscriber. Returns
/// `true` only when this call installed the subscriber.
///
/// A `false` return covers both (a) a subscriber was already
/// installed (first init wins) and (b) path couldn't be opened.
///
/// # Safety
/// - `path` must be a valid pointer to a null terminated string
///   or null.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_enable_file_logging(
    level: u8,
    path: *const std::ffi::c_char,
) -> bool {
    if path.is_null() {
        return false;
    }

    let Some(path) = CStr::from_ptr(path).to_str().ok().map(PathBuf::from) else {
        return false;
    };

    enable_file_logging(level_to_directive(level), &path)
}

/// Level for the network-diagnostics targets (`rs_dapi_client`,
/// `rs_sdk_trusted_context_provider`): their useful events (per-request
/// execution, address ban/unban, quorum cache misses) sit at `debug`, so
/// they get at least that regardless of the caller's global level — but a
/// caller asking for `trace` still gets `trace`.
fn diag_level(log_level: &str) -> &str {
    if log_level == "trace" {
        "trace"
    } else {
        "debug"
    }
}

fn enable_file_logging(log_level: &str, path: &Path) -> bool {
    let Some(f_sdk) = open_file(path.join("dash_sdk").join("run.log")) else {
        return false;
    };
    let Some(f_sdk_metrics) = open_file(path.join("dash_sdk").join("metrics.log")) else {
        return false;
    };
    let Some(f_pw) = open_file(path.join("platform_wallet").join("run.log")) else {
        return false;
    };
    let Some(f_pw_metrics) = open_file(path.join("platform_wallet").join("metrics.log")) else {
        return false;
    };
    let Some(f_spv) = open_file(path.join("dash_spv").join("run.log")) else {
        return false;
    };
    let Some(f_kw) = open_file(path.join("key_wallet").join("run.log")) else {
        return false;
    };
    let Some(f_grpc) = open_file(path.join("grpc").join("run.log")) else {
        return false;
    };

    let l_sdk = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_sdk))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "dash_sdk={log_level},rs_sdk_ffi={log_level},rs_sdk_ffi::metrics=off"
        )));

    let l_sdk_metrics = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_sdk_metrics))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "rs_sdk_ffi::metrics={log_level}"
        )));

    let l_pw = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_pw))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "platform_wallet={log_level},platform_wallet_ffi={log_level},\
             platform_wallet_ffi::metrics=off"
        )));

    let l_pw_metrics = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_pw_metrics))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "platform_wallet_ffi::metrics={log_level}"
        )));

    let l_spv = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_spv))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "dash_spv={log_level}"
        )));

    let l_kw = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_kw))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "key_wallet={log_level}"
        )));

    let l_grpc = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(f_grpc))
        .with_ansi(false)
        .with_filter(tracing_subscriber::EnvFilter::new(format!(
            "dapi_grpc={log_level},tonic={log_level},h2={log_level},\
             hyper={log_level},tower={log_level},\
             rs_dapi_client={diag},rs_sdk_trusted_context_provider={diag}",
            diag = diag_level(log_level)
        )));

    if fs::write(path.join("build_info.txt"), build_info_string()).is_err() {
        return false;
    }

    let stdout_layer = tracing_subscriber::fmt::layer().with_filter(broad_env_filter(log_level));

    let layers: Vec<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>> = vec![
        stdout_layer.boxed(),
        l_sdk.boxed(),
        l_sdk_metrics.boxed(),
        l_pw.boxed(),
        l_pw_metrics.boxed(),
        l_spv.boxed(),
        l_kw.boxed(),
        l_grpc.boxed(),
    ];

    if tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .is_err()
    {
        return false;
    }

    tracing::info!(level = log_level, "file logging enabled");
    true
}

fn open_file(path: PathBuf) -> Option<fs::File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()
}

fn broad_env_filter(log_level: &str) -> tracing_subscriber::EnvFilter {
    let directives = format!(
        "dash_sdk={log_level},rs_sdk={log_level},rs_sdk_ffi={log_level},\
         platform_wallet={log_level},platform_wallet_ffi={log_level},\
         dash_spv={log_level},key_wallet={log_level},\
         dapi_grpc={log_level},h2={log_level},tower={log_level},\
         hyper={log_level},tonic={log_level},\
         rs_dapi_client={diag},rs_sdk_trusted_context_provider={diag}",
        diag = diag_level(log_level)
    );

    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(directives))
}

fn level_to_directive(level: u8) -> &'static str {
    match level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    }
}

fn build_info_string() -> String {
    let dirty = match GIT_DIRTY {
        "0" => "no",
        "1" => "yes",
        _ => "unknown",
    };
    let mut out = format!(
        "platform-wallet-version: {VERSION}\n\
         git-commit: {GIT_COMMIT}\n\
         git-dirty: {dirty}\n\
         # to reproduce: git checkout {GIT_COMMIT}\n"
    );
    if dirty == "yes" {
        out.push_str(
            "# WARNING: this build had uncommitted changes; the commit hash above does NOT \
             fully describe the source state.\n",
        );
    }
    out
}
