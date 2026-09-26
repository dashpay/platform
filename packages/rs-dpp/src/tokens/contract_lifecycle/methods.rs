use crate::tokens::contract_lifecycle::v0::{
    ContractTokenLifecycleV0Accessors, ContractWipeV0Accessors,
};
use crate::tokens::contract_lifecycle::{ContractTokenLifecycle, ContractWipe, TokenLifecycle};
use crate::ProtocolError;

impl ContractTokenLifecycleV0Accessors for ContractTokenLifecycle {
    fn issued_supply(&self) -> u128 {
        match self {
            ContractTokenLifecycle::V0(v0) => v0.issued_supply(),
        }
    }

    fn set_issued_supply(&mut self, issued_supply: u128) {
        match self {
            ContractTokenLifecycle::V0(v0) => v0.set_issued_supply(issued_supply),
        }
    }

    fn wiped(&self) -> Option<&ContractWipe> {
        match self {
            ContractTokenLifecycle::V0(v0) => v0.wiped(),
        }
    }

    fn set_wiped(&mut self, wipe: ContractWipe) {
        match self {
            ContractTokenLifecycle::V0(v0) => v0.set_wiped(wipe),
        }
    }
}

impl ContractTokenLifecycle {
    /// Whether the issuer was destroyed.
    pub fn is_wiped(&self) -> bool {
        self.wiped().is_some()
    }

    /// What the record means for the issuer's tokens.
    pub fn token_lifecycle(&self) -> TokenLifecycle {
        match self.wiped() {
            None => TokenLifecycle::Live,
            Some(wipe) => TokenLifecycle::Wiped {
                block_height: wipe.block_height(),
            },
        }
    }

    /// Raises the supply rollup, failing on overflow.
    pub fn checked_add_issued_supply(&mut self, amount: u128) -> Result<(), ProtocolError> {
        let issued_supply = self.issued_supply().checked_add(amount).ok_or_else(|| {
            ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                "adding {} to the issued supply rollup {} would overflow",
                amount,
                self.issued_supply()
            ))
        })?;
        self.set_issued_supply(issued_supply);
        Ok(())
    }

    /// Lowers the supply rollup, failing on underflow.
    pub fn checked_sub_issued_supply(&mut self, amount: u128) -> Result<(), ProtocolError> {
        let issued_supply = self.issued_supply().checked_sub(amount).ok_or_else(|| {
            ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                "removing {} from the issued supply rollup {} would underflow",
                amount,
                self.issued_supply()
            ))
        })?;
        self.set_issued_supply(issued_supply);
        Ok(())
    }
}

impl ContractWipeV0Accessors for ContractWipe {
    fn block_height(&self) -> u64 {
        match self {
            ContractWipe::V0(v0) => v0.block_height(),
        }
    }

    fn block_time_ms(&self) -> u64 {
        match self {
            ContractWipe::V0(v0) => v0.block_time_ms(),
        }
    }
}
