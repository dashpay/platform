//! Panic hook that trips the e2e cancellation token so SPV /
//! background tasks shut down cleanly. Delegates to the previous
//! hook so panic message + backtrace still surface.

use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

/// Guards against duplicate installation — without it repeat
/// calls would deeply nest hooks via `take_hook`.
static INSTALLED: Mutex<bool> = Mutex::new(false);

/// Install a panic hook that calls [`CancellationToken::cancel`]
/// before delegating to the previous hook. Idempotent across
/// repeat calls (even with different tokens).
pub fn install(cancel_token: CancellationToken) {
    let mut guard = match INSTALLED.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if *guard {
        tracing::debug!(
            target: "platform_wallet::e2e::panic_hook",
            "panic hook already installed; skipping re-registration"
        );
        return;
    }

    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        cancel_token.cancel();
        prev(info);
    }));
    *guard = true;

    tracing::debug!(
        target: "platform_wallet::e2e::panic_hook",
        "installed cancellation panic hook"
    );
}
