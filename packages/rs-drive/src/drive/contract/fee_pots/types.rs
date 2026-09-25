use dpp::balances::credits::Credits;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;

/// What Drive holds about one of a contract's fee pots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContractFeePotState {
    /// The credits in the pot
    pub credits: Credits,
    /// The last payout of the pot (its epoch, its block time and who claimed it), `None` when
    /// the pot was never paid out
    pub last_claim: Option<ContractFeePotLastClaim>,
}

impl ContractFeePotState {
    /// The epoch the pot was last paid out in, `None` when it never was. A pot is paid out at
    /// most once per epoch.
    pub fn last_claim_epoch(&self) -> Option<EpochIndex> {
        self.last_claim.map(|last_claim| last_claim.epoch_index)
    }
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

/// The stored size of a last claim: the epoch index (2 bytes), the block time (8 bytes) and the
/// claimant's id (32 bytes). Every last claim has this size, so a claim that replaces the one
/// before it never changes the size of the item.
pub const CONTRACT_FEE_POT_LAST_CLAIM_SIZE: usize = 2 + 8 + 32;

/// The stored form of a last claim: the epoch index, two bytes big endian, the block time in
/// milliseconds, eight bytes big endian, and the 32 bytes of the claimant's id.
pub fn encode_last_claim(last_claim: &ContractFeePotLastClaim) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(CONTRACT_FEE_POT_LAST_CLAIM_SIZE);
    bytes.extend_from_slice(&last_claim.epoch_index.to_be_bytes());
    bytes.extend_from_slice(&last_claim.time_ms.to_be_bytes());
    bytes.extend_from_slice(last_claim.claimant_id.as_slice());
    bytes
}

/// Reads a stored last claim.
pub fn decode_last_claim(value: &[u8]) -> Result<ContractFeePotLastClaim, String> {
    let bytes: &[u8; CONTRACT_FEE_POT_LAST_CLAIM_SIZE] = value.try_into().map_err(|_| {
        format!(
            "expected {} bytes, got {}",
            CONTRACT_FEE_POT_LAST_CLAIM_SIZE,
            value.len()
        )
    })?;
    let (epoch_index, rest) = bytes.split_at(2);
    let (time_ms, claimant_id) = rest.split_at(8);
    let malformed = |what: &str| format!("the {what} of a last claim has the wrong length");
    Ok(ContractFeePotLastClaim {
        epoch_index: EpochIndex::from_be_bytes(
            epoch_index
                .try_into()
                .map_err(|_| malformed("epoch index"))?,
        ),
        time_ms: TimestampMillis::from_be_bytes(
            time_ms.try_into().map_err(|_| malformed("block time"))?,
        ),
        claimant_id: Identifier::from_bytes(claimant_id).map_err(|_| malformed("claimant id"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_a_last_claim_through_its_stored_form() {
        let last_claim = ContractFeePotLastClaim {
            epoch_index: 0x0102,
            time_ms: 0x0304_0506_0708_090a,
            claimant_id: Identifier::from([0xee; 32]),
        };
        let stored = encode_last_claim(&last_claim);
        let mut expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        expected.extend_from_slice(&[0xee; 32]);
        assert_eq!(stored, expected);
        assert_eq!(decode_last_claim(&stored), Ok(last_claim));
    }

    #[test]
    fn should_refuse_a_stored_last_claim_of_another_size() {
        // The two bytes of the epoch alone, which is what a claim stored before it recorded
        // its time and its claimant.
        for stored in [vec![], vec![0, 7], vec![0; 41], vec![0; 43]] {
            let err = decode_last_claim(&stored).expect_err("expected the value to be refused");
            assert!(err.contains("expected 42 bytes"), "{err}");
        }
    }
}
