use crate::util::object_size_info::DataContractInfo;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;

/// Operations on Documents
#[derive(Clone, Debug)]
pub enum TokenOperationType<'a> {
    /// Adds a document to a contract matching the desired info.
    TokenBurn {
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Token position in the contract, is 0 if there is only one token
        token_position: u16,
        /// The token id
        token_id: Identifier,
        /// The amount to burn
        burn_amount: TokenAmount,
    },
    /// Adds a document to a contract matching the desired info.
    TokenIssuance {
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Token position in the contract, is 0 if there is only one token
        token_position: u16,
        /// The token id
        token_id: Identifier,
        /// The amount to issue
        issuance_amount: TokenAmount,
    },
    /// Adds a document to a contract matching the desired info.
    TokenTransfer {
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Token position in the contract, is 0 if there is only one token
        token_position: u16,
        /// The token id
        token_id: Identifier,
        /// The recipient of the transfer
        recipient_id: Identifier,
        /// The amount to transfer
        amount: TokenAmount,
    },
}
