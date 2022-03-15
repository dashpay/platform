use getrandom::getrandom;

pub fn generate_random_identifier() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    getrandom(&mut buffer);
    buffer
}
