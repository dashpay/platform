use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("There is no token at this position")]
    TokenNotFoundAtPositionError,
}
