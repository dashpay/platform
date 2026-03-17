use platform_wallet::{CoreWallet, IdentityManager, PlatformWallet};
use static_assertions::assert_impl_all;

assert_impl_all!(PlatformWallet: Send, Sync);
assert_impl_all!(CoreWallet: Send, Sync);
assert_impl_all!(IdentityManager: Send, Sync);
