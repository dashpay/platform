use platform_wallet::broadcaster::SpvBroadcaster;
use platform_wallet::{CoreWallet, IdentityManager, PlatformWallet};
use static_assertions::assert_impl_all;

assert_impl_all!(PlatformWallet: Send, Sync);
// `CoreWallet` is now generic over the broadcaster type; check
// the concrete monomorphization we actually use at runtime.
assert_impl_all!(CoreWallet<SpvBroadcaster>: Send, Sync);
assert_impl_all!(IdentityManager: Send, Sync);
