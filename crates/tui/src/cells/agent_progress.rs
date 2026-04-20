use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Done,
    Error,
}

impl AgentStatus {
    #[must_use]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Running => "⏺",
            Self::Done => "✓",
            Self::Error => "✗",
        }
    }

    #[must_use]
    pub fn color(&self) -> Color {
        match self {
            Self::Running => Color::Cyan,
            Self::Done => Color::Green,
            Self::Error => Color::Red,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentProgressNode {
    pub name: String,
    pub status: AgentStatus,
    pub tool_count: usize,
    pub depth: usize,
}

impl AgentProgressNode {
    #[must_use]
    pub fn new(name: impl Into<String>, status: AgentStatus, depth: usize) -> Self {
        Self {
            name: name.into(),
            status,
            tool_count: 0,
            depth,
        }
    }
}

pub struct AgentProgressTree {
    nodes: Vec<AgentProgressNode>,
}

impl AgentProgressTree {
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn set_nodes(&mut self, nodes: Vec<AgentProgressNode>) {
        self.nodes = nodes;
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for AgentProgressTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for AgentProgressTree {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if self.nodes.is_empty() || area.height == 0 {
            return;
        }

        for (i, node) in self.nodes.iter().enumerate() {
            if i as u16 >= area.height {
                break;
            }
            let indent = "  ".repeat(node.depth);
            let icon_style = Style::default()
                .fg(node.status.color())
                .add_modifier(Modifier::BOLD);
            let name_style = Style::default().fg(Color::White);
            let detail_style = Style::default().fg(Color::DarkGray);

            let tool_text = if node.tool_count > 0 {
                format!(" ({} tools)", node.tool_count)
            } else {
                String::new()
            };

            let status_label = match node.status {
                AgentStatus::Running => " running",
                AgentStatus::Done => " done",
                AgentStatus::Error => " error",
            };

            let line = Line::from(vec![
                Span::raw(indent),
                Span::styled(format!("{} ", node.status.icon()), icon_style),
                Span::styled(node.name.clone(), name_style),
                Span::styled(tool_text, detail_style),
                Span::styled(
                    status_label.to_string(),
                    Style::default()
                        .fg(node.status.color())
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);

            let row = Rect {
                x: area.x,
                y: area.y + i as u16,
                width: area.width,
                height: 1,
            };
            Widget::render(line, row, buf);
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        self.nodes.len() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree() {
        let tree = AgentProgressTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.desired_height(80), 0);
    }

    #[test]
    fn nodes_set_and_height() {
        let mut tree = AgentProgressTree::new();
        tree.set_nodes(vec![
            AgentProgressNode::new("main", AgentStatus::Running, 0),
            AgentProgressNode::new("researcher", AgentStatus::Done, 1),
        ]);
        assert!(!tree.is_empty());
        assert_eq!(tree.desired_height(80), 2);
    }

    #[test]
    fn status_icons() {
        assert_eq!(AgentStatus::Running.icon(), "⏺");
        assert_eq!(AgentStatus::Done.icon(), "✓");
        assert_eq!(AgentStatus::Error.icon(), "✗");
    }

    #[test]
    fn render_no_panic() {
        let mut tree = AgentProgressTree::new();
        tree.set_nodes(vec![
            AgentProgressNode::new("main", AgentStatus::Running, 0),
            AgentProgressNode {
                name: "worker".into(),
                status: AgentStatus::Done,
                tool_count: 3,
                depth: 1,
            },
        ]);
        let area = Rect::new(0, 0, 80, 10);
        let mut buf = Buffer::empty(area);
        tree.render(area, &mut buf);
    }
}
