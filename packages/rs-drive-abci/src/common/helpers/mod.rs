pub mod fee_pools;
pub mod setup;

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    // TODO Move to tests from helpers
    mod overflow {
        use std::str::FromStr;

        #[test]
        fn test_u64_fee_conversion() {
            let processing_fee = u64::MAX;

            let decimal = super::Decimal::from_str(processing_fee.to_string().as_str())
                .expect("should convert u64::MAX to Decimal");

            let converted_to_u64: u64 = decimal
                .try_into()
                .expect("should convert Decimal back to u64::MAX");

            assert_eq!(processing_fee, converted_to_u64);
        }
    }
}
