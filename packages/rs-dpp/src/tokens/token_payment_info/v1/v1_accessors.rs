use crate::tokens::token_payment_info::v1::TokenShieldedPayment;

/// Accessors for the shielded payment a `TokenPaymentInfo` may carry (format version 1 and up).
pub trait TokenPaymentInfoAccessorsV1 {
    /// The spend bundle paying the token cost out of the token's shielded pool, when the
    /// payment is shielded (`V1`); `None` for a `V0` that pays from the identity's balance.
    fn shielded_payment(&self) -> Option<&TokenShieldedPayment>;

    /// Sets or clears the shielded payment. Setting one on a `V0` upgrades it to `V1`;
    /// clearing a `V1`'s downgrades it to `V0`, so a payment info without a pool payment
    /// always has the `V0` shape.
    fn set_shielded_payment(&mut self, shielded_payment: Option<TokenShieldedPayment>);
}
