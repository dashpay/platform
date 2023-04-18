pub struct TimeWindowValidationResult {
    pub valid: bool,
    pub time_window_start: u64,
    pub time_window_end: u64,
}

impl TimeWindowValidationResult {
    pub fn get_time_window_start(&self) -> u64 {
        self.time_window_start
    }

    pub fn get_time_window_end(&self) -> u64 {
        self.time_window_end
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}
