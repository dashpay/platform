use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeeError {
    #[error("Current credits balance {balance} is not enough to pay {fee} fee")]
    BalanceIsNotEnoughError { balance: i64, fee: i64 },
}
