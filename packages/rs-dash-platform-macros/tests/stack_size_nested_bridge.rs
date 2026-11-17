//! Regression test for <https://github.com/dashpay/platform/issues/3535>.
//!
//! An async `#[stack_size]` body must be able to perform a nested
//! sync-over-async bridge. `tokio::task::block_in_place` panics with
//! "can call blocking only when running on the multi-threaded runtime" on a
//! current-thread runtime, so it fails fast (never hangs) if the macro drives
//! the body on the wrong runtime flavor.

use dash_platform_macros::stack_size;

#[stack_size(8 * 1024 * 1024)]
#[test]
async fn stack_size_async_allows_nested_block_in_place() {
    let sum = tokio::task::block_in_place(|| 1 + 1);
    assert_eq!(sum, 2);
}
