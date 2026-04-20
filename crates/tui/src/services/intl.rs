use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn format_relative_time(secs_ago: u64) -> String {
    match secs_ago {
        0..=59 => "just now".to_string(),
        60..=3599 => {
            let mins = secs_ago / 60;
            if mins == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{mins} minutes ago")
            }
        }
        3600..=86399 => {
            let hours = secs_ago / 3600;
            if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{hours} hours ago")
            }
        }
        86400..=2_591_999 => {
            let days = secs_ago / 86400;
            if days == 1 {
                "1 day ago".to_string()
            } else {
                format!("{days} days ago")
            }
        }
        _ => {
            let days = secs_ago / 86400;
            format!("{days} days ago")
        }
    }
}

pub fn format_timestamp(epoch_secs: u64) -> String {
    let secs_since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
        .saturating_sub(epoch_secs);
    format_relative_time(secs_since)
}

pub fn format_number(n: u64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    let remaining_secs = secs % 60;
    if mins < 60 {
        if remaining_secs == 0 {
            return format!("{mins}m");
        }
        return format!("{mins}m {remaining_secs}s");
    }
    let hours = mins / 60;
    let remaining_mins = mins % 60;
    if remaining_mins == 0 {
        return format!("{hours}h");
    }
    format!("{hours}h {remaining_mins}m")
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let kb = bytes as f64 / 1024.0;
    if kb < 1024.0 {
        return format!("{kb:.1} KB");
    }
    let mb = kb / 1024.0;
    if mb < 1024.0 {
        return format!("{mb:.1} MB");
    }
    let gb = mb / 1024.0;
    format!("{gb:.1} GB")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_just_now() {
        assert_eq!(format_relative_time(0), "just now");
        assert_eq!(format_relative_time(30), "just now");
    }

    #[test]
    fn relative_time_minutes() {
        assert_eq!(format_relative_time(60), "1 minute ago");
        assert_eq!(format_relative_time(300), "5 minutes ago");
    }

    #[test]
    fn relative_time_hours() {
        assert_eq!(format_relative_time(3600), "1 hour ago");
        assert_eq!(format_relative_time(7200), "2 hours ago");
    }

    #[test]
    fn relative_time_days() {
        assert_eq!(format_relative_time(86400), "1 day ago");
        assert_eq!(format_relative_time(172800), "2 days ago");
    }

    #[test]
    fn number_formatting() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1_000_000), "1,000,000");
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(3660), "1h 1m");
    }

    #[test]
    fn bytes_formatting() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    }
}
