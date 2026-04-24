//! Signer interface for iOS FFI
//!
//! # Async completion-callback signer (BREAKING CHANGE from the pre-async design)
//!
//! The `Signer<K>` trait in `rs-dpp` is now `async`, which means iOS (or any
//! FFI caller) can implement *truly* async signing — for example, calling the
//! iOS Secure Enclave or showing a biometric prompt — without blocking any
//! Tokio worker thread.
//!
//! To make this work across the C ABI we use a **completion-callback** pattern
//! instead of the old synchronous `sign` return-value pattern:
//!
//! 1. Rust calls `SignAsyncCallback` with a `completion_ctx` token and a
//!    `SignCompletionCallback` function pointer.
//! 2. The C / Swift / Kotlin implementation returns **immediately** from the
//!    `SignAsyncCallback` — it has not signed yet. It stashes the `completion_ctx`
//!    and `completion` callback somewhere (e.g. captured in a Swift closure).
//! 3. When signing finishes (possibly on another thread, possibly minutes
//!    later after a biometric prompt), the caller invokes `completion(ctx, ...)`
//!    with either a signature or an error message.
//! 4. The Rust side (see `dash_sdk_sign_async_completion` below) converts the completion
//!    args to a `Result<BinaryData, ProtocolError>` and wakes the awaiting
//!    `async fn sign` via a `tokio::sync::oneshot`. No thread is blocked during
//!    the wait.
//!
//! ## iOS migration notes (from the old vtable)
//!
//! The old sync vtable returned a `*mut u8` signature buffer directly and took
//! a `free_result` deallocator slot. Both of those are gone:
//!
//! - `SignerVTable::sign`  → replaced with `SignerVTable::sign_async`
//! - `SignerVTable::free_result` → **removed**. The Rust side now *copies* the
//!   signature bytes out of the completion call before returning, so the iOS
//!   side owns its buffer start-to-finish and frees it with its own allocator.
//! - `SignCallback` type → replaced with `SignAsyncCallback`
//! - `dash_sdk_signer_create` signature changed — see the function docs below.
//!
//! Callers of `dash_sdk_signer_create` (notably the Swift SDK) **must** be
//! updated before they will compile. Until then, iOS HSM signing from the
//! old Swift code will not work.

use crate::types::SignerHandle;
use async_trait::async_trait;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::prelude::{IdentityPublicKey, ProtocolError};
use simple_signer::SingleKeySigner;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

/// Upper bound on how long the Rust side will wait for an iOS / FFI signer
/// to invoke its completion callback after `sign_async` is called.
///
/// If the caller never invokes `completion` (a contract violation) the
/// awaiting `async fn sign` would otherwise hang forever because the
/// `oneshot::Sender` is inside a `Box` that was handed to the C side and
/// never dropped. This timeout bounds the damage to a single leaked
/// `Box<oneshot::Sender<_>>` per stuck request and surfaces a recoverable
/// `ProtocolError` to the caller instead of wedging the runtime.
///
/// Five minutes is generous enough for biometric prompts + HSM round-trips
/// while still being short enough that runaway tasks eventually fail.
const SIGN_ASYNC_COMPLETION_TIMEOUT: Duration = Duration::from_secs(300);

/// C-compatible async vtable for signers.
///
/// `sign_async` returns immediately; the caller is expected to eventually
/// invoke the supplied `SignCompletionCallback` with the signing result.
/// `can_sign_with` remains synchronous — checking whether a key is available
/// is fast and doesn't need async.
#[repr(C)]
pub struct SignerVTable {
    /// Async sign function pointer.
    ///
    /// Implementations MUST eventually call the supplied `completion` exactly
    /// once with the supplied `completion_ctx`. Failing to call it will cause
    /// the awaiting `async fn sign` on the Rust side to hang forever.
    pub sign_async: SignAsyncCallback,

    /// Can-sign-with function pointer. Synchronous.
    pub can_sign_with: CanSignCallback,

    /// Destructor function pointer for the `signer_ptr` state.
    pub destroy: unsafe extern "C" fn(signer: *mut c_void),
}

/// Function pointer type for the async sign callback.
///
/// Invoked by the Rust side whenever a signature is needed. Implementations
/// MUST return immediately and MUST arrange for `completion` to be called
/// exactly once with the matching `completion_ctx`. It is valid (and
/// expected, for things like biometric-gated HSM signing) for the completion
/// to run on a different thread than `SignAsyncCallback` itself.
///
/// # Safety
/// - `key_bytes` / `data` are only valid for the duration of this call.
///   If the implementation needs them after returning, it must copy them.
/// - `completion_ctx` is opaque; it must be passed verbatim to `completion`.
/// - `completion` must be called exactly once.
pub type SignAsyncCallback = unsafe extern "C" fn(
    signer: *const c_void,
    key_bytes: *const u8,
    key_len: usize,
    data: *const u8,
    data_len: usize,
    completion_ctx: *mut c_void,
    completion: SignCompletionCallback,
);

/// Completion callback invoked by the C / iOS side when a signature is ready.
///
/// # Parameters
/// - `completion_ctx`: the exact token passed in by the Rust side.
/// - `signature` / `signature_len`: signature bytes on success. Ignored when
///   `error_message` is non-null. May be null with `signature_len == 0` if
///   `error_message` is non-null.
/// - `error_message`: null-terminated UTF-8 error string on failure, null on
///   success. Ownership is *not* transferred — the Rust side copies the string
///   before returning, so the caller can free/reuse the buffer immediately
///   after the completion returns.
///
/// # Safety
/// - Must be called at most once per `completion_ctx`.
/// - The `signature` buffer may be freed/reused as soon as this function
///   returns — the Rust side copies the bytes into an owned `Vec<u8>` before
///   the call returns.
pub type SignCompletionCallback = unsafe extern "C" fn(
    completion_ctx: *mut c_void,
    signature: *const u8,
    signature_len: usize,
    error_message: *const c_char,
);

/// Function pointer type for the synchronous `can_sign_with` callback.
pub type CanSignCallback =
    unsafe extern "C" fn(signer: *const c_void, key_bytes: *const u8, key_len: usize) -> bool;

/// Optional destructor callback (may be NULL from C).
pub type DestroyCallback = Option<unsafe extern "C" fn(signer: *mut c_void)>;

/// Payload passed through `completion_ctx`. Sent over the oneshot channel
/// back to the awaiting Rust `async fn sign`.
type SignResult = Result<Vec<u8>, ProtocolError>;

/// `completion_ctx` payload. First call swaps `sender` to null and sends;
/// duplicates see null and are no-ops. Slot is leaked per `sign_async` call
/// (small bounded cost) so late / duplicate FFI completions stay defined.
struct CompletionSlot {
    sender: AtomicPtr<oneshot::Sender<SignResult>>,
}

/// Generic signer that dispatches to either:
/// - a C-ABI async vtable (for iOS HSM / keychain / external signers), or
/// - a native Rust `Signer` implementation (e.g. `SingleKeySigner`), which
///   avoids the C callback bounce entirely.
pub struct VTableSigner {
    inner: Inner,
}

enum Inner {
    /// C-ABI callback signer. `signer_ptr` and `vtable` are opaque FFI
    /// pointers owned by this `VTableSigner` (or pointing at a static
    /// vtable — see `owns_vtable`).
    Callback {
        signer_ptr: *mut c_void,
        vtable: *const SignerVTable,
        /// When true, the vtable was heap-allocated by
        /// `dash_sdk_signer_create` and must be freed on destroy.
        owns_vtable: bool,
    },
    /// Native Rust signer. No C callbacks involved at all.
    Native(Arc<dyn Signer<IdentityPublicKey>>),
}

// SAFETY: VTableSigner can be sent between threads because:
// 1. The vtable is immutable (static or heap-allocated once, never mutated).
// 2. The actual signer implementations must handle their own thread safety.
// 3. Native variant stores an Arc<dyn Signer + Send + Sync>.
unsafe impl Send for VTableSigner {}
unsafe impl Sync for VTableSigner {}

impl std::fmt::Debug for VTableSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Inner::Callback {
                signer_ptr,
                vtable,
                owns_vtable,
            } => f
                .debug_struct("VTableSigner::Callback")
                .field("signer_ptr", signer_ptr)
                .field("vtable", vtable)
                .field("owns_vtable", owns_vtable)
                .finish(),
            Inner::Native(_) => f
                .debug_struct("VTableSigner::Native")
                .finish_non_exhaustive(),
        }
    }
}

impl VTableSigner {
    /// Create a new callback-based signer. The caller retains responsibility
    /// for the underlying vtable (is it static? heap? does it need freeing?).
    ///
    /// When `owns_vtable` is true, dropping / destroying this signer will
    /// `Box::from_raw` the vtable pointer. When false, the vtable is assumed
    /// to be `'static` or otherwise owned externally.
    ///
    /// # Safety
    /// - `vtable` must point at a valid, properly-initialized `SignerVTable`
    ///   for the entire lifetime of this signer.
    /// - If `owns_vtable == true`, `vtable` must have been produced by
    ///   `Box::into_raw(Box::new(...))`.
    /// - `signer_ptr` must remain valid for the entire lifetime of this
    ///   signer, and must be compatible with the vtable's `destroy`.
    pub unsafe fn from_callback(
        signer_ptr: *mut c_void,
        vtable: *const SignerVTable,
        owns_vtable: bool,
    ) -> Self {
        Self {
            inner: Inner::Callback {
                signer_ptr,
                vtable,
                owns_vtable,
            },
        }
    }

    /// Create a new signer that wraps a native Rust `Signer` implementation.
    /// No C callbacks are involved — the trait impl delegates directly.
    pub fn from_native(signer: Arc<dyn Signer<IdentityPublicKey>>) -> Self {
        Self {
            inner: Inner::Native(signer),
        }
    }
}

impl Drop for VTableSigner {
    fn drop(&mut self) {
        if let Inner::Callback {
            signer_ptr,
            vtable,
            owns_vtable,
        } = &self.inner
        {
            // SAFETY: vtable is still valid here (we haven't freed it yet).
            // The destructor is responsible for cleaning up `signer_ptr`.
            let signer_ptr = *signer_ptr;
            let vtable = *vtable;
            let owns_vtable = *owns_vtable;
            unsafe {
                if !vtable.is_null() {
                    ((*vtable).destroy)(signer_ptr);
                    if owns_vtable {
                        let _ = Box::from_raw(vtable as *mut SignerVTable);
                    }
                }
            }
        }
        // Inner::Native drops its Arc automatically.
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for VTableSigner {
    async fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        match &self.inner {
            Inner::Native(signer) => signer.sign(identity_public_key, data).await,
            Inner::Callback {
                signer_ptr, vtable, ..
            } => {
                // Serialize the public key for the C side.
                let key_bytes =
                    bincode::encode_to_vec(identity_public_key, bincode::config::standard())
                        .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

                // oneshot + leaked `CompletionSlot` — the slot's `AtomicPtr`
                // makes duplicate C completions a no-op (see `CompletionSlot`).
                let (tx, rx) = oneshot::channel::<SignResult>();
                let tx_ptr = Box::into_raw(Box::new(tx));
                let slot = Box::new(CompletionSlot {
                    sender: AtomicPtr::new(tx_ptr),
                });
                let completion_ctx = Box::into_raw(slot) as *mut c_void;

                // SAFETY: vtable is non-null for the Callback variant by
                // construction, and sign_async is required to eventually call
                // `completion` with `completion_ctx`.
                unsafe {
                    ((*(*vtable)).sign_async)(
                        *signer_ptr as *const c_void,
                        key_bytes.as_ptr(),
                        key_bytes.len(),
                        data.as_ptr(),
                        data.len(),
                        completion_ctx,
                        dash_sdk_sign_async_completion,
                    );
                }

                // Await the completion. The oneshot receiver is a real async
                // point — the Tokio worker is free to run other tasks.
                //
                // A non-conforming FFI signer that never invokes `completion`
                // would otherwise hang forever: the `Sender` lives inside a
                // `Box` we handed to C, so it never drops and `rx` never
                // receives `RecvError`. Bound the wait with a timeout so the
                // caller gets a recoverable error instead of a deadlock.
                match tokio::time::timeout(SIGN_ASYNC_COMPLETION_TIMEOUT, rx).await {
                    Ok(Ok(Ok(sig))) => Ok(BinaryData::from(sig)),
                    Ok(Ok(Err(e))) => Err(e),
                    Ok(Err(_recv_err)) => {
                        // Sender dropped without sending — only reachable via
                        // exotic contract violations; surface as recoverable.
                        Err(ProtocolError::Generic(
                            "Signer completion channel dropped without a result; \
                             the FFI signer did not call its completion callback"
                                .to_string(),
                        ))
                    }
                    Err(_elapsed) => {
                        // Timeout: slot stays alive so a late duplicate FFI
                        // completion remains a defined no-op.
                        Err(ProtocolError::Generic(format!(
                            "Signer completion callback not invoked within {:?}; \
                             the FFI signer is unresponsive",
                            SIGN_ASYNC_COMPLETION_TIMEOUT
                        )))
                    }
                }
            }
        }
    }

    async fn sign_create_witness(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<dash_sdk::dpp::address_funds::AddressWitness, ProtocolError> {
        // For callback-based iOS signers we always produce a P2PKH witness
        // from the raw signature — matching the pre-async behavior. Native
        // signers get to use their own `sign_create_witness` impl.
        match &self.inner {
            Inner::Native(signer) => signer.sign_create_witness(identity_public_key, data).await,
            Inner::Callback { .. } => {
                let signature = self.sign(identity_public_key, data).await?;
                Ok(dash_sdk::dpp::address_funds::AddressWitness::P2pkh { signature })
            }
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        match &self.inner {
            Inner::Native(signer) => signer.can_sign_with(identity_public_key),
            Inner::Callback {
                signer_ptr, vtable, ..
            } => {
                // SAFETY: vtable is non-null for the Callback variant.
                unsafe {
                    match bincode::encode_to_vec(identity_public_key, bincode::config::standard()) {
                        Ok(key_bytes) => ((*(*vtable)).can_sign_with)(
                            *signer_ptr as *const c_void,
                            key_bytes.as_ptr(),
                            key_bytes.len(),
                        ),
                        Err(_) => false,
                    }
                }
            }
        }
    }
}

/// Non-owning reference wrapper around a `VTableSigner` that itself
/// implements `Signer<IdentityPublicKey>`. Several rs-sdk APIs
/// (`transfer_credits`, `withdraw`, `register_dpns_name`, ...) take the
/// signer by value (`S: Signer<IdentityPublicKey>`). Since `VTableSigner`
/// is neither `Copy` nor `Clone` — the `Inner::Callback` variant owns a
/// vtable pointer whose destructor must not run twice — the FFI layer
/// passes this lightweight reference wrapper instead. It forwards every
/// `Signer` method to the underlying `VTableSigner`.
#[derive(Debug)]
pub struct VTableSignerRef<'a>(pub &'a VTableSigner);

#[async_trait]
impl<'a> Signer<IdentityPublicKey> for VTableSignerRef<'a> {
    async fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.0.sign(identity_public_key, data).await
    }

    async fn sign_create_witness(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<dash_sdk::dpp::address_funds::AddressWitness, ProtocolError> {
        self.0.sign_create_witness(identity_public_key, data).await
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        self.0.can_sign_with(identity_public_key)
    }
}

/// Rust-side completion callback. Exported so the iOS side can call it via
/// the `SignCompletionCallback` function pointer handed to `sign_async`.
///
/// Single-shot is enforced at runtime by `CompletionSlot`'s `AtomicPtr`:
/// first call claims the sender, duplicates are a safe no-op.
///
/// # Safety
/// - `completion_ctx` must be the exact pointer passed to `SignAsyncCallback`.
///   Reusing another pointer is UB; reusing the same valid pointer is defined
///   (duplicates become no-ops).
/// - `error_message`, if non-null, must be a valid CStr-safe string; copied
///   before return.
/// - `signature`, if non-null, must point to `signature_len` readable bytes;
///   copied before return.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_sign_async_completion(
    completion_ctx: *mut c_void,
    signature: *const u8,
    signature_len: usize,
    error_message: *const c_char,
) {
    if completion_ctx.is_null() {
        // Nothing we can do — the Rust side will time out via channel drop
        // if it ever receives no completion at all.
        return;
    }

    // SAFETY: slot is leaked per `sign_async` call — always valid.
    let slot = &*(completion_ctx as *const CompletionSlot);

    // Single-shot: only the first caller wins the sender; duplicates exit.
    let tx_ptr = slot.sender.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if tx_ptr.is_null() {
        return;
    }
    let tx: Box<oneshot::Sender<SignResult>> = Box::from_raw(tx_ptr);

    let result: SignResult = if !error_message.is_null() {
        let msg = CStr::from_ptr(error_message).to_string_lossy().into_owned();
        Err(ProtocolError::Generic(msg))
    } else if signature.is_null() {
        Err(ProtocolError::Generic(
            "Signer completion returned null signature with no error message".to_string(),
        ))
    } else {
        // Copy out the signature bytes — the iOS side owns its buffer.
        let bytes = std::slice::from_raw_parts(signature, signature_len).to_vec();
        Ok(bytes)
    };

    // `oneshot::Sender::send` is non-blocking and thread-safe; it returns
    // Err if the receiver was already dropped, which we ignore because the
    // awaiting task is gone anyway.
    let _ = tx.send(result);
}

/// Create a new signer with async callbacks from iOS / external code.
///
/// # Breaking change from the pre-async design
/// The `SignCallback` + `free_result_callback` pair has been replaced with a
/// single `SignAsyncCallback`. See the top-of-file docs for migration notes.
///
/// # Parameters
/// - `sign_async_callback`: async sign function (must call its completion).
/// - `can_sign_callback`: synchronous key-availability check.
/// - `destroy_callback`: optional destructor for the `signer` state. Pass NULL
///   if there is nothing to clean up.
///
/// Note: there is intentionally no `signer_ptr` parameter here because the
/// pre-async iOS path never used it — the signing state was captured by the
/// C function pointers themselves. If you need a signer context pointer,
/// construct a `VTableSigner` from Rust using `VTableSigner::from_callback`.
///
/// # Safety
/// - Callback function pointers must be valid and follow the required ABI
///   for the duration of use.
/// - The returned `SignerHandle` must be destroyed with
///   `dash_sdk_signer_destroy` to avoid leaks.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_create(
    sign_async_callback: SignAsyncCallback,
    can_sign_callback: CanSignCallback,
    destroy_callback: DestroyCallback,
) -> *mut SignerHandle {
    // Create a vtable on the heap so it persists for the life of the signer.
    let vtable = Box::new(SignerVTable {
        sign_async: sign_async_callback,
        can_sign_with: can_sign_callback,
        destroy: destroy_callback.unwrap_or(default_destroy),
    });

    let vtable_ptr = Box::into_raw(vtable);

    // SAFETY: vtable_ptr was just produced by Box::into_raw, so it is valid
    // and we own it (owns_vtable = true).
    let vtable_signer = VTableSigner::from_callback(std::ptr::null_mut(), vtable_ptr, true);

    Box::into_raw(Box::new(vtable_signer)) as *mut SignerHandle
}

/// Default destroy function that does nothing — used when the C caller
/// passes a NULL destroy callback.
unsafe extern "C" fn default_destroy(_signer: *mut c_void) {
    // No-op.
}

/// Destroy a signer.
///
/// # Safety
/// - `handle` must be a valid pointer previously returned by this SDK and
///   not yet destroyed.
/// - It may be null (no-op). After this call the handle must not be used
///   again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_destroy(handle: *mut SignerHandle) {
    if handle.is_null() {
        return;
    }
    // Drop the box — Drop impl calls the vtable destructor and frees the
    // vtable if we own it (or drops the Arc for the Native variant).
    let _ = Box::from_raw(handle as *mut VTableSigner);
}

/// Free bytes that were allocated with `malloc`/`calloc`.
///
/// Kept for backwards compatibility with older FFI code that exchanges
/// opaque byte buffers over the ABI. The signer path no longer uses this —
/// signature bytes are copied via the completion callback, not malloc'd.
///
/// # Safety
/// - `bytes` must be a pointer allocated with `malloc` or `calloc`, or null.
/// - It may be null (no-op). After this call the pointer must not be used
///   again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_bytes_free(bytes: *mut u8) {
    if !bytes.is_null() {
        libc::free(bytes as *mut libc::c_void);
    }
}

/// Build a native-Rust signer handle backed by a `SingleKeySigner`. This
/// replaces the old `SINGLE_KEY_SIGNER_VTABLE` static — there is no C
/// callback bounce on this path; the async trait impl calls
/// `SingleKeySigner::sign` directly.
///
/// The returned handle must be destroyed with `dash_sdk_signer_destroy`.
pub fn signer_handle_from_single_key(signer: SingleKeySigner) -> *mut SignerHandle {
    let vtable_signer = VTableSigner::from_native(Arc::new(signer));
    Box::into_raw(Box::new(vtable_signer)) as *mut SignerHandle
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Global flag set by the completion invocation during tests so we can
    /// verify the async bridge actually wired up end-to-end.
    static COMPLETION_CALLED: AtomicBool = AtomicBool::new(false);

    /// Test sign callback: immediately completes synchronously with a fake
    /// 64-byte signature. This simulates the simplest possible iOS signer.
    unsafe extern "C" fn test_sign_async_sync(
        _signer: *const c_void,
        _key_bytes: *const u8,
        _key_len: usize,
        _data: *const u8,
        _data_len: usize,
        completion_ctx: *mut c_void,
        completion: SignCompletionCallback,
    ) {
        COMPLETION_CALLED.store(true, Ordering::SeqCst);
        let sig = [0xABu8; 64];
        // Null error_message = success.
        completion(completion_ctx, sig.as_ptr(), sig.len(), std::ptr::null());
    }

    /// Test sign callback that reports an error via the completion callback.
    unsafe extern "C" fn test_sign_async_error(
        _signer: *const c_void,
        _key_bytes: *const u8,
        _key_len: usize,
        _data: *const u8,
        _data_len: usize,
        completion_ctx: *mut c_void,
        completion: SignCompletionCallback,
    ) {
        let msg = c"simulated hsm error";
        completion(completion_ctx, std::ptr::null(), 0, msg.as_ptr());
    }

    /// Test sign callback that spawns a *different thread* to invoke the
    /// completion — exercising the thread-safety of `oneshot::Sender`.
    unsafe extern "C" fn test_sign_async_threaded(
        _signer: *const c_void,
        _key_bytes: *const u8,
        _key_len: usize,
        _data: *const u8,
        _data_len: usize,
        completion_ctx: *mut c_void,
        completion: SignCompletionCallback,
    ) {
        // Move the raw pointer over thread boundary. This is exactly what
        // the iOS side will do for a biometric prompt.
        let ctx_usize = completion_ctx as usize;
        std::thread::spawn(move || {
            // Small delay to make sure the await is actually suspending.
            std::thread::sleep(std::time::Duration::from_millis(20));
            let sig = [0x55u8; 64];
            unsafe {
                completion(
                    ctx_usize as *mut c_void,
                    sig.as_ptr(),
                    sig.len(),
                    std::ptr::null(),
                );
            }
        });
    }

    /// Test can-sign callback.
    unsafe extern "C" fn test_can_sign(
        _signer: *const c_void,
        _key_bytes: *const u8,
        _key_len: usize,
    ) -> bool {
        true
    }

    /// Test destroy callback (no-op).
    unsafe extern "C" fn test_destroy(_signer: *mut c_void) {}

    fn make_dummy_key() -> IdentityPublicKey {
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
        use dash_sdk::dpp::platform_value::BinaryData;

        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
            contract_bounds: None,
        })
    }

    fn make_signer(sign_async: SignAsyncCallback) -> VTableSigner {
        let vtable = Box::new(SignerVTable {
            sign_async,
            can_sign_with: test_can_sign,
            destroy: test_destroy,
        });
        let vtable_ptr = Box::into_raw(vtable);
        // SAFETY: we just produced the vtable with Box::into_raw and we take
        // ownership of it here (owns_vtable = true) so it will be freed on drop.
        unsafe { VTableSigner::from_callback(std::ptr::null_mut(), vtable_ptr, true) }
    }

    #[tokio::test]
    async fn completion_callback_same_thread_success() {
        COMPLETION_CALLED.store(false, Ordering::SeqCst);
        let signer = make_signer(test_sign_async_sync);
        let key = make_dummy_key();

        let sig = signer
            .sign(&key, &[1, 2, 3])
            .await
            .expect("sign should succeed");
        assert_eq!(sig.len(), 64);
        assert!(
            COMPLETION_CALLED.load(Ordering::SeqCst),
            "completion callback must have been invoked"
        );
    }

    #[tokio::test]
    async fn completion_callback_error_path() {
        let signer = make_signer(test_sign_async_error);
        let key = make_dummy_key();

        let err = signer
            .sign(&key, &[4, 5, 6])
            .await
            .expect_err("sign should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("simulated hsm error"),
            "error message should propagate: got {msg}"
        );
    }

    #[tokio::test]
    async fn completion_callback_cross_thread() {
        // Exercises the realistic case: completion runs on a different
        // thread than sign_async was called from. tokio::sync::oneshot
        // handles this without blocking our worker.
        let signer = make_signer(test_sign_async_threaded);
        let key = make_dummy_key();

        let sig = signer
            .sign(&key, &[7, 8, 9])
            .await
            .expect("sign should succeed from another thread");
        assert_eq!(sig.len(), 64);
        assert_eq!(sig.as_slice()[0], 0x55);
    }

    /// Calls completion three times to exercise the `CompletionSlot`
    /// single-shot guard; second and third calls must be no-ops.
    unsafe extern "C" fn test_sign_async_double_complete(
        _signer: *const c_void,
        _key_bytes: *const u8,
        _key_len: usize,
        _data: *const u8,
        _data_len: usize,
        completion_ctx: *mut c_void,
        completion: SignCompletionCallback,
    ) {
        let sig = [0x42u8; 64];
        completion(completion_ctx, sig.as_ptr(), sig.len(), std::ptr::null());
        // Duplicate error payload — must not overwrite the first result.
        let err_msg = c"duplicate completion — should be ignored";
        completion(completion_ctx, std::ptr::null(), 0, err_msg.as_ptr());
        // Duplicate success payload — still a no-op.
        let sig2 = [0x99u8; 64];
        completion(completion_ctx, sig2.as_ptr(), sig2.len(), std::ptr::null());
    }

    #[tokio::test]
    async fn completion_callback_duplicate_is_no_op() {
        let signer = make_signer(test_sign_async_double_complete);
        let key = make_dummy_key();

        let sig = signer
            .sign(&key, &[10, 11, 12])
            .await
            .expect("first completion wins — sign should succeed");
        assert_eq!(sig.len(), 64);
        assert_eq!(
            sig.as_slice()[0],
            0x42,
            "first completion's signature must win; duplicate overwrite must be ignored"
        );
    }
}
