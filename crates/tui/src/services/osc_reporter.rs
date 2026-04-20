use std::io::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscCapability {
    Title,
    Progress,
    Iterm2TabStatus,
    Hyperlink,
}

#[derive(Debug)]
pub struct OscReporter {
    capabilities: Vec<OscCapability>,
}

impl OscReporter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: detect_capabilities(),
        }
    }

    #[must_use]
    pub fn supports(&self, cap: &OscCapability) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn set_title(&self, title: &str) {
        if !self.supports(&OscCapability::Title) {
            return;
        }
        let _ = write_osc(&format!("\x1b]0;{title}\x07"));
    }

    pub fn report_progress(&self, percent: u8) {
        if !self.supports(&OscCapability::Progress) {
            return;
        }
        let state = i32::from(percent < 100);
        let _ = write_osc(&format!("\x1b]9;4;{state};{percent}\x07"));
    }

    pub fn clear_progress(&self) {
        if !self.supports(&OscCapability::Progress) {
            return;
        }
        let _ = write_osc("\x1b]9;4;0;0\x07");
    }

    pub fn set_iterm2_tab_status(&self, text: &str) {
        if !self.supports(&OscCapability::Iterm2TabStatus) {
            return;
        }
        let _ = write_osc(&format!("\x1b]1337;SetBadgeFormat={}\x07", base64(text)));
    }

    #[must_use]
    pub fn hyperlink_start(&self, url: &str) -> String {
        if self.supports(&OscCapability::Hyperlink) {
            format!("\x1b]8;;{url}\x1b\\")
        } else {
            String::new()
        }
    }

    #[must_use]
    pub fn hyperlink_end(&self) -> &'static str {
        if self.supports(&OscCapability::Hyperlink) {
            "\x1b]8;;\x1b\\"
        } else {
            ""
        }
    }
}

impl Default for OscReporter {
    fn default() -> Self {
        Self::new()
    }
}

fn detect_capabilities() -> Vec<OscCapability> {
    let mut caps = vec![OscCapability::Title];

    let term = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let term_lower = term.to_lowercase();

    if term_lower.contains("iterm") || term_lower.contains("wezterm") {
        caps.push(OscCapability::Progress);
        caps.push(OscCapability::Iterm2TabStatus);
    }

    if std::env::var("TERM").unwrap_or_default().contains("xterm")
        || term_lower.contains("iterm")
        || term_lower.contains("wezterm")
        || term_lower.contains("kitty")
    {
        caps.push(OscCapability::Hyperlink);
    }

    caps
}

fn write_osc(seq: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(seq.as_bytes())?;
    stdout.flush()
}

fn base64(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = if chunk.len() > 1 {
            u32::from(chunk[1])
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            u32::from(chunk[2])
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_capability_always_present() {
        let reporter = OscReporter::new();
        assert!(reporter.supports(&OscCapability::Title));
    }

    #[test]
    fn hyperlink_markers() {
        let reporter = OscReporter {
            capabilities: vec![OscCapability::Hyperlink],
        };
        let start = reporter.hyperlink_start("https://example.com");
        assert!(start.contains("https://example.com"));
        assert!(!reporter.hyperlink_end().is_empty());
    }

    #[test]
    fn hyperlink_disabled() {
        let reporter = OscReporter {
            capabilities: vec![],
        };
        assert!(reporter.hyperlink_start("https://example.com").is_empty());
        assert!(reporter.hyperlink_end().is_empty());
    }

    #[test]
    fn base64_encoding() {
        assert_eq!(base64("hello"), "aGVsbG8=");
        assert_eq!(base64(""), "");
    }
}
