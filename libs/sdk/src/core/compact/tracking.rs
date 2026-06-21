#[derive(Debug, Clone, Default)]
pub struct AutoCompactState {
    pub compacted: bool,
    pub consecutive_failures: usize,
}

impl AutoCompactState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&mut self) {
        self.compacted = true;
        self.consecutive_failures = 0;
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
    }

    pub fn should_skip(&self) -> bool {
        self.consecutive_failures >= 3
    }
}
