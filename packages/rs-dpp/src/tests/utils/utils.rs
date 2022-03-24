use anyhow::Result;
use getrandom::getrandom;

pub fn generate_random_identifier() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    buffer
}

pub fn get_data_from_file(file_path: &str) -> Result<String> {
    let current_dir = std::env::current_dir()?;
    let file_path = format!("{}/{}", current_dir.display(), file_path);
    let d = std::fs::read_to_string(file_path)?;
    Ok(d)
}
