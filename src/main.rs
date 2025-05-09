mod focus_mode;
mod progress;
mod themes;
mod utils;

use chrono::{DateTime, Local, TimeZone};
use clap::{Parser, Subcommand};
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

#[derive(Subcommand, Debug)]
enum Command {
    /// Start a countdown to a specific date/time (e.g. "2025-12-31 23:59:59")
    Countdown {
        /// Target date/time (e.g. "2025-12-31 23:59:59", "20:00", etc.)
        #[arg(value_name = "DATETIME")]
        datetime: String,
        /// Name for the countdown event
        #[arg(short, long, default_value = "Countdown")]
        name: String,
        /// Progress bar theme
        #[arg(short, long, default_value = "gradient")]
        theme: String,
        /// Play bell sound when countdown completes
        #[arg(short = 'b', long, default_value_t = true)]
        bell: bool,
        /// Send a desktop notification when countdown completes
        #[arg(short = 'N', long, default_value_t = false)]
        notify: bool,
        /// Show big ASCII art clock mode
        #[arg(long, default_value_t = false)]
        big: bool,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "tempus",
    version = "0.3.0",
    about = "Minimalist timer for your terminal"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Sleep duration (e.g. 5s, 2m, 1h30m)
    #[arg(value_name = "DURATION")]
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

    /// Show big ASCII art clock mode
    #[arg(long, default_value_t = false)]
    big: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match &args.command {
        Some(Command::Countdown {
            datetime,
            name,
            theme,
            bell,
            notify,
            big,
        }) => {
            let target = DateTime::parse_from_rfc3339(datetime)
                .map(|dt| dt.with_timezone(&Local))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| Local.from_local_datetime(&ndt).unwrap())
                })
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M")
                        .map(|ndt| Local.from_local_datetime(&ndt).unwrap())
                })
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(datetime, "%Y-%m-%d")
                        .map(|nd| nd.and_hms_opt(0, 0, 0).unwrap())
                        .map(|ndt| Local.from_local_datetime(&ndt).unwrap())
                })
                .or_else(|_| {
                    chrono::NaiveTime::parse_from_str(datetime, "%H:%M:%S").map(|nt| {
                        let today = Local::now().date_naive();
                        let ndt = today.and_time(nt);
                        Local.from_local_datetime(&ndt).unwrap()
                    })
                })
                .or_else(|_| {
                    chrono::NaiveTime::parse_from_str(datetime, "%H:%M").map(|nt| {
                        let today = Local::now().date_naive();
                        let ndt = today.and_time(nt);
                        Local.from_local_datetime(&ndt).unwrap()
                    })
                });
            let target = match target {
                Ok(dt) => dt,
                Err(_) => {
                    eprintln!("Could not parse date/time: {}", datetime);
                    std::process::exit(1);
                }
            };
            let now = Local::now();
            if target <= now {
                eprintln!("Target date/time is in the past!");
                std::process::exit(1);
            }
            let duration = (target - now).to_std().unwrap();
            let theme = parse_theme(theme);
            if *big {
                return progress::run_big_clock(duration, name, *bell)
                    .map_err(TempusError::IoError);
            }
            run_timer(duration, name, false, theme, *bell, *notify)?;
            return Ok(());
        }
        _ => {
            if args.duration.is_none() && args.preset.is_none() {
                eprintln!(
                    "Error: Either DURATION or --preset must be provided when not using a subcommand"
                );
                std::process::exit(1);
            }

            let duration_str = match args.preset.as_deref() {
                Some("pomodoro") => "25m".to_string(),
                Some("short-break") => "5m".to_string(),
                Some("long-break") => "15m".to_string(),
                Some("tea") => "3m".to_string(),
                Some("coffee") => "4m".to_string(),
                Some(custom) => custom.to_string(),
                None => args.duration.clone().unwrap_or_default(),
            };

            let duration = parse_duration(&duration_str)
                .map_err(|_| TempusError::InvalidDuration(duration_str))?;

            let theme = parse_theme(&args.theme);

            if args.big {
                return progress::run_big_clock(duration, &args.name, args.bell)
                    .map_err(TempusError::IoError);
            }

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
    }
}
