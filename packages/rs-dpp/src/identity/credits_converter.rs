use crate::fee::Credits;
use crate::ProtocolError;
use crate::balances::credits::CREDITS_PER_DUFF;

pub fn convert_duffs_to_credits(amount: u64) -> Result<Credits, ProtocolError> {
    amount.checked_mul(CREDITS_PER_DUFF).ok_or(ProtocolError::Overflow(
        "converting duffs to credits failed",
    ))
}

pub fn convert_credits_to_duffs(amount: Credits) -> Result<u64, ProtocolError> {
    amount.checked_div(CREDITS_PER_DUFF).ok_or(ProtocolError::Overflow(
        "converting credits to duffs failed",
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_should_convert_satoshi_to_credits() {
        let amount = 42;
        let converted = convert_duffs_to_credits(amount).unwrap();
        assert_eq!(converted, amount * CREDITS_PER_DUFF);
    }

    #[test]
    fn test_should_convert_credits_to_satoshi() {
        let amount = 10000;
        let converted = convert_credits_to_duffs(amount).unwrap();
        assert_eq!(converted, amount / CREDITS_PER_DUFF);
    }

    #[test]
    fn test_convert_to_0_satoshi_if_amount_lower_than_ratio() {
        let amount = CREDITS_PER_DUFF - 1;
        let converted = convert_credits_to_duffs(amount).unwrap();
        assert_eq!(converted, 0);
    }
}
