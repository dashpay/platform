//! Panic hook that trips the e2e cancellation token so the SPV
//! runtime + background tasks shut down cleanly when a test panics
//! during framework initialisation or test-body execution.
//!
//! The captured pre-existing hook still runs after ours — test
//! output (panic message + backtrace) must not be suppressed, only
//! augmented with the cancellation signal.
//!
//! Wave 2 stub. Wave 3 wires `std::panic::set_hook(Box::new(...))`
//! after capturing the existing hook and accepts a real
//! `tokio_util::sync::CancellationToken`.

/// Install the cancellation panic hook.
///
/// Wave 2 stub: accepts a placeholder unit type. Wave 3 changes the
/// signature to `install(cancel_token: CancellationToken)` and
/// performs the actual hook installation. Calling the stub is
/// harmless — it does nothing.
pub fn install(_cancel_token: ()) {
    // Wave 3 wires the actual hook installation:
    //
    //   let prev = std::panic::take_hook();
    //   std::panic::set_hook(Box::new(move |info| {
    //       cancel_token.cancel();
    //       prev(info);
    //   }));
}
