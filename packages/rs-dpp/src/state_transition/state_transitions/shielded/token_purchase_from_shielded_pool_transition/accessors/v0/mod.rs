use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::fee::Credits;
use crate::shielded::SerializedAction;
use platform_value::Identifier;

/// Accessors for the fields of a `TokenPurchaseFromShieldedPoolTransition`.
pub trait TokenPurchaseFromShieldedPoolTransitionAccessorsV0 {
    /// The contract defining the token.
    fn data_contract_id(&self) -> Identifier;
    /// The token's position in the contract.
    fn token_contract_position(&self) -> TokenContractPosition;
    /// The token id, `calculate_token_id(data_contract_id, token_contract_position)`.
    fn token_id(&self) -> Identifier;
    /// Tokens bought and minted into the pool; the token bundle's value balance is its negation.
    fn token_count(&self) -> TokenAmount;
    /// The credits the buyer agrees to pay for `token_count`, paid out of the credit bundle to the contract owner; `credit_amount` minus this is the fee.
    fn total_agreed_price(&self) -> Credits;
    /// Orchard actions of the bundle in the token's shielded pool.
    fn token_actions(&self) -> &[SerializedAction];
    /// Sinsemilla root of the token pool's note commitment tree the token bundle was built against.
    fn token_anchor(&self) -> [u8; 32];
    /// Halo 2 proof of the token bundle.
    fn token_proof(&self) -> &[u8];
    /// RedPallas binding signature of the token bundle.
    fn token_binding_signature(&self) -> [u8; 64];
    /// Orchard actions of the spend bundle in the credit shielded pool that pays the fee.
    fn fee_actions(&self) -> &[SerializedAction];
    /// Sinsemilla root of the credit pool's note commitment tree the fee bundle was built against.
    fn fee_anchor(&self) -> [u8; 32];
    /// Halo 2 proof of the fee bundle.
    fn fee_proof(&self) -> &[u8];
    /// RedPallas binding signature of the fee bundle.
    fn fee_binding_signature(&self) -> [u8; 64];
    /// Credits leaving the credit pool: the fee bundle's value balance.
    fn credit_amount(&self) -> Credits;

    /// The nullifiers the token bundle spends.
    fn token_nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.token_actions()
            .iter()
            .map(|action| T::from(action.nullifier))
            .collect()
    }

    /// The nullifiers the fee bundle spends.
    fn fee_nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.fee_actions()
            .iter()
            .map(|action| T::from(action.nullifier))
            .collect()
    }
}
