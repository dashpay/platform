pub trait MockRequest {}
impl<T> MockRequest for T {}

pub trait MockResponse {}
impl<T> MockResponse for T {}
