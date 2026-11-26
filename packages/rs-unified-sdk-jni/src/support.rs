//! Shared JNI plumbing: JVM caching, panic guards, exception throwing.

use jni::errors::{Error as JniError, JniError as JniCallError};
use jni::objects::{JString, JThrowable, JValue};
use jni::sys::jint;
use jni::{JNIEnv, JavaVM};
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIConsensusErrorKind, PlatformWalletFFIResult,
    PlatformWalletFFIResultCode,
};
use rs_sdk_ffi::{DashSDKConsensusErrorKind, FFINetwork};
use std::ffi::{CStr, CString};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;
use zeroize::{Zeroize, Zeroizing};

/// The platform-wallet kind that shares each rs-sdk-ffi consensus kind's
/// meaning. The match is exhaustive, so a family added to one FFI alone
/// fails the build here.
const fn wallet_consensus_kind(
    kind: DashSDKConsensusErrorKind,
) -> PlatformWalletFFIConsensusErrorKind {
    match kind {
        DashSDKConsensusErrorKind::ConsensusErrorKindNone => {
            PlatformWalletFFIConsensusErrorKind::None
        }
        DashSDKConsensusErrorKind::ConsensusErrorKindBasic => {
            PlatformWalletFFIConsensusErrorKind::Basic
        }
        DashSDKConsensusErrorKind::ConsensusErrorKindSignature => {
            PlatformWalletFFIConsensusErrorKind::Signature
        }
        DashSDKConsensusErrorKind::ConsensusErrorKindFee => {
            PlatformWalletFFIConsensusErrorKind::Fee
        }
        DashSDKConsensusErrorKind::ConsensusErrorKindState => {
            PlatformWalletFFIConsensusErrorKind::State
        }
    }
}

const fn crosses_as_wallet_kind(kind: DashSDKConsensusErrorKind) -> bool {
    kind as i32 == wallet_consensus_kind(kind) as i32
}

/// Compile-time drift guard for the consensus kind discriminant.
///
/// Both FFIs hand a consensus rejection's kind to
/// [`throw_sdk_consensus_exception`] as its bare discriminant, and Kotlin's
/// `ConsensusErrorKind.fromNative` decodes it with one table. So every
/// `DashSDKConsensusErrorKind` must cross as the same number as the
/// `PlatformWalletFFIConsensusErrorKind` of the same family; a drift is a
/// build failure rather than a rejection Kotlin files under the wrong family.
const _: () = assert!(
    crosses_as_wallet_kind(DashSDKConsensusErrorKind::ConsensusErrorKindNone)
        && crosses_as_wallet_kind(DashSDKConsensusErrorKind::ConsensusErrorKindBasic)
        && crosses_as_wallet_kind(DashSDKConsensusErrorKind::ConsensusErrorKindSignature)
        && crosses_as_wallet_kind(DashSDKConsensusErrorKind::ConsensusErrorKindFee)
        && crosses_as_wallet_kind(DashSDKConsensusErrorKind::ConsensusErrorKindState),
    "rs-sdk-ffi's DashSDKConsensusErrorKind drifted from platform-wallet-ffi's \
     PlatformWalletFFIConsensusErrorKind; Kotlin decodes both with one table"
);

/// Offset added to every `PlatformWalletFFIResultCode` before it is thrown
/// as a `DashSDKException` code — see [`take_pwffi_error`].
///
/// platform-wallet-ffi's result codes (`PlatformWalletFFIResultCode`, 0–20 +
/// 98/99) occupy the SAME small-integer range as rs-sdk-ffi's
/// `DashSDKErrorCode` (1–10), which Kotlin's `DashSdkError.fromNative`
/// interprets. Throwing a raw platform-wallet code would collide: e.g.
/// `ErrorWalletOperation` (6) would surface to Kotlin as `CryptoError` (6),
/// and the retry-semantics-bearing codes (`ErrorShieldedNoRecordedAnchor`
/// = 19 retryable, `ErrorTransactionBroadcastUnconfirmed` = 20 do-NOT-retry)
/// would flatten into the `else -> InternalError` bucket, losing their
/// contract. Shifting into a dedicated `>= 1000` namespace lets
/// `DashSdkError.fromNative` route these to a distinct `PlatformWallet`
/// subtree (by subtracting the offset) while the native rs-sdk-ffi codes
/// stay in 1–10. Must stay in lockstep with
/// `DashSdkError.PLATFORM_WALLET_CODE_OFFSET` on the Kotlin side.
pub const PWFFI_CODE_OFFSET: i32 = 1000;

/// FFINetwork ordinal → enum (0=Mainnet, 2=Devnet, 3=Regtest, else
/// Testnet). Must stay in lockstep with Kotlin's `Network.ffiValue`.
///
/// This is the single shared mapping for every JNI module:
/// `platform_wallet_ffi::FFINetwork` is a re-export of `rs_sdk_ffi::FFINetwork`,
/// so the one helper serves callers regardless of which FFI crate they talk to.
pub fn net_from_ord(ord: i32) -> FFINetwork {
    match ord {
        0 => FFINetwork::Mainnet,
        2 => FFINetwork::Devnet,
        3 => FFINetwork::Regtest,
        _ => FFINetwork::Testnet,
    }
}

/// If `result` carries a non-`Success` code: throw `DashSDKException`,
/// free its message, and return `true` (the caller bails with its
/// default). On `Success` frees nothing (message is null) and returns
/// `false`.
///
/// The thrown exception code is the `PlatformWalletFFIResultCode` value
/// shifted by [`PWFFI_CODE_OFFSET`] so it never collides with the native
/// rs-sdk-ffi `DashSDKErrorCode` range Kotlin's `DashSdkError.fromNative`
/// also decodes. This is the single shared mapping used by every JNI module
/// that calls a platform-wallet-ffi entry point (mirrors `results::take_error`
/// for the rs-sdk-ffi side).
pub fn take_pwffi_error(env: &mut JNIEnv, mut result: PlatformWalletFFIResult) -> bool {
    if result.code == PlatformWalletFFIResultCode::Success {
        return false;
    }
    throw_pwffi_result(env, &result);
    // SAFETY: `result` is a fresh PlatformWalletFFIResult; free its message.
    unsafe { platform_wallet_ffi_result_free(&mut result) };
    true
}

/// Throw the `DashSDKException` for a non-`Success` `result`, leaving its
/// message for the caller to free. The exception code is the result code
/// shifted by [`PWFFI_CODE_OFFSET`]; a result that carries a consensus
/// rejection (`consensus_code != 0`) hands its code and kind to the exception
/// as well, so Kotlin can branch on them instead of on the message.
pub fn throw_pwffi_result(env: &mut JNIEnv, result: &PlatformWalletFFIResult) {
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: non-null message is a valid CString produced by the FFI.
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    let code = result.code as i32 + PWFFI_CODE_OFFSET;
    if result.consensus_code == 0 {
        throw_sdk_exception(env, code, &message);
    } else {
        throw_sdk_consensus_exception(
            env,
            code,
            &message,
            result.consensus_code,
            result.consensus_kind as jint,
        );
    }
}

/// The process-wide JVM, cached in [`crate::JNI_OnLoad`]. Callback
/// trampolines use this to attach Tokio worker threads.
pub static JVM: OnceLock<JavaVM> = OnceLock::new();

/// JNI-internal name of the Kotlin exception type thrown for SDK errors.
pub const SDK_EXCEPTION_CLASS: &str = "org/dashfoundation/dashsdk/ffi/DashSDKException";

/// Throw `DashSDKException(code, message)`; falls back to a plain
/// `RuntimeException` if the class or constructor lookup fails (e.g. the
/// library is loaded outside the Kotlin SDK).
pub fn throw_sdk_exception(env: &mut JNIEnv, code: i32, message: &str) {
    throw_sdk_exception_with(env, code, message, "(ILjava/lang/String;)V", &[]);
}

/// Throw `DashSDKException(code, message, consensusCode, consensusKind)` for
/// a failure that is a consensus rejection. `consensus_kind` is the
/// discriminant of either FFI's kind (`PlatformWalletFFIConsensusErrorKind`
/// or rs-sdk-ffi's `DashSDKConsensusErrorKind`, held equal by the drift guard
/// at the top of this module), which Kotlin's `ConsensusErrorKind.fromNative`
/// decodes. Same `RuntimeException` fallback as [`throw_sdk_exception`].
pub fn throw_sdk_consensus_exception(
    env: &mut JNIEnv,
    code: i32,
    message: &str,
    consensus_code: u32,
    consensus_kind: jint,
) {
    // Consensus codes top out in the 40000s, far inside a `jint`; one that
    // somehow is not goes out as `jint::MAX` rather than wrapping negative.
    let consensus_code = jint::try_from(consensus_code).unwrap_or(jint::MAX);
    throw_sdk_exception_with(
        env,
        code,
        message,
        "(ILjava/lang/String;II)V",
        &[consensus_code, consensus_kind],
    );
}

/// Construct `DashSDKException` through the constructor `signature` names,
/// passing `code`, `message` and then `extra`, and throw it.
fn throw_sdk_exception_with(
    env: &mut JNIEnv,
    code: i32,
    message: &str,
    signature: &str,
    extra: &[jint],
) {
    // If an exception is already pending we must not call further JNI
    // functions that would themselves throw.
    if env.exception_check().unwrap_or(false) {
        return;
    }
    let thrown = (|| -> jni::errors::Result<()> {
        let jmsg = env.new_string(message)?;
        let mut args: Vec<JValue> = vec![code.into(), (&jmsg).into()];
        args.extend(extra.iter().map(|value| JValue::from(*value)));
        let obj = env.new_object(SDK_EXCEPTION_CLASS, signature, &args)?;
        env.throw(JThrowable::from(obj))
    })();
    if thrown.is_err() {
        let _ = env.exception_clear();
        let _ = env.throw_new("java/lang/RuntimeException", message);
    }
}

/// Why a Java `String` argument could not be read by [`read_jstring`].
#[derive(Debug)]
pub(crate) enum JStringError {
    /// The string could not be read: a null reference, a Java exception
    /// raised by the read (left pending for the caller to clear or
    /// propagate), or a buffer that could not be allocated.
    Jni(JniError),
    /// The string holds an unpaired UTF-16 surrogate, so it has no UTF-8
    /// form. Nothing is pending.
    MalformedUtf16,
}

impl fmt::Display for JStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JStringError::Jni(e) => write!(f, "could not be read: {e}"),
            JStringError::MalformedUtf16 => f.write_str("is not well-formed UTF-16"),
        }
    }
}

impl From<JStringError> for JniError {
    fn from(e: JStringError) -> Self {
        match e {
            JStringError::Jni(e) => e,
            JStringError::MalformedUtf16 => JniError::WrongJValueType(
                "well-formed UTF-16 String",
                "String with an unpaired surrogate",
            ),
        }
    }
}

/// Why a Java `String` argument could not be read as a C string by
/// [`read_jcstring`] or [`read_jcstring_zeroizing`].
#[derive(Debug)]
pub(crate) enum JCStringError {
    /// The string could not be read or is not well-formed UTF-16; see
    /// [`JStringError`].
    Read(JStringError),
    /// The string holds U+0000, which a C string cannot carry. Nothing is
    /// pending.
    InteriorNul,
}

impl From<JStringError> for JCStringError {
    fn from(e: JStringError) -> Self {
        JCStringError::Read(e)
    }
}

impl fmt::Display for JCStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JCStringError::Read(e) => e.fmt(f),
            JCStringError::InteriorNul => f.write_str("contains a NUL character"),
        }
    }
}

impl From<JCStringError> for JniError {
    fn from(e: JCStringError) -> Self {
        match e {
            JCStringError::Read(e) => e.into(),
            JCStringError::InteriorNul => {
                JniError::WrongJValueType("String without interior NUL", "String with NUL")
            }
        }
    }
}

/// Read a Java `String` as UTF-8, rejecting an unpaired surrogate.
///
/// `JNIEnv::get_string(..).into()` decodes Java's modified UTF-8 and, when
/// that fails, silently falls back to a lossy conversion, so a string with an
/// unpaired surrogate reaches Rust as different text (U+FFFD in its place).
/// Every string argument goes through this helper instead: it copies the
/// UTF-16 code units with `GetStringRegion` and decodes them strictly, so
/// malformed input is refused rather than rewritten, whichever way the VM
/// encodes modified UTF-8.
///
/// After a failed read it makes no further JNI call, so a pending exception
/// is never followed by one (CheckJNI aborts on that). The code units are
/// validated before any text is decoded, the output is allocated once at its
/// exact size, and the UTF-16 copy is zeroed before it is dropped, since one
/// argument is a mnemonic.
pub(crate) fn read_jstring(env: &mut JNIEnv, s: &JString) -> Result<String, JStringError> {
    if s.is_null() {
        return Err(JStringError::Jni(JniError::NullPtr("read_jstring string")));
    }
    let mut units = read_utf16_units(env, s)?;
    let decoded = decode_utf16_strict(&units);
    units.zeroize();
    decoded
}

fn read_utf16_units(env: &mut JNIEnv, s: &JString) -> Result<Vec<u16>, JStringError> {
    let raw = env.get_raw();
    // SAFETY: `raw` is this thread's live `JNIEnv` and its function table is
    // valid for the thread's lifetime.
    let functions = unsafe { ((**raw).GetStringLength, (**raw).GetStringRegion) };
    let (Some(string_length), Some(string_region)) = functions else {
        return Err(JStringError::Jni(JniError::JNIEnvMethodNotFound(
            "GetStringRegion",
        )));
    };
    // SAFETY: `s` is a live, non-null string reference (checked by the
    // caller); `GetStringLength` neither throws nor retains it.
    let length = unsafe { string_length(raw, s.as_raw()) };
    let capacity = usize::try_from(length)
        .map_err(|_| JStringError::Jni(JniError::JniCall(JniCallError::Other(length))))?;
    let mut units: Vec<u16> = Vec::new();
    if units.try_reserve_exact(capacity).is_err() {
        return Err(JStringError::Jni(JniError::JniCall(JniCallError::NoMemory)));
    }
    units.resize(capacity, 0);
    // SAFETY: `units` holds exactly `length` code units, which is what
    // `GetStringRegion` writes for the range `0..length` of `s`.
    unsafe { string_region(raw, s.as_raw(), 0, length, units.as_mut_ptr()) };
    if !matches!(env.exception_check(), Ok(false)) {
        units.zeroize();
        return Err(JStringError::Jni(JniError::JavaException));
    }
    Ok(units)
}

/// The UTF-8 length of `units` and whether they hold U+0000, or
/// [`JStringError::MalformedUtf16`] for an unpaired surrogate. Decodes
/// nothing, so it runs before any output is allocated.
fn utf8_len_strict(units: &[u16]) -> Result<(usize, bool), JStringError> {
    let mut utf8_len = 0usize;
    let mut has_nul = false;
    for c in char::decode_utf16(units.iter().copied()) {
        match c {
            Ok(c) => {
                utf8_len += c.len_utf8();
                has_nul |= c == '\0';
            }
            Err(_) => return Err(JStringError::MalformedUtf16),
        }
    }
    Ok((utf8_len, has_nul))
}

/// Strict UTF-16 to UTF-8: an unpaired surrogate is an error, never U+FFFD.
///
/// Validates every code unit before allocating, then reserves the output
/// once at its exact size, so a rejected string leaves no decoded prefix
/// behind and an accepted one never reallocates.
fn decode_utf16_strict(units: &[u16]) -> Result<String, JStringError> {
    let (utf8_len, _) = utf8_len_strict(units)?;
    let mut out = String::new();
    if out.try_reserve_exact(utf8_len).is_err() {
        return Err(JStringError::Jni(JniError::JniCall(JniCallError::NoMemory)));
    }
    // Every item is `Ok`: `utf8_len_strict` returned on the first error.
    out.extend(char::decode_utf16(units.iter().copied()).filter_map(Result::ok));
    Ok(out)
}

/// Strict UTF-16 to a NUL-terminated UTF-8 buffer: an unpaired surrogate or
/// an interior NUL is an error. The buffer is reserved once, fallibly, at its
/// exact size including the terminator, so it never reallocates and a
/// `CString` built from it adopts it as is.
fn encode_c_string_strict(units: &[u16]) -> Result<Vec<u8>, JCStringError> {
    let (utf8_len, has_nul) = utf8_len_strict(units)?;
    if has_nul {
        return Err(JCStringError::InteriorNul);
    }
    let mut out: Vec<u8> = Vec::new();
    if out.try_reserve_exact(utf8_len + 1).is_err() {
        return Err(JStringError::Jni(JniError::JniCall(JniCallError::NoMemory)).into());
    }
    let mut buf = [0u8; 4];
    // Every item is `Ok`: `utf8_len_strict` returned on the first error.
    for c in char::decode_utf16(units.iter().copied()).filter_map(Result::ok) {
        out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }
    buf.zeroize();
    out.push(0);
    Ok(out)
}

/// The non-null Java string `s` as a NUL-terminated UTF-8 buffer
/// ([`encode_c_string_strict`]), with the UTF-16 copy zeroed.
fn read_jcstring_bytes(env: &mut JNIEnv, s: &JString) -> Result<Vec<u8>, JCStringError> {
    if s.is_null() {
        return Err(JStringError::Jni(JniError::NullPtr("read_jcstring string")).into());
    }
    let mut units = read_utf16_units(env, s)?;
    let encoded = encode_c_string_strict(&units);
    units.zeroize();
    encoded
}

/// Read a Java `String` argument as a `CString` for a C call: as
/// [`read_jstring`], and a string holding U+0000 is
/// [`JCStringError::InteriorNul`] rather than truncated.
///
/// The terminator is part of the one fallible allocation, so no step after
/// the read can abort on allocation failure or leave an earlier copy behind.
pub(crate) fn read_jcstring(env: &mut JNIEnv, s: &JString) -> Result<CString, JCStringError> {
    let bytes = read_jcstring_bytes(env, s)?;
    // The bytes end in their only NUL, so this adopts the buffer without
    // copying it; the error arm cannot be reached.
    CString::from_vec_with_nul(bytes).map_err(|_| JCStringError::InteriorNul)
}

/// [`read_jcstring`] for a secret (a mnemonic): the NUL-terminated bytes in a
/// buffer that is zeroed when dropped. Pass `as_ptr()` to the C call and keep
/// the buffer alive until it returns; no other copy of the text is left in
/// Rust memory.
pub(crate) fn read_jcstring_zeroizing(
    env: &mut JNIEnv,
    s: &JString,
) -> Result<Zeroizing<Vec<u8>>, JCStringError> {
    read_jcstring_bytes(env, s).map(Zeroizing::new)
}

/// Run an export body under `catch_unwind` so a Rust panic surfaces as a
/// Java `RuntimeException` instead of unwinding across the JNI boundary
/// (which is undefined behavior).
pub fn guard<T>(env: &mut JNIEnv, default: T, f: impl FnOnce(&mut JNIEnv) -> T) -> T {
    match catch_unwind(AssertUnwindSafe(|| f(env))) {
        Ok(value) => value,
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "Rust panic in dash_sdk_jni".to_string());
            if !env.exception_check().unwrap_or(false) {
                let _ = env.throw_new("java/lang/RuntimeException", &msg);
            }
            default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_utf16_strict, encode_c_string_strict, net_from_ord, JCStringError, JStringError,
    };
    use rs_sdk_ffi::FFINetwork;

    #[test]
    fn net_from_ord_matches_kotlin_ffi_values() {
        assert_eq!(net_from_ord(0), FFINetwork::Mainnet);
        assert_eq!(net_from_ord(1), FFINetwork::Testnet);
        assert_eq!(net_from_ord(2), FFINetwork::Devnet);
        assert_eq!(net_from_ord(3), FFINetwork::Regtest);
        assert_eq!(net_from_ord(-1), FFINetwork::Testnet, "unknown → Testnet");
    }

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn should_decode_well_formed_utf16_including_supplementary_characters() {
        for text in ["", "memo", "caf\u{e9}", "\u{1F600} pay", "a\u{0}b"] {
            let decoded = decode_utf16_strict(&utf16(text)).unwrap();
            assert_eq!(decoded, text);
            // Reserved once at the exact size: no reallocation left an
            // unwiped earlier buffer behind.
            assert_eq!(decoded.capacity(), text.len());
        }
    }

    #[test]
    fn should_reject_an_unpaired_surrogate_instead_of_replacing_it() {
        let lone_high = [u16::from(b'a'), 0xD800, u16::from(b'b')];
        let lone_low = [0xDC00, u16::from(b'a')];
        let trailing_high = [u16::from(b'a'), 0xDBFF];
        let reversed_pair = [0xDE00, 0xD83D];
        for units in [&lone_high[..], &lone_low, &trailing_high, &reversed_pair] {
            assert!(matches!(
                decode_utf16_strict(units),
                Err(JStringError::MalformedUtf16)
            ));
        }
    }

    #[test]
    fn should_encode_a_c_string_with_its_terminator_in_one_exact_buffer() {
        for text in ["", "memo", "caf\u{e9}", "\u{1F600} pay"] {
            let bytes = encode_c_string_strict(&utf16(text)).unwrap();
            assert_eq!(&bytes[..text.len()], text.as_bytes());
            assert_eq!(bytes[text.len()..], [0]);
            // Reserved once with room for the terminator: `CString` adopts
            // the buffer without reallocating.
            assert_eq!(bytes.capacity(), text.len() + 1);
        }
    }

    #[test]
    fn should_refuse_a_c_string_with_an_interior_nul_or_an_unpaired_surrogate() {
        assert!(matches!(
            encode_c_string_strict(&utf16("a\u{0}b")),
            Err(JCStringError::InteriorNul)
        ));
        assert!(matches!(
            encode_c_string_strict(&utf16("\u{1F600}\u{0}")),
            Err(JCStringError::InteriorNul)
        ));
        // A malformed string is reported as malformed even beside a NUL.
        assert!(matches!(
            encode_c_string_strict(&[0, 0xD800]),
            Err(JCStringError::Read(JStringError::MalformedUtf16))
        ));
    }
}
