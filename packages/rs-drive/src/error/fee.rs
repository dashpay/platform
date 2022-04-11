#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    #[error("overflow error: {0}")]
    Overflow(&'static str),
}
