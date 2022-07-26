use rs_drive::drive::fee_pools::epochs::constants::PERPETUAL_STORAGE_YEARS;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub const DEFAULT_ORIGINAL_FEE_MULTIPLIER: f64 = 2.0;

// TODO: Should be updated from the doc

#[rustfmt::skip]
pub const FEE_DISTRIBUTION_TABLE: [Decimal; PERPETUAL_STORAGE_YEARS as usize] = [
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
