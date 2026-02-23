// Re-export Platform Wallet FFI types for cbindgen to pick up

pub use platform_wallet_ffi::{
    BlockTime, Handle, IdentifierArray, IdentifierBytes, PlatformWalletFFIError,
    PlatformWalletFFIResult, NULL_HANDLE,
};
