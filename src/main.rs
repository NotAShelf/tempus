use clap::Parser;
use humantime::parse_duration;
use std::io::{self, Write, stdout};
use std::thread::sleep;
use std::time::{Duration, Instant};
use thiserror::Error;

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

const PROGRESS_CHARS: [char; 9] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█', ' '];
const SPINNER_CHARS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const LEFT_BRACKET: &str = "┃";
const RIGHT_BRACKET: &str = "┃";

const COLOR_RESET: &str = "\x1B[0m";
const COLOR_RED: &str = "\x1B[31m";
const COLOR_BRIGHT_RED: &str = "\x1B[91m";
const COLOR_GREEN: &str = "\x1B[32m";
const COLOR_YELLOW: &str = "\x1B[33m";
const COLOR_BLUE: &str = "\x1B[34m";
const COLOR_MAGENTA: &str = "\x1B[35m";
const COLOR_CYAN: &str = "\x1B[36m";
const COLOR_BRIGHT_CYAN: &str = "\x1B[96m";
const COLOR_BRIGHT_BLUE: &str = "\x1B[94m";
const COLOR_BRIGHT_GREEN: &str = "\x1B[92m";
const COLOR_BRIGHT_YELLOW: &str = "\x1B[93m";
const COLOR_BRIGHT_WHITE: &str = "\x1B[97m";

const STYLE_BOLD: &str = "\x1B[1m";

// Progress bar themes
#[derive(Debug, Clone, Copy)]
enum ProgressBarTheme {
    Gradient,
    Rainbow,
    Simple,
    Pulse,
}

#[derive(Parser, Debug)]
#[command(name = "tempus", version = "1.0", about = "Minimalist timer for your terminal")]
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

    /// Progress bar theme (gradient, rainbow, simple, pulse)
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

    let duration = parse_duration(&duration_str)
        .map_err(|_| TempusError::InvalidDuration(duration_str))?;

    // Parse theme from string
    let theme = match args.theme.to_lowercase().as_str() {
        "rainbow" => ProgressBarTheme::Rainbow,
        "simple" => ProgressBarTheme::Simple,
        "pulse" => ProgressBarTheme::Pulse,
        _ => ProgressBarTheme::Gradient,
    };

    run_timer(duration, &args.name, args.verbose, theme, args.bell, args.notify)?;

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

fn run_timer(duration: Duration, name: &str, verbose: bool, theme: ProgressBarTheme, bell: bool, notify: bool) -> Result<()> {
    let total_millis = duration.as_millis() as f64;
    let start_time = Instant::now();

    print!("\x1B[?25l"); // hide cursor
    stdout().flush()?;

    // Guard to restore cursor visibility when function exits
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

    let bar_width = 40;
    let mut spinner_idx = 0;
    let mut pulse_offset = 0.0;
    let pulse_speed = 0.2;

    while start_time.elapsed() < duration {
        let elapsed = start_time.elapsed();
        let elapsed_millis = elapsed.as_millis() as f64;

        let progress_ratio = elapsed_millis / total_millis;
        let percent = (progress_ratio * 100.0).min(100.0);

        print!("\r\x1B[K");

        let spinner_color = match theme {
            ProgressBarTheme::Rainbow => {
                let colors = [COLOR_RED, COLOR_YELLOW, COLOR_GREEN, COLOR_CYAN, COLOR_BLUE, COLOR_MAGENTA];
                colors[(spinner_idx / 2) % colors.len()]
            },
            ProgressBarTheme::Gradient => COLOR_CYAN,
            ProgressBarTheme::Simple => COLOR_RESET,
            ProgressBarTheme::Pulse => {
                let colors = [COLOR_CYAN, COLOR_BRIGHT_CYAN];
                colors[spinner_idx % colors.len()]
            },
        };

        print!("{}{}{} ", spinner_color, SPINNER_CHARS[spinner_idx], COLOR_RESET);
        spinner_idx = (spinner_idx + 1) % SPINNER_CHARS.len();

        match theme {
            ProgressBarTheme::Simple => print!("{}", LEFT_BRACKET),
            _ => print!("{}", LEFT_BRACKET),
        }

        // Draw the progress bar
        match theme {
            ProgressBarTheme::Gradient => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        let color = if position < 0.33 {
                            COLOR_GREEN
                        } else if position < 0.66 {
                            COLOR_YELLOW
                        } else {
                            COLOR_BRIGHT_RED
                        };

                        print!("{}{}{}", color, PROGRESS_CHARS[7], COLOR_RESET);
                    } else if i == (progress_ratio * bar_width as f64) as usize && progress_ratio < 1.0 {
                        let partial = (progress_ratio * bar_width as f64) - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}{}{}", COLOR_BRIGHT_GREEN, PROGRESS_CHARS[idx], COLOR_RESET);
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            },
            ProgressBarTheme::Rainbow => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        let color_idx = (i * 6 / bar_width) % 6;
                        let color = match color_idx {
                            0 => COLOR_RED,
                            1 => COLOR_YELLOW,
                            2 => COLOR_GREEN,
                            3 => COLOR_CYAN,
                            4 => COLOR_BLUE,
                            _ => COLOR_MAGENTA,
                        };

                        print!("{}{}{}", color, PROGRESS_CHARS[7], COLOR_RESET);
                    } else if i == (progress_ratio * bar_width as f64) as usize && progress_ratio < 1.0 {
                        let partial = (progress_ratio * bar_width as f64) - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}{}{}", COLOR_BRIGHT_WHITE, PROGRESS_CHARS[idx], COLOR_RESET);
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            },
            ProgressBarTheme::Simple => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        print!("{}", PROGRESS_CHARS[7]);
                    } else if i == (progress_ratio * bar_width as f64) as usize && progress_ratio < 1.0 {
                        let partial = (progress_ratio * bar_width as f64) - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}", PROGRESS_CHARS[idx]);
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            },
            ProgressBarTheme::Pulse => {
                pulse_offset += pulse_speed;
                if pulse_offset > 1.0 {
                    pulse_offset = 0.0;
                }

                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        let pulse_position = (position + pulse_offset) % 1.0;
                        let brightness = (pulse_position * 3.14159).sin().abs();

                        let color = if brightness > 0.7 {
                            COLOR_BRIGHT_CYAN
                        } else if brightness > 0.3 {
                            COLOR_CYAN
                        } else {
                            COLOR_BLUE
                        };

                        print!("{}{}{}", color, PROGRESS_CHARS[7], COLOR_RESET);
                    } else if i == (progress_ratio * bar_width as f64) as usize && progress_ratio < 1.0 {
                        let partial = (progress_ratio * bar_width as f64) - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}{}{}", COLOR_BRIGHT_BLUE, PROGRESS_CHARS[idx], COLOR_RESET);
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            },
        }

        // Bar end decoration
        match theme {
            ProgressBarTheme::Simple => print!("{}", RIGHT_BRACKET),
            _ => print!("{}", RIGHT_BRACKET),
        }

        // Percentage display
        let percent_color = match theme {
            ProgressBarTheme::Simple => COLOR_RESET,
            ProgressBarTheme::Gradient => {
                if percent < 33.0 {
                    COLOR_GREEN
                } else if percent < 66.0 {
                    COLOR_YELLOW
                } else {
                    COLOR_BRIGHT_YELLOW
                }
            },
            ProgressBarTheme::Rainbow => COLOR_BRIGHT_WHITE,
            ProgressBarTheme::Pulse => COLOR_BRIGHT_CYAN,
        };

        print!(" {}{}{:.1}%{}", STYLE_BOLD, percent_color, percent, COLOR_RESET);

        // Additional information (if verbose)
        if verbose {
            let remaining = duration
                .checked_sub(elapsed)
                .unwrap_or(Duration::from_secs(0));

            let time_color = match theme {
                ProgressBarTheme::Simple => COLOR_RESET,
                _ => COLOR_BRIGHT_WHITE,
            };

            print!(
                " {}({}){} {}{}{}",
                time_color,
                format_simple_duration(remaining),
                COLOR_RESET,
                STYLE_BOLD,
                name,
                COLOR_RESET
            );
        }

        stdout().flush()?;
        sleep(update_frequency);
    }

    // Timer complete
    let total_elapsed = start_time.elapsed();

    // Play bell sound if enabled
    if bell {
        print!("\x07"); // bell character
    }

    print!("\r\x1B[K");

    // Completion message
    let complete_color = match theme {
        ProgressBarTheme::Simple => COLOR_RESET,
        ProgressBarTheme::Gradient => COLOR_BRIGHT_GREEN,
        ProgressBarTheme::Rainbow => COLOR_BRIGHT_CYAN,
        ProgressBarTheme::Pulse => COLOR_BRIGHT_CYAN,
    };

    println!(
        "{}{}{}{} completed!{} (took {})",
        STYLE_BOLD,
        complete_color,
        name,
        COLOR_RESET,
        COLOR_RESET,
        format_simple_duration(total_elapsed)
    );

    // Send desktop notification if enabled
    if notify {
        send_notification(name, total_elapsed)?;
    }

    Ok(())
}


fn send_notification(name: &str, duration: Duration) -> Result<()> {
    if cfg!(target_os = "linux") {
        let _ = std::process::Command::new("notify-send")
            .args([&format!("{} completed!", name), &format!("Duration: {}", format_simple_duration(duration))])
            .spawn();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("osascript")
            .args(["-e", &format!("display notification \"Duration: {}\" with title \"{}\"",
                   format_simple_duration(duration), format!("{} completed!", name))])
            .spawn();
    } else if cfg!(target_os = "windows") {
        let script = format!(
            // Thank you Sky for the PS script. I wouldn't care about it otherwise.
            "powershell -Command \"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); $toastXml = [xml] $template.GetXml(); $toastXml.GetElementsByTagName('text')[0].AppendChild($toastXml.CreateTextNode('{} completed!')) > $null; $toastXml.GetElementsByTagName('text')[1].AppendChild($toastXml.CreateTextNode('Duration: {}')) > $null; $toast = [Windows.UI.Notifications.ToastNotification]::new($toastXml); [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Tempus').Show($toast);\"",
            name,
            format_simple_duration(duration)
        );
        let _ = std::process::Command::new("cmd")
            .args(["/C", &script])
            .spawn();
    }
    Ok(())
}