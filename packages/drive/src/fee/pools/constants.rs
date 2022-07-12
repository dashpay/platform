use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub const KEY_STORAGE_FEE_POOL: &[u8; 1] = b"s";

#[rustfmt::skip]
pub const FEE_DISTRIBUTION_TABLE: [Decimal; 50] = [
    dec!(0.05000), dec!(0.04800), dec!(0.04600), dec!(0.04400), dec!(0.04200),
    dec!(0.04000), dec!(0.03850), dec!(0.03700), dec!(0.03550), dec!(0.03400),
    dec!(0.03250), dec!(0.03100), dec!(0.02950), dec!(0.02850), dec!(0.02750),
    dec!(0.02650), dec!(0.02550), dec!(0.02450), dec!(0.02350), dec!(0.02250),
    dec!(0.02150), dec!(0.02050), dec!(0.01950), dec!(0.01875), dec!(0.01800),
    dec!(0.01725), dec!(0.01650), dec!(0.01575), dec!(0.01500), dec!(0.01425),
    dec!(0.01350), dec!(0.01275), dec!(0.01200), dec!(0.01125), dec!(0.01050),
    dec!(0.00975), dec!(0.00900), dec!(0.00825), dec!(0.00750), dec!(0.00675),
    dec!(0.00600), dec!(0.00525), dec!(0.00475), dec!(0.00425), dec!(0.00375),
    dec!(0.00325), dec!(0.00275), dec!(0.00225), dec!(0.00175), dec!(0.00125),
];

pub const MN_REWARD_SHARES_CONTRACT_ID: [u8; 32] = [
    0x0c, 0xac, 0xe2, 0x05, 0x24, 0x66, 0x93, 0xa7, 0xc8, 0x15, 0x65, 0x23, 0x62, 0x0d, 0xaa, 0x93,
    0x7d, 0x2f, 0x22, 0x47, 0x93, 0x44, 0x63, 0xee, 0xb0, 0x1f, 0xf7, 0x21, 0x95, 0x90, 0x95, 0x8c,
];

pub const MN_REWARD_SHARES_DOCUMENT_TYPE: &str = "rewardShare";

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[test]
    fn test_distribution_table_sum() {
        assert_eq!(
            super::FEE_DISTRIBUTION_TABLE.iter().sum::<Decimal>(),
            dec!(1.0),
        );
    }

    #[test]
    fn test_distribution_of_value() {
        let mut buffer = dec!(0.0);
        let value = Decimal::new(i64::MAX, 0);

        for i in 0..50 {
            let share = value * super::FEE_DISTRIBUTION_TABLE[i];
            buffer += share;
        }

        assert_eq!(buffer, value);
    }
}
