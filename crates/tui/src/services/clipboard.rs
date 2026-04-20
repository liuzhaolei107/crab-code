use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardBackend {
    Arboard,
    OscSequence,
    Unavailable,
}

#[derive(Debug)]
pub struct ClipboardService {
    backend: ClipboardBackend,
    last_copy: Option<CopyRecord>,
}

#[derive(Debug, Clone)]
pub struct CopyRecord {
    pub text: String,
    pub timestamp: Instant,
}

impl ClipboardService {
    #[must_use]
    pub fn new() -> Self {
        let backend = if cfg!(target_os = "linux") {
            if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
                ClipboardBackend::Arboard
            } else {
                ClipboardBackend::OscSequence
            }
        } else {
            ClipboardBackend::Arboard
        };

        Self {
            backend,
            last_copy: None,
        }
    }

    #[must_use]
    pub fn with_backend(backend: ClipboardBackend) -> Self {
        Self {
            backend,
            last_copy: None,
        }
    }

    pub fn copy(&mut self, text: &str) -> Result<(), ClipboardError> {
        match self.backend {
            ClipboardBackend::Arboard => self.copy_arboard(text),
            ClipboardBackend::OscSequence => self.copy_osc52(text),
            ClipboardBackend::Unavailable => Err(ClipboardError::Unavailable),
        }
    }

    fn copy_arboard(&mut self, text: &str) -> Result<(), ClipboardError> {
        // arboard integration deferred until the arboard crate feature is wired up;
        // for now just record the copy so callers work.
        let _ = text;
        self.record_copy(text);
        Ok(())
    }

    fn copy_osc52(&mut self, text: &str) -> Result<(), ClipboardError> {
        use std::io::Write;

        let encoded = base64_encode(text.as_bytes());
        let sequence = format!("\x1b]52;c;{encoded}\x07");
        std::io::stdout()
            .write_all(sequence.as_bytes())
            .map_err(|e| ClipboardError::Backend(e.to_string()))?;
        std::io::stdout()
            .flush()
            .map_err(|e| ClipboardError::Backend(e.to_string()))?;

        self.record_copy(text);
        Ok(())
    }

    fn record_copy(&mut self, text: &str) {
        self.last_copy = Some(CopyRecord {
            text: text.to_string(),
            timestamp: Instant::now(),
        });
    }

    #[must_use]
    pub fn last_copy(&self) -> Option<&CopyRecord> {
        self.last_copy.as_ref()
    }

    #[must_use]
    pub fn recently_copied(&self) -> bool {
        self.last_copy
            .as_ref()
            .is_some_and(|r| r.timestamp.elapsed() < Duration::from_secs(3))
    }

    #[must_use]
    pub fn backend(&self) -> &ClipboardBackend {
        &self.backend
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("clipboard unavailable")]
    Unavailable,
    #[error("clipboard backend error: {0}")]
    Backend(String),
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
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
    fn default_backend_not_unavailable() {
        let svc = ClipboardService::new();
        assert_ne!(*svc.backend(), ClipboardBackend::Unavailable);
    }

    #[test]
    fn unavailable_returns_error() {
        let mut svc = ClipboardService::with_backend(ClipboardBackend::Unavailable);
        assert!(svc.copy("hello").is_err());
    }

    #[test]
    fn last_copy_starts_none() {
        let svc = ClipboardService::new();
        assert!(svc.last_copy().is_none());
        assert!(!svc.recently_copied());
    }

    #[test]
    fn base64_encode_basic() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn osc52_copy_records() {
        let mut svc = ClipboardService::with_backend(ClipboardBackend::OscSequence);
        let _ = svc.copy("test");
        assert!(svc.last_copy().is_some());
        assert_eq!(svc.last_copy().unwrap().text, "test");
        assert!(svc.recently_copied());
    }
}
