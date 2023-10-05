pub fn convert_to_base58_chars(input: &str) -> String {
    let mut replaced = String::with_capacity(input.len());

    for c in input.chars() {
        match c {
            '0' => replaced.push('o'),
            'l' => replaced.push('1'),
            _ => replaced.push(c),
        }
    }

    replaced
}

#[cfg(test)]
mod tests {

    use super::*;
    mod convert_to_base58_chars {
        use super::*;

        #[test]
        fn should_convert_0_and_l_to_o_and_l() {
            let result = convert_to_base58_chars("a0b0c0dlelfl");

            assert_eq!(result, "aobocod1e1f1");
        }
    }
}
