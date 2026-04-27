//! Panic hook that trips the e2e cancellation token so the SPV
//! runtime + background tasks shut down cleanly when a test panics
//! during framework initialisation or test-body execution.
//!
//! The captured pre-existing hook still runs after ours — test
//! output (panic message + backtrace) must not be suppressed, only
//! augmented with the cancellation signal.

use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

/// Guards [`install`] against re-entrant or duplicate installation.
/// `std::panic::set_hook` overwrites previous hooks unconditionally;
/// without this guard a second `install` call would chain hooks
/// through `take_hook`, eventually nesting deeply.
static INSTALLED: Mutex<bool> = Mutex::new(false);

/// Install a panic hook that calls
/// [`CancellationToken::cancel`] before delegating to the previously
/// installed hook (so default panic output / backtrace is still
/// emitted).
///
/// Idempotent: repeat calls are no-ops, even with different tokens
/// — the harness installs once during init and never replaces it,
/// so a second registration would only chain hooks unnecessarily.
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
