pub const RATIO: i64 = 1000;

pub fn convert_satoshi_to_credits(amount: i64) -> i64 {
    amount * RATIO
}

pub fn convert_credits_to_satoshi(amount: i64) -> i64 {
    amount / RATIO
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_should_convert_satoshi_to_credits() {
        let amount = 42;
        let converted = convert_satoshi_to_credits(amount);

        assert_eq!(converted, amount * RATIO);
    }

    #[test]
    fn test_should_convert_credits_to_satoshi() {
        let amount = 10000;
        let converted = convert_credits_to_satoshi(amount);
        assert_eq!(converted, amount / RATIO);
    }

    #[test]
    fn test_convert_to_0_satoshi_if_amount_lower_than_ratio() {
        let amount = RATIO - 1;
        let converted = convert_credits_to_satoshi(amount);
        assert_eq!(converted, 0);
    }
}
