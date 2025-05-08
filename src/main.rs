mod focus_mode;
mod progress;
mod themes;
mod utils;

use clap::Parser;
use humantime::parse_duration;
use progress::{ProgressBarTheme, run_timer};
use std::io;
use themes::parse_theme;
use thiserror::Error;

// Error handling
#[derive(Error, Debug)]
enum TempusError {
    #[error("Invalid duration format: {0}")]
    InvalidDuration(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Ctrl-C error: {0}")]
    CtrlcError(#[from] ctrlc::Error),
}

type Result<T> = std::result::Result<T, TempusError>;

#[derive(Parser, Debug)]
#[command(
    name = "tempus",
    version = "1.0",
    about = "Minimalist timer for your terminal"
)]
struct Args {
    /// Sleep duration (e.g. 5s, 2m, 1h30m)
    #[arg(value_name = "DURATION", required_unless_present = "preset")]
    duration: Option<String>,

    /// Give this timer a name
    #[arg(short, long, default_value = "Timer")]
    name: String,

    /// Show more detailed output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Progress bar theme (gradient, rainbow, plain, pulse)
    #[arg(short, long, default_value = "gradient")]
    theme: String,

    /// Use a preset duration (pomodoro, short-break, long-break, tea, coffee)
    #[arg(short = 'p', long)]
    preset: Option<String>,

    /// Play bell sound when timer completes
    #[arg(short = 'b', long, default_value_t = true)]
    bell: bool,

    /// Send a desktop notification when timer completes
    #[arg(short = 'N', long, default_value_t = false)]
    notify: bool,

    /// Enable focus mode with full-screen TUI
    #[arg(short = 'f', long, default_value_t = false)]
    focus: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let duration_str = match args.preset.as_deref() {
        Some("pomodoro") => "25m".to_string(),
        Some("short-break") => "5m".to_string(),
        Some("long-break") => "15m".to_string(),
        Some("tea") => "3m".to_string(),
        Some("coffee") => "4m".to_string(),
        Some(custom) => custom.to_string(),
        None => args.duration.clone().unwrap_or_default(),
    };

    let duration =
        parse_duration(&duration_str).map_err(|_| TempusError::InvalidDuration(duration_str))?;

    let theme = parse_theme(&args.theme);

    if args.focus {
        focus_mode::run_focus_mode(duration, &args.name, theme, args.bell, args.notify)?;
    } else {
        run_timer(
            duration,
            &args.name,
            args.verbose,
            theme,
            args.bell,
            args.notify,
        )?;
    }

    Ok(())
}
