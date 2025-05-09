mod focus_mode;
mod progress;
mod themes;
mod utils;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use clap::{Parser, Subcommand};
use humantime::parse_duration;
use progress::{run_timer, ProgressBarTheme};
use std::{io, process};
use themes::parse_theme;
use thiserror::Error;

#[derive(Error, Debug)]
enum TempusError {
    #[error("Invalid duration format: {0}")]
    InvalidDuration(String),

    #[error("Invalid date/time format: {0}")]
    InvalidDateTime(String),

    #[error("Target date/time is in the past")]
    PastDateTime,

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

fn parse_datetime(datetime: &str) -> Result<DateTime<Local>> {
    DateTime::parse_from_rfc3339(datetime)
        .map(|dt| dt.with_timezone(&Local))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| Local.from_local_datetime(&ndt).single().unwrap())
        })
        .or_else(|_| {
            NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M")
                .map(|ndt| Local.from_local_datetime(&ndt).single().unwrap())
        })
        .or_else(|_| {
            NaiveDate::parse_from_str(datetime, "%Y-%m-%d")
                .map(|nd| nd.and_hms_opt(0, 0, 0).unwrap())
                .map(|ndt| Local.from_local_datetime(&ndt).single().unwrap())
        })
        .or_else(|_| {
            NaiveTime::parse_from_str(datetime, "%H:%M:%S").map(|nt| {
                let now = Local::now();
                let today = now.date_naive();
                let ndt = today.and_time(nt);
                let dt = Local.from_local_datetime(&ndt).single().unwrap();
                
                // If the time is in the past, set it for tomorrow
                if dt <= now {
                    let tomorrow = today.succ_opt().unwrap();
                    let ndt_tomorrow = tomorrow.and_time(nt);
                    Local.from_local_datetime(&ndt_tomorrow).single().unwrap()
                } else {
                    dt
                }
            })
        })
        .or_else(|_| {
            NaiveTime::parse_from_str(datetime, "%H:%M").map(|nt| {
                let now = Local::now();
                let today = now.date_naive();
                let ndt = today.and_time(nt);
                let dt = Local.from_local_datetime(&ndt).single().unwrap();
                
                // If the time is in the past, set it for tomorrow
                if dt <= now {
                    let tomorrow = today.succ_opt().unwrap();
                    let ndt_tomorrow = tomorrow.and_time(nt);
                    Local.from_local_datetime(&ndt_tomorrow).single().unwrap()
                } else {
                    dt
                }
            })
        })
        .map_err(|_| TempusError::InvalidDateTime(datetime.to_string()))
}

fn get_duration_from_preset(preset: &str) -> String {
    match preset {
        "pomodoro" => "25m".to_string(),
        "short-break" => "5m".to_string(),
        "long-break" => "15m".to_string(),
        "tea" => "3m".to_string(),
        "coffee" => "4m".to_string(),
        custom => custom.to_string(),
    }
}

fn handle_countdown(cmd: &Command) -> Result<()> {
    let Command::Countdown { datetime, name, theme, bell, notify, big } = cmd;
    
    let target = parse_datetime(datetime)?;
    let now = Local::now();
    
    let duration = (target - now).to_std().expect("Duration should be positive");
    let theme_enum = parse_theme(theme);
    
    if *big {
        return progress::run_big_clock(duration, name, *bell)
            .map_err(TempusError::IoError);
    }
    
    run_timer(duration, name, false, theme_enum, *bell, *notify)
}

fn handle_timer(args: &Args) -> Result<()> {
    let duration_str = match &args.preset {
        Some(preset) => get_duration_from_preset(preset),
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

fn main() -> Result<()> {
    let args = Args::parse();
    
    match &args.command {
        Some(cmd) => handle_countdown(cmd),
        None => {
            if args.duration.is_none() && args.preset.is_none() {
                eprintln!("Error: Either DURATION or --preset must be provided when not using a subcommand");
                process::exit(1);
            }
            
            handle_timer(&args)
        }
    }
}
