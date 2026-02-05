fn is_perfect_square(number: u64) -> bool {
    if number < 2 {
        return true;
    }
    // Integer square root via Newton's method
    let mut x = number;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + number / x) / 2;
    }
    x * x == number
}

pub fn is_non_zero_fibonacci_number(number: u64) -> bool {
    if number == 0 {
        return false;
    }

    let square_check_up = 5u64
        .checked_mul(number)
        .and_then(|n| n.checked_mul(number))
        .and_then(|n| n.checked_add(4));

    let square_check_down = 5u64
        .checked_mul(number)
        .and_then(|n| n.checked_mul(number))
        .and_then(|n| n.checked_sub(4));

    match (square_check_up, square_check_down) {
        (Some(n1), Some(n2)) => is_perfect_square(n1) || is_perfect_square(n2),
        (Some(n1), None) => is_perfect_square(n1),
        (None, Some(n2)) => is_perfect_square(n2),
        (None, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AUDIT M6: Floating-point imprecision in is_perfect_square for large values.
    ///
    /// `is_perfect_square` casts u64 to f64 and checks `sqrt().fract() == 0.0`.
    /// f64 has only 52 bits of mantissa, so for values above 2^52, precision is lost.
    /// This means `is_perfect_square` can return incorrect results for large numbers.
    ///
    /// Currently not exploitable for `core_fee_per_byte` (u32), but the function
    /// accepts u64 and is technically unsound for large inputs.
    ///
    /// Location: rs-dpp/src/util/is_non_zero_fibonacci_number.rs:1-3
    #[test]
    fn test_is_perfect_square_large_values() {
        // For small values, is_perfect_square works correctly
        assert!(is_perfect_square(0));
        assert!(is_perfect_square(1));
        assert!(is_perfect_square(4));
        assert!(is_perfect_square(9));
        assert!(is_perfect_square(16));
        assert!(!is_perfect_square(2));
        assert!(!is_perfect_square(3));

        // Find a value where f64 imprecision causes is_perfect_square to give wrong answer.
        // The number (2^26 + 1)^2 = 2^52 + 2^27 + 1 = 4503599761588225
        // When cast to f64, this may lose the +1 and appear as (2^26 + 1)^2 exactly,
        // or nearby non-squares may appear as perfect squares.
        //
        // Strategy: find a non-square near a large perfect square where f64 rounds
        // the sqrt to an exact integer.
        let base: u64 = (1u64 << 26) + 1; // 67108865
        let perfect_square = base * base; // 4503599761588225

        // Verify perfect_square is correctly identified
        assert!(
            is_perfect_square(perfect_square),
            "AUDIT M6: {} should be a perfect square ({}^2)",
            perfect_square,
            base
        );

        // Check non-squares adjacent to large perfect squares.
        // Due to f64 imprecision, some of these may incorrectly return true.
        let false_positives: Vec<u64> = (1..=10u64)
            .filter(|&offset| {
                let candidate = perfect_square + offset;
                // This should NOT be a perfect square, but f64 may say it is
                is_perfect_square(candidate)
            })
            .collect();

        // If is_perfect_square is sound, there should be no false positives.
        // With f64 imprecision, some non-squares will be falsely identified as perfect squares.
        assert!(
            false_positives.is_empty(),
            "AUDIT M6: is_perfect_square returned true for non-square values near {}^2: \
            offsets {:?}. This is caused by f64 imprecision — (number as f64).sqrt() \
            rounds to an exact integer for these non-square values. \
            Fix: use integer square root instead of floating-point.",
            base,
            false_positives
        );
    }

    /// AUDIT M6 (supplementary): Verify is_non_zero_fibonacci_number works for known values
    #[test]
    fn test_known_non_zero_fibonacci_numbers() {
        // Known non-zero Fibonacci numbers
        let fibs = [
            1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597,
        ];
        for &n in &fibs {
            assert!(
                is_non_zero_fibonacci_number(n),
                "{} should be recognized as a Fibonacci number",
                n
            );
        }

        // Known non-Fibonacci numbers
        let non_fibs = [0, 4, 6, 7, 9, 10, 11, 12, 14, 15, 16, 22, 100];
        for &n in &non_fibs {
            assert!(
                !is_non_zero_fibonacci_number(n),
                "{} should NOT be recognized as a Fibonacci number",
                n
            );
        }
    }
}
