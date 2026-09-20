use dpp::balances::credits::Credits;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::document_type::action_fees::ContractFeePot;

/// What Drive holds about one of a contract's fee pots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContractFeePotState {
    /// The credits in the pot
    pub credits: Credits,
    /// The epoch the pot was last claimed in, `None` when it never was
    pub last_claim_epoch: Option<EpochIndex>,
}

/// What Drive holds about both fee pots of a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContractFeePots {
    /// The pot the contract owner claims
    pub owner: ContractFeePotState,
    /// The pot the contract's moderation team shares
    pub moderators: ContractFeePotState,
}

impl ContractFeePots {
    /// The state of `pot`
    pub fn pot(&self, pot: ContractFeePot) -> &ContractFeePotState {
        match pot {
            ContractFeePot::Owner => &self.owner,
            ContractFeePot::Moderators => &self.moderators,
        }
    }

    /// The state of `pot`, mutably
    pub fn pot_mut(&mut self, pot: ContractFeePot) -> &mut ContractFeePotState {
        match pot {
            ContractFeePot::Owner => &mut self.owner,
            ContractFeePot::Moderators => &mut self.moderators,
        }
    }
}

/// The stored form of a last claim epoch: the epoch index, two bytes big endian.
pub fn encode_epoch_index(epoch_index: EpochIndex) -> Vec<u8> {
    epoch_index.to_be_bytes().to_vec()
}

/// Reads a stored last claim epoch.
pub fn decode_epoch_index(value: &[u8]) -> Result<EpochIndex, String> {
    let bytes: [u8; 2] = value
        .try_into()
        .map_err(|_| format!("expected 2 bytes, got {}", value.len()))?;
    Ok(EpochIndex::from_be_bytes(bytes))
}
