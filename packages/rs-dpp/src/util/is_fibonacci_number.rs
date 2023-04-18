fn is_perfect_square(number: u32) -> bool {
    (number as f64).sqrt().fract() == 0.0
}

pub fn is_fibonacci_number(number: u32) -> bool {
    is_perfect_square(5 * number * number + 4) || is_perfect_square(5 * number * number - 4)
}
