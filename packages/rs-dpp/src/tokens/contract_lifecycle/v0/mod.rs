#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::tokens::contract_lifecycle::ContractWipe;
use bincode::{Decode, Encode};
use derive_more::From;

/// The lifecycle record of a contract that issues tokens (version 0).
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, From, PartialEq)]
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContractTokenLifecycleV0 {
    /// The sum of the supplies of every token the contract issues. Every native supply write
    /// moves it by the same amount in the same batch, so it equals the summed supply leaves and,
    /// because every native supply change moves a balance by the same amount, the summed holder
    /// balances of the contract's tokens.
    ///
    /// The record sits inside an internally tagged enum, so the serde shape goes through the
    /// content buffer that cannot hold a 128-bit integer; the helper writes a number while the
    /// value fits a `u64` and a string above that.
    #[cfg_attr(
        any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
        serde(with = "crate::serialization::json_safe_u128_content")
    )]
    pub issued_supply: u128,
    /// Set once the issuer is destroyed. A wiped record never goes back to live.
    pub wiped: Option<ContractWipe>,
}

/// The moment an issuer was destroyed (version 0).
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, From, PartialEq)]
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContractWipeV0 {
    /// The height of the block that destroyed the issuer.
    pub block_height: u64,
    /// The time of the block that destroyed the issuer, in milliseconds.
    pub block_time_ms: u64,
}

/// Accessors for [`ContractTokenLifecycleV0`].
pub trait ContractTokenLifecycleV0Accessors {
    /// The supply rollup.
    fn issued_supply(&self) -> u128;

    /// Sets the supply rollup.
    fn set_issued_supply(&mut self, issued_supply: u128);

    /// The wipe marker, if the issuer was destroyed.
    fn wiped(&self) -> Option<&ContractWipe>;

    /// Marks the issuer destroyed.
    fn set_wiped(&mut self, wipe: ContractWipe);
}

impl ContractTokenLifecycleV0Accessors for ContractTokenLifecycleV0 {
    fn issued_supply(&self) -> u128 {
        self.issued_supply
    }

    fn set_issued_supply(&mut self, issued_supply: u128) {
        self.issued_supply = issued_supply;
    }

    fn wiped(&self) -> Option<&ContractWipe> {
        self.wiped.as_ref()
    }

    fn set_wiped(&mut self, wipe: ContractWipe) {
        self.wiped = Some(wipe);
    }
}

/// Accessors for [`ContractWipeV0`].
pub trait ContractWipeV0Accessors {
    /// The height of the block that destroyed the issuer.
    fn block_height(&self) -> u64;

    /// The time of the block that destroyed the issuer, in milliseconds.
    fn block_time_ms(&self) -> u64;
}

impl ContractWipeV0Accessors for ContractWipeV0 {
    fn block_height(&self) -> u64 {
        self.block_height
    }

    fn block_time_ms(&self) -> u64 {
        self.block_time_ms
    }
}
