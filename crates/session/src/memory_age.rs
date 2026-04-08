//! Memory aging: decay relevance over time, suggest pruning.
//!
//! Memories lose relevance as they age. This module computes an age-based
//! score and identifies memories that should be pruned to keep the memory
//! store lean and useful.

use super::memory_relevance::MemoryEntry;

/// Default pruning threshold — memories scoring below this are candidates.
const DEFAULT_PRUNE_THRESHOLD: f64 = 0.1;

/// Half-life in days for exponential decay. A memory is scored 0.5 after
/// this many days, ~0.25 after double, etc.
const HALF_LIFE_DAYS: f64 = 30.0;

// ── Scoring ───────────────────────────────────────────────────────────

/// Compute an age-decay score for a memory entry.
///
/// Both timestamps should be ISO 8601 date strings (at minimum `YYYY-MM-DD`).
/// The score is in `[0.0, 1.0]` where 1.0 means the memory was just created
/// and approaches 0.0 as it ages.
///
/// Uses exponential decay with a 30-day half-life:
/// `score = 0.5^(age_days / 30)`
pub fn age_score(created_at: &str, now: &str) -> f64 {
    let Some(created_days) = parse_date_to_days(created_at) else {
        return 0.5; // unparseable → neutral score
    };
    let Some(now_days) = parse_date_to_days(now) else {
        return 0.5;
    };

    let age_days = (now_days - created_days).max(0) as f64;
    // Exponential decay: 0.5^(age / half_life)
    0.5_f64.powf(age_days / HALF_LIFE_DAYS)
}

/// Compute the age of a memory in days, returning a human-readable string.
///
/// Maps to CCB `memoryAge.ts`.
pub fn memory_age_text(age_days: u64) -> String {
    match age_days {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        n => format!("{n} days ago"),
    }
}

/// Return a staleness caveat for memories older than 1 day, or empty string
/// for fresh memories.
pub fn memory_freshness_text(age_days: u64) -> String {
    if age_days <= 1 {
        String::new()
    } else {
        format!(
            "This memory is {}. Memories are point-in-time observations. \
             Verify against current code before asserting as fact.",
            memory_age_text(age_days)
        )
    }
}

// ── Pruning ───────────────────────────────────────────────────────────

/// Determine whether a memory with the given age score should be pruned.
///
/// Returns `true` when the score falls below the given threshold.
#[must_use]
pub fn should_prune(score: f64, threshold: f64) -> bool {
    score < threshold
}

/// Suggest memory entries that should be pruned based on their age.
///
/// Returns the file paths (as strings) of memories whose age score falls
/// below the default threshold (0.1, roughly 100 days old).
#[must_use]
pub fn suggest_pruning(memories: &[MemoryEntry]) -> Vec<String> {
    let now = current_date_string();

    memories
        .iter()
        .filter(|entry| {
            let created = entry.metadata.created_at.as_deref().unwrap_or("2000-01-01");
            let score = age_score(created, &now);
            should_prune(score, DEFAULT_PRUNE_THRESHOLD)
        })
        .map(|entry| entry.path.display().to_string())
        .collect()
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Parse an ISO 8601 date string to days since epoch (approximate).
///
/// Accepts formats: `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM:SS...`
fn parse_date_to_days(date_str: &str) -> Option<i64> {
    // Extract just the date part (first 10 chars: YYYY-MM-DD)
    let date_part = if date_str.len() >= 10 {
        &date_str[..10]
    } else {
        date_str
    };

    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let year: i64 = parts[0].parse().ok()?;
    let month: i64 = parts[1].parse().ok()?;
    let day: i64 = parts[2].parse().ok()?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Approximate days since epoch using a simple formula
    // (good enough for relative age calculation)
    Some(year * 365 + year / 4 - year / 100 + year / 400 + month * 30 + day)
}

/// Get current date as `YYYY-MM-DD` string.
fn current_date_string() -> String {
    // Use system time to get current date
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_days = now.as_secs() / 86400;

    // Convert days since epoch to YYYY-MM-DD
    // Simple civil date calculation
    let (y, m, d) = days_to_civil(total_days.cast_signed() + 719_468);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Convert days since epoch 0000-03-01 to (year, month, day).
/// Algorithm from Howard Hinnant.
fn days_to_civil(days: i64) -> (i64, u32, u32) {
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = i64::from(yoe) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_prune_below_threshold() {
        assert!(should_prune(0.1, 0.2));
    }

    #[test]
    fn should_not_prune_above_threshold() {
        assert!(!should_prune(0.5, 0.2));
    }

    #[test]
    fn should_not_prune_at_threshold() {
        assert!(!should_prune(0.2, 0.2));
    }

    #[test]
    fn age_score_same_day_is_one() {
        let score = age_score("2025-06-15", "2025-06-15");
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn age_score_30_days_is_half() {
        let score = age_score("2025-05-16", "2025-06-15");
        assert!((score - 0.5).abs() < 0.05);
    }

    #[test]
    fn age_score_60_days_is_quarter() {
        let score = age_score("2025-04-16", "2025-06-15");
        assert!((score - 0.25).abs() < 0.05);
    }

    #[test]
    fn age_score_unparseable_returns_neutral() {
        let score = age_score("not-a-date", "2025-06-15");
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn age_score_future_date_clamps_to_one() {
        let score = age_score("2025-06-20", "2025-06-15");
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn memory_age_text_today() {
        assert_eq!(memory_age_text(0), "today");
    }

    #[test]
    fn memory_age_text_yesterday() {
        assert_eq!(memory_age_text(1), "yesterday");
    }

    #[test]
    fn memory_age_text_multiple_days() {
        assert_eq!(memory_age_text(47), "47 days ago");
    }

    #[test]
    fn freshness_text_fresh() {
        assert!(memory_freshness_text(0).is_empty());
        assert!(memory_freshness_text(1).is_empty());
    }

    #[test]
    fn freshness_text_stale() {
        let text = memory_freshness_text(5);
        assert!(text.contains("5 days ago"));
        assert!(text.contains("Verify"));
    }

    #[test]
    fn parse_date_to_days_valid() {
        assert!(parse_date_to_days("2025-06-15").is_some());
        assert!(parse_date_to_days("2025-06-15T12:00:00Z").is_some());
    }

    #[test]
    fn parse_date_to_days_invalid() {
        assert!(parse_date_to_days("not-a-date").is_none());
        assert!(parse_date_to_days("2025-13-01").is_none());
    }

    #[test]
    fn suggest_pruning_empty_input() {
        let result = suggest_pruning(&[]);
        assert!(result.is_empty());
    }
}
