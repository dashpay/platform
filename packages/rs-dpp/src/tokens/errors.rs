use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("There is no destination identity to put the token balance to")]
    DestinationIdentityForMintingNotSetError,
    #[error("There is no token at this position")]
    TokenNotFoundAtPositionError,
}
