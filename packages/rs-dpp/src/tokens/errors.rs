use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("There is no token at this position")]
    TokenNotFoundAtPositionError,
    #[error("The contract version does not allow tokens")]
    TokenNotFoundOnContractVersion,
    #[error("There is no minting recipient set")]
    TokenNoMintingRecipient,
}
