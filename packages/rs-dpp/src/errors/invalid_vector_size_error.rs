use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to covert vector to array of size {expected_size:?}")]
pub struct InvalidVectorSizeError {
    expected_size: usize,
    actual_size: usize,
}

impl InvalidVectorSizeError {
    pub fn new(expected_size: usize, actual_size: usize) -> Self {
        Self { expected_size, actual_size }
    }

    pub fn expected_size(&self) -> usize {
        self.expected_size
    }

    pub fn actual_size(&self) -> usize {
        self.actual_size
    }
}