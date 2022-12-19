use getrandom::getrandom;

pub fn generate() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    buffer
}
