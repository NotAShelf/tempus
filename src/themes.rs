use crate::progress::ProgressBarTheme;
use std::str::FromStr;

/// Error type for theme parsing failures
#[derive(Debug, Clone, Copy)]
pub struct ThemeParseError;

impl std::fmt::Display for ThemeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid theme name")
    }
}

impl std::error::Error for ThemeParseError {}

impl FromStr for ProgressBarTheme {
    type Err = ThemeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "rainbow" => ProgressBarTheme::Rainbow,
            "plain" => ProgressBarTheme::Plain,
            "pulse" => ProgressBarTheme::Pulse,
            "gradient" => ProgressBarTheme::Gradient,
            "color" => ProgressBarTheme::Color,
            _ => return Err(ThemeParseError),
        })
    }
}

/// Parse a theme name into a ProgressBarTheme, defaulting to Gradient for unknown names.
pub fn parse_theme(theme_name: &str) -> ProgressBarTheme {
    ProgressBarTheme::from_str(theme_name).unwrap_or(ProgressBarTheme::Gradient)
}
