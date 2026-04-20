use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Checking,
    Downloading,
    Installing,
    Installed(String),
    Failed(String),
    UpToDate,
}

pub struct UpdateBanner {
    status: UpdateStatus,
    visible: bool,
}

impl UpdateBanner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            status: UpdateStatus::UpToDate,
            visible: false,
        }
    }

    pub fn set_status(&mut self, status: UpdateStatus) {
        self.visible = !matches!(status, UpdateStatus::UpToDate);
        self.status = status;
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    #[must_use]
    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }
}

impl Default for UpdateBanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for UpdateBanner {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible || area.height == 0 || area.width == 0 {
            return;
        }

        let (text, style) = match &self.status {
            UpdateStatus::Checking => (
                " ⟳ Checking for updates...".to_string(),
                Style::default().fg(Color::DarkGray),
            ),
            UpdateStatus::Downloading => (
                " ⟳ Downloading update...".to_string(),
                Style::default().fg(Color::Cyan),
            ),
            UpdateStatus::Installing => (
                " ⟳ Installing update...".to_string(),
                Style::default().fg(Color::Cyan),
            ),
            UpdateStatus::Installed(version) => (
                format!(" ✓ Updated to {version}. Restart to apply."),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            UpdateStatus::Failed(reason) => (
                format!(" ✗ Update failed: {reason}"),
                Style::default().fg(Color::Red),
            ),
            UpdateStatus::UpToDate => return,
        };

        let row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        let line = Line::from(vec![Span::styled(text, style)]);
        Widget::render(line, row, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::from(self.visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_when_up_to_date() {
        let banner = UpdateBanner::new();
        assert!(!banner.is_visible());
    }

    #[test]
    fn shows_on_status_change() {
        let mut banner = UpdateBanner::new();
        banner.set_status(UpdateStatus::Downloading);
        assert!(banner.is_visible());
    }

    #[test]
    fn dismiss() {
        let mut banner = UpdateBanner::new();
        banner.set_status(UpdateStatus::Installing);
        banner.dismiss();
        assert!(!banner.is_visible());
    }

    #[test]
    fn installed_shows_version() {
        let mut banner = UpdateBanner::new();
        banner.set_status(UpdateStatus::Installed("1.2.3".into()));
        assert!(banner.is_visible());
    }

    #[test]
    fn render_no_panic() {
        let mut banner = UpdateBanner::new();
        banner.set_status(UpdateStatus::Installed("2.0.0".into()));
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        banner.render(area, &mut buf);
    }
}
