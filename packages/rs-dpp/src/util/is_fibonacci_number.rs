fn is_perfect_square(number: u64) -> bool {
    (number as f64).sqrt().fract() == 0.0
}

pub fn is_fibonacci_number(number: u64) -> bool {
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
        _ => false, // Return false if either calculation overflows
    }
}
