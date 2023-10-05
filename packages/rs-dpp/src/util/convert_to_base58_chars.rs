pub fn convert_to_base58_chars(input: &str) -> String {
    let mut replaced = String::with_capacity(input.len());

    for c in input.chars() {
        match c {
            'o' => replaced.push('0'),
            'i' => replaced.push('1'),
            _ => replaced.push(c),
        }
    }

    replaced
}
