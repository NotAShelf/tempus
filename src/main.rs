use clap::{Arg, Command};
use humantime::parse_duration;
use std::io::{self, Write, stdout};
use std::thread::sleep;
use std::time::{Duration, Instant};
use thiserror::Error;
use clap::Parser;

#[derive(Error, Debug)]
enum TempusError {
    #[error("Invalid duration format: {0}")]
    InvalidDuration(String),

    #[error("Timer interrupted")]
    TimerInterrupted,

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

type Result<T> = std::result::Result<T, TempusError>;

const PROGRESS_CHARS: [char; 4] = ['█', '▓', '▒', '░'];
const SPINNER_CHARS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Parser, Debug)]
#[command(name = "tempus", version = "1.0", about = "Enhanced timer with progress visualization")]
struct Args {
    /// Sleep duration (e.g. 5s, 2m, 1h30m)
    #[arg(value_name = "DURATION")]
    duration: String,

    /// Give this timer a name
    #[arg(short, long, default_value = "Timer")]
    name: String,

    /// Show more detailed output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let duration = parse_duration(&args.duration)
        .map_err(|_| TempusError::InvalidDuration(args.duration.clone()))?;

    run_timer(duration, &args.name, args.verbose)?;

    Ok(())
}

fn format_simple_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn run_timer(duration: Duration, name: &str, verbose: bool) -> Result<()> {
    let total_millis = duration.as_millis() as f64;
    let start_time = Instant::now();

    print!("\x1B[?25l");
    stdout().flush()?;

    struct CursorGuard;
    impl Drop for CursorGuard {
        fn drop(&mut self) {
            print!("\x1B[?25h");
            let _ = stdout().flush();
        }
    }
    let _cursor_guard = CursorGuard;

    ctrlc::set_handler(move || {
        print!("\r\x1B[K\x1B[?25h");
        println!("Timer interrupted.");
        std::process::exit(1);
    })
    .map_err(|_| TempusError::TimerInterrupted)?;

    let update_frequency = if duration.as_secs() > 3600 {
        Duration::from_millis(1000)
    } else if duration.as_secs() > 60 {
        Duration::from_millis(100)
    } else {
        Duration::from_millis(20)
    };

    let bar_width = 30;
    let mut spinner_idx = 0;

    while start_time.elapsed() < duration {
        let elapsed = start_time.elapsed();
        let elapsed_millis = elapsed.as_millis() as f64;

        let progress_ratio = elapsed_millis / total_millis;
        let percent = (progress_ratio * 100.0).min(100.0);
        let filled_width = (progress_ratio * bar_width as f64).floor() as usize;

        print!("\r\x1B[K");

        print!("{} ", SPINNER_CHARS[spinner_idx]);
        spinner_idx = (spinner_idx + 1) % SPINNER_CHARS.len();

        print!("[");
        for i in 0..bar_width {
            if i < filled_width {
                print!("{}", PROGRESS_CHARS[0]);
            } else if i == filled_width && progress_ratio < 1.0 {
                let partial_progress = progress_ratio * bar_width as f64 - filled_width as f64;
                let partial_idx = (partial_progress * PROGRESS_CHARS.len() as f64).floor() as usize;
                if partial_idx < PROGRESS_CHARS.len() {
                    print!("{}", PROGRESS_CHARS[partial_idx]);
                }
            } else {
                print!(" ");
            }
        }

        print!("] {:.1}%", percent);

        if verbose {
            let remaining = duration
                .checked_sub(elapsed)
                .unwrap_or(Duration::from_secs(0));
            print!(" ({}) {}", format_simple_duration(remaining), name);
        }

        stdout().flush()?;

        sleep(update_frequency);
    }

    let total_elapsed = start_time.elapsed();

    print!("\x07");

    print!("\r\x1B[K");

    println!(
        "{} completed! (took {})",
        name,
        format_simple_duration(total_elapsed)
    );

    Ok(())
}
