#[macro_export]
macro_rules! dash_to_credits {
    // The macro takes a string literal representing the Dash amount.
    ($dash:expr) => {{
        let dash_str = stringify!($dash);

        // Parsing the input string to separate the whole and fractional parts.
        let parts: Vec<&str> = dash_str.split('.').collect();
        let mut credits: u128 = 0;

        // Process the whole number part if it exists.
        if let Some(whole) = parts.get(0) {
            if let Ok(whole_number) = whole.parse::<u128>() {
                credits += whole_number * 100_000_000_000; // Whole Dash amount to credits
            }
        }

        // Process the fractional part if it exists.
        if let Some(fraction) = parts.get(1) {
            let fraction_length = fraction.len();
            let fraction_number = fraction.parse::<u128>().unwrap_or(0);
            // Calculate the multiplier based on the number of digits in the fraction.
            let multiplier = 10u128.pow(11 - fraction_length as u32);
            credits += fraction_number * multiplier; // Fractional Dash to credits
        }

        credits as u64
    }};
}
