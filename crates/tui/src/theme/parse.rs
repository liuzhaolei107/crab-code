//! Color string parser used by `ThemeConfig` to deserialize JSON colors.

use ratatui::style::Color;

/// Parse a color from various string formats.
/// Supports: named ("red"), hex ("#ff0000", "#f00"), rgb ("rgb(255,0,0)"),
/// 256-color ("256:42").
pub(super) fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();
    match s.as_str() {
        "black" => return Some(Color::Black),
        "red" => return Some(Color::Red),
        "green" => return Some(Color::Green),
        "yellow" => return Some(Color::Yellow),
        "blue" => return Some(Color::Blue),
        "magenta" => return Some(Color::Magenta),
        "cyan" => return Some(Color::Cyan),
        "gray" | "grey" => return Some(Color::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => return Some(Color::DarkGray),
        "lightred" | "light_red" => return Some(Color::LightRed),
        "lightgreen" | "light_green" => return Some(Color::LightGreen),
        "lightyellow" | "light_yellow" => return Some(Color::LightYellow),
        "lightblue" | "light_blue" => return Some(Color::LightBlue),
        "lightmagenta" | "light_magenta" => return Some(Color::LightMagenta),
        "lightcyan" | "light_cyan" => return Some(Color::LightCyan),
        "white" => return Some(Color::White),
        "reset" | "default" => return Some(Color::Reset),
        _ => {}
    }

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color::Rgb(r, g, b));
        }
    }

    if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r: u8 = parts[0].trim().parse().ok()?;
            let g: u8 = parts[1].trim().parse().ok()?;
            let b: u8 = parts[2].trim().parse().ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }

    if let Some(idx) = s.strip_prefix("256:") {
        let n: u8 = idx.trim().parse().ok()?;
        return Some(Color::Indexed(n));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_colors() {
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("Blue"), Some(Color::Blue));
        assert_eq!(parse_color("DARKGRAY"), Some(Color::DarkGray));
        assert_eq!(parse_color("dark_gray"), Some(Color::DarkGray));
        assert_eq!(parse_color("light_red"), Some(Color::LightRed));
        assert_eq!(parse_color("reset"), Some(Color::Reset));
        assert_eq!(parse_color("default"), Some(Color::Reset));
    }

    #[test]
    fn parse_hex_colors() {
        assert_eq!(parse_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("#0000FF"), Some(Color::Rgb(0, 0, 255)));
        assert_eq!(parse_color("#f00"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#abc"), Some(Color::Rgb(170, 187, 204)));
    }

    #[test]
    fn parse_rgb_colors() {
        assert_eq!(parse_color("rgb(255, 0, 0)"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(
            parse_color("rgb(128,128,128)"),
            Some(Color::Rgb(128, 128, 128))
        );
    }

    #[test]
    fn parse_indexed_colors() {
        assert_eq!(parse_color("256:42"), Some(Color::Indexed(42)));
        assert_eq!(parse_color("256:0"), Some(Color::Indexed(0)));
    }

    #[test]
    fn parse_invalid_colors() {
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color("#gggggg"), None);
        assert_eq!(parse_color("rgb(300, 0, 0)"), None);
    }
}
