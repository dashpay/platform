pub fn convert_to_homograph_safe_chars(input: &str) -> String {
    let mut replaced = String::with_capacity(input.len());

    for c in input.to_lowercase().chars() {
        match c {
            'o' => replaced.push('0'),
            'l' | 'i' => replaced.push('1'),
            _ => replaced.push(c),
        }
    }

    replaced
}

#[cfg(test)]
mod tests {

    use super::*;
    mod convert_to_homograph_safe_chars {
        use super::*;

        #[test]
        fn should_convert_0_and_l_to_o_and_l() {
            let result = convert_to_homograph_safe_chars("A0boic0Dlelfl");

            assert_eq!(result, "a0b01c0d1e1f1");
        }
    }
}
