pub fn convert_to_base58_chars(input: &str) -> String {
    input.replace("o", "0").replace("i", "1")
}
