//! Token contract info types
//!
//! This module contains types for retrieving token contract information from proofs.

use dpp::tokens::contract_info::TokenContractInfo as DppTokenContractInfo;

/// Token contract info
#[derive(Debug, Clone, PartialEq)]
pub struct TokenContractInfo(pub DppTokenContractInfo);

impl From<DppTokenContractInfo> for TokenContractInfo {
    fn from(info: DppTokenContractInfo) -> Self {
        TokenContractInfo(info)
    }
}

impl From<TokenContractInfo> for DppTokenContractInfo {
    fn from(info: TokenContractInfo) -> Self {
        info.0
    }
}