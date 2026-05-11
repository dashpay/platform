#![allow(clippy::field_reassign_with_default)]

//! TC-076, TC-077, TC-078 — compile-time assertions.

use std::sync::Arc;

use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use static_assertions::assert_impl_all;

assert_impl_all!(SqlitePersister: Send, Sync, PlatformWalletPersistence);

/// TC-078: SqlitePersister fits behind Arc<dyn PlatformWalletPersistence>.
#[test]
fn tc078_object_safety() {
    fn accepts(_: Arc<dyn PlatformWalletPersistence>) {}
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let cfg = SqlitePersisterConfig::new(&path);
    let p = SqlitePersister::open(cfg).unwrap();
    let arc: Arc<dyn PlatformWalletPersistence> = Arc::new(p);
    accepts(arc);
}
