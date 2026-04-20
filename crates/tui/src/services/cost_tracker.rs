use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub timestamp: Instant,
}

impl TokenUsage {
    #[must_use]
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            timestamp: Instant::now(),
        }
    }

    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_write_tokens
    }
}

#[derive(Debug)]
pub struct CostTracker {
    total_input: u64,
    total_output: u64,
    total_cache_read: u64,
    total_cache_write: u64,
    total_cost_usd: f64,
    turn_count: u32,
    threshold_usd: Option<f64>,
    threshold_acknowledged: bool,
    records: Vec<TokenUsage>,
}

impl CostTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_input: 0,
            total_output: 0,
            total_cache_read: 0,
            total_cache_write: 0,
            total_cost_usd: 0.0,
            turn_count: 0,
            threshold_usd: None,
            threshold_acknowledged: false,
            records: Vec::new(),
        }
    }

    pub fn record_usage(&mut self, usage: TokenUsage, cost_usd: f64) {
        self.total_input += usage.input_tokens;
        self.total_output += usage.output_tokens;
        self.total_cache_read += usage.cache_read_tokens;
        self.total_cache_write += usage.cache_write_tokens;
        self.total_cost_usd += cost_usd;
        self.turn_count += 1;
        self.records.push(usage);
    }

    pub fn set_threshold(&mut self, threshold_usd: f64) {
        self.threshold_usd = Some(threshold_usd);
        self.threshold_acknowledged = false;
    }

    pub fn acknowledge_threshold(&mut self) {
        self.threshold_acknowledged = true;
    }

    #[must_use]
    pub fn threshold_exceeded(&self) -> bool {
        self.threshold_usd
            .is_some_and(|t| self.total_cost_usd >= t && !self.threshold_acknowledged)
    }

    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.total_input + self.total_output + self.total_cache_read + self.total_cache_write
    }

    #[must_use]
    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_usd
    }

    #[must_use]
    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }

    #[must_use]
    pub fn total_input(&self) -> u64 {
        self.total_input
    }

    #[must_use]
    pub fn total_output(&self) -> u64 {
        self.total_output
    }

    #[must_use]
    pub fn total_cache_read(&self) -> u64 {
        self.total_cache_read
    }

    #[must_use]
    pub fn total_cache_write(&self) -> u64 {
        self.total_cache_write
    }

    #[must_use]
    pub fn threshold_usd(&self) -> Option<f64> {
        self.threshold_usd
    }

    #[must_use]
    pub fn cost_summary(&self) -> String {
        format!(
            "${:.4} | {} turns | {} tokens",
            self.total_cost_usd,
            self.turn_count,
            format_tokens(self.total_tokens()),
        )
    }

    #[must_use]
    pub fn token_breakdown(&self) -> String {
        format!(
            "In: {} | Out: {} | Cache R: {} | Cache W: {}",
            format_tokens(self.total_input),
            format_tokens(self.total_output),
            format_tokens(self.total_cache_read),
            format_tokens(self.total_cache_write),
        )
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tracker() {
        let tracker = CostTracker::new();
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_cost_usd(), 0.0);
        assert_eq!(tracker.turn_count(), 0);
        assert!(!tracker.threshold_exceeded());
    }

    #[test]
    fn record_usage() {
        let mut tracker = CostTracker::new();
        tracker.record_usage(TokenUsage::new(100, 200), 0.01);
        assert_eq!(tracker.total_input(), 100);
        assert_eq!(tracker.total_output(), 200);
        assert_eq!(tracker.total_tokens(), 300);
        assert_eq!(tracker.turn_count(), 1);
    }

    #[test]
    fn accumulates() {
        let mut tracker = CostTracker::new();
        tracker.record_usage(TokenUsage::new(100, 50), 0.01);
        tracker.record_usage(TokenUsage::new(200, 75), 0.02);
        assert_eq!(tracker.total_input(), 300);
        assert_eq!(tracker.total_output(), 125);
        assert_eq!(tracker.turn_count(), 2);
        assert!((tracker.total_cost_usd() - 0.03).abs() < f64::EPSILON);
    }

    #[test]
    fn threshold() {
        let mut tracker = CostTracker::new();
        tracker.set_threshold(0.05);
        tracker.record_usage(TokenUsage::new(100, 50), 0.06);
        assert!(tracker.threshold_exceeded());
        tracker.acknowledge_threshold();
        assert!(!tracker.threshold_exceeded());
    }

    #[test]
    fn cost_summary_format() {
        let mut tracker = CostTracker::new();
        tracker.record_usage(TokenUsage::new(1500, 500), 0.0123);
        let summary = tracker.cost_summary();
        assert!(summary.contains("$0.0123"));
        assert!(summary.contains("1 turns"));
        assert!(summary.contains("2.0K tokens"));
    }

    #[test]
    fn format_tokens_units() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(2_500_000), "2.5M");
    }

    #[test]
    fn token_breakdown_format() {
        let mut tracker = CostTracker::new();
        let mut usage = TokenUsage::new(1000, 500);
        usage.cache_read_tokens = 200;
        usage.cache_write_tokens = 100;
        tracker.record_usage(usage, 0.01);
        let breakdown = tracker.token_breakdown();
        assert!(breakdown.contains("In: 1.0K"));
        assert!(breakdown.contains("Out: 500"));
        assert!(breakdown.contains("Cache R: 200"));
        assert!(breakdown.contains("Cache W: 100"));
    }
}
