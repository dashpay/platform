mod handlers;

use crate::error::Error;
use crate::platform_types::platform::Platform;
use std::fmt::Debug;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct ReadOnlyAbciApplication<'a, C> {
    /// Platform
    pub platform: &'a Platform<C>,
}

impl<'a, C> ReadOnlyAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Result<ReadOnlyAbciApplication<'a, C>, Error> {
        let app = ReadOnlyAbciApplication { platform };

        Ok(app)
    }
}

impl<'a, C> Debug for ReadOnlyAbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<ReadOnlyAbciApp>")
    }
}
