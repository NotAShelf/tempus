use crate::progress::ProgressBarTheme;

pub fn parse_theme(theme_name: &str) -> ProgressBarTheme {
    match theme_name.to_lowercase().as_str() {
        "rainbow" => ProgressBarTheme::Rainbow,
        "simple" => ProgressBarTheme::Simple,
        "pulse" => ProgressBarTheme::Pulse,
        _ => ProgressBarTheme::Gradient,
    }
}