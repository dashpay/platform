/// Noop mock response trait; used only when `mocks` feature is disabled.
pub trait MockResponse {}
impl<T> MockResponse for T {}
