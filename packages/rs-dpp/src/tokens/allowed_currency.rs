use platform_value::Identifier;

#[derive(Debug, PartialEq, Clone)]
pub enum AllowedCurrency {
    TradingInDash,
    OnContract(Identifier, String),
}
