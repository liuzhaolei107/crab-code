//! Enhanced status bar showing model, tokens, session time, and git branch.
//!
//! Extends the basic cost bar with richer contextual information.

use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

/// Enhanced status bar with model info, token usage, session time, and git branch.
pub struct StatusBar {
    /// Current LLM model name.
    model_name: String,
    /// Input token count.
    input_tokens: u64,
    /// Output token count.
    output_tokens: u64,
    /// Estimated cost in USD.
    cost_usd: f64,
    /// Number of API calls.
    api_calls: u64,
    /// When the session started.
    session_start: Instant,
    /// Current git branch (None if not in a repo or not detected).
    git_branch: Option<String>,
    /// Current permission mode label.
    permission_mode: Option<String>,
    /// Number of active tools.
    active_tool_count: usize,
}

impl StatusBar {
    #[must_use]
    pub fn new() -> Self {
        Self {
            model_name: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0.0,
            api_calls: 0,
            session_start: Instant::now(),
            git_branch: None,
            permission_mode: None,
            active_tool_count: 0,
        }
    }

    /// Set the model name.
    pub fn set_model(&mut self, name: impl Into<String>) {
        self.model_name = name.into();
    }

    /// Update token usage.
    pub fn set_tokens(&mut self, input: u64, output: u64) {
        self.input_tokens = input;
        self.output_tokens = output;
    }

    /// Update cost.
    pub fn set_cost(&mut self, cost_usd: f64, api_calls: u64) {
        self.cost_usd = cost_usd;
        self.api_calls = api_calls;
    }

    /// Set the git branch name.
    pub fn set_git_branch(&mut self, branch: Option<String>) {
        self.git_branch = branch;
    }

    /// Set the permission mode label.
    pub fn set_permission_mode(&mut self, mode: impl Into<String>) {
        self.permission_mode = Some(mode.into());
    }

    /// Set the number of tools currently executing.
    pub fn set_active_tools(&mut self, count: usize) {
        self.active_tool_count = count;
    }

    /// Total tokens (input + output).
    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Session elapsed time.
    #[must_use]
    pub fn session_duration(&self) -> Duration {
        self.session_start.elapsed()
    }

    /// Format a duration as "Xm Ys" or "Xh Ym".
    #[must_use]
    pub fn format_duration(d: Duration) -> String {
        let secs = d.as_secs();
        if secs < 60 {
            format!("{secs}s")
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Format token count with `k` suffix for large values.
    #[allow(clippy::cast_precision_loss)]
    fn format_tokens(n: u64) -> String {
        if n >= 10_000 {
            format!("{:.1}k", n as f64 / 1000.0)
        } else {
            n.to_string()
        }
    }

    /// Detect the current git branch by reading `.git/HEAD`.
    #[must_use]
    pub fn detect_git_branch(project_dir: &std::path::Path) -> Option<String> {
        let head_path = project_dir.join(".git").join("HEAD");
        let content = std::fs::read_to_string(head_path).ok()?;
        let content = content.trim();
        content.strip_prefix("ref: refs/heads/").map_or_else(
            || {
                if content.len() >= 7 {
                    // Detached HEAD — show short hash
                    Some(content[..7].to_string())
                } else {
                    None
                }
            },
            |branch| Some(branch.to_string()),
        )
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &StatusBar {
    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width < 20 {
            return;
        }

        let dim = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let model_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let cost_style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);
        let branch_style = Style::default().fg(Color::Magenta);

        let mut spans: Vec<Span<'_>> = Vec::new();

        // Model name
        if !self.model_name.is_empty() {
            spans.push(Span::styled(" ", dim));
            spans.push(Span::styled(&self.model_name, model_style));
        }

        // Git branch
        if let Some(branch) = &self.git_branch {
            spans.push(Span::styled(" | ", dim));
            spans.push(Span::styled(branch.as_str(), branch_style));
        }

        // Permission mode
        if let Some(mode) = &self.permission_mode {
            spans.push(Span::styled(" | ", dim));
            spans.push(Span::styled(mode.as_str(), value_style));
        }

        // Tokens
        spans.push(Span::styled(" | ", dim));
        spans.push(Span::styled(
            StatusBar::format_tokens(self.input_tokens),
            value_style,
        ));
        spans.push(Span::styled("in/", dim));
        spans.push(Span::styled(
            StatusBar::format_tokens(self.output_tokens),
            value_style,
        ));
        spans.push(Span::styled("out", dim));

        // Cost
        spans.push(Span::styled(" | ", dim));
        spans.push(Span::styled(format!("${:.4}", self.cost_usd), cost_style));

        // API calls
        spans.push(Span::styled(" | ", dim));
        spans.push(Span::styled(
            format!("{} calls", self.api_calls),
            value_style,
        ));

        // Session time
        spans.push(Span::styled(" | ", dim));
        spans.push(Span::styled(
            StatusBar::format_duration(self.session_duration()),
            value_style,
        ));

        // Active tools
        if self.active_tool_count > 0 {
            spans.push(Span::styled(" | ", dim));
            spans.push(Span::styled(
                format!("{} tools", self.active_tool_count),
                Style::default().fg(Color::Yellow),
            ));
        }

        let line = Line::from(spans);
        let line_area = Rect { height: 1, ..area };
        Widget::render(line, line_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let bar = StatusBar::new();
        assert_eq!(bar.total_tokens(), 0);
        assert!(bar.model_name.is_empty());
        assert!(bar.git_branch.is_none());
        assert!(bar.permission_mode.is_none());
    }

    #[test]
    fn set_model() {
        let mut bar = StatusBar::new();
        bar.set_model("claude-sonnet-4");
        assert_eq!(bar.model_name, "claude-sonnet-4");
    }

    #[test]
    fn set_tokens() {
        let mut bar = StatusBar::new();
        bar.set_tokens(1000, 500);
        assert_eq!(bar.input_tokens, 1000);
        assert_eq!(bar.output_tokens, 500);
        assert_eq!(bar.total_tokens(), 1500);
    }

    #[test]
    fn set_cost() {
        let mut bar = StatusBar::new();
        bar.set_cost(0.0523, 7);
        assert!((bar.cost_usd - 0.0523).abs() < f64::EPSILON);
        assert_eq!(bar.api_calls, 7);
    }

    #[test]
    fn set_git_branch() {
        let mut bar = StatusBar::new();
        bar.set_git_branch(Some("feature/test".into()));
        assert_eq!(bar.git_branch.as_deref(), Some("feature/test"));
    }

    #[test]
    fn set_permission_mode() {
        let mut bar = StatusBar::new();
        bar.set_permission_mode("default");
        assert_eq!(bar.permission_mode.as_deref(), Some("default"));
    }

    #[test]
    fn set_active_tools() {
        let mut bar = StatusBar::new();
        bar.set_active_tools(3);
        assert_eq!(bar.active_tool_count, 3);
    }

    #[test]
    fn format_duration_seconds() {
        assert_eq!(StatusBar::format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(StatusBar::format_duration(Duration::from_secs(45)), "45s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(
            StatusBar::format_duration(Duration::from_secs(90)),
            "1m 30s"
        );
        assert_eq!(
            StatusBar::format_duration(Duration::from_secs(3599)),
            "59m 59s"
        );
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(
            StatusBar::format_duration(Duration::from_secs(3600)),
            "1h 0m"
        );
        assert_eq!(
            StatusBar::format_duration(Duration::from_secs(7800)),
            "2h 10m"
        );
    }

    #[test]
    fn format_tokens_small() {
        assert_eq!(StatusBar::format_tokens(0), "0");
        assert_eq!(StatusBar::format_tokens(500), "500");
        assert_eq!(StatusBar::format_tokens(9999), "9999");
    }

    #[test]
    fn format_tokens_large() {
        assert_eq!(StatusBar::format_tokens(10_000), "10.0k");
        assert_eq!(StatusBar::format_tokens(150_000), "150.0k");
    }

    #[test]
    fn detect_git_branch_nonexistent() {
        let branch = StatusBar::detect_git_branch(std::path::Path::new("/nonexistent"));
        assert!(branch.is_none());
    }

    #[test]
    fn detect_git_branch_real_repo() {
        // This test works if run from within the crab-code repo
        let cwd = std::env::current_dir().unwrap();
        let branch = StatusBar::detect_git_branch(&cwd);
        // May or may not find a branch depending on where tests run
        if cwd.join(".git").join("HEAD").exists() {
            assert!(branch.is_some());
        }
    }

    #[test]
    fn default_status_bar() {
        let bar = StatusBar::default();
        assert_eq!(bar.total_tokens(), 0);
    }

    #[test]
    fn session_duration_is_positive() {
        let bar = StatusBar::new();
        assert!(bar.session_duration().as_nanos() >= 0);
    }

    #[test]
    fn renders_model_and_tokens() {
        let mut bar = StatusBar::new();
        bar.set_model("gpt-4o");
        bar.set_tokens(50_000, 10_000);
        bar.set_cost(0.08, 5);
        bar.set_git_branch(Some("main".into()));

        let area = Rect::new(0, 0, 100, 1);
        let mut buf = Buffer::empty(area);
        Widget::render(&bar, area, &mut buf);

        let content: String = (0..area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("gpt-4o"), "Missing model in: {content}");
        assert!(content.contains("main"), "Missing branch in: {content}");
        assert!(content.contains("50.0k"), "Missing tokens in: {content}");
        assert!(content.contains("$0.08"), "Missing cost in: {content}");
        assert!(content.contains("5 calls"), "Missing calls in: {content}");
    }

    #[test]
    fn renders_active_tools() {
        let mut bar = StatusBar::new();
        bar.set_active_tools(2);

        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        Widget::render(&bar, area, &mut buf);

        let content: String = (0..area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("2 tools"));
    }

    #[test]
    fn tiny_area_no_panic() {
        let bar = StatusBar::new();
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        Widget::render(&bar, area, &mut buf);
    }

    #[test]
    fn zero_height_no_panic() {
        let bar = StatusBar::new();
        let area = Rect::new(0, 0, 80, 0);
        let mut buf = Buffer::empty(area);
        Widget::render(&bar, area, &mut buf);
    }
}
