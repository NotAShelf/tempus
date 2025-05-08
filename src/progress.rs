use crate::Result;
use crate::utils::{format_simple_duration, send_notification, should_use_color};
use std::io::{Write, stdout};
use std::thread::sleep;
use std::time::{Duration, Instant};
use yansi::{Color, Paint};

#[derive(Debug, Clone, Copy)]
pub enum ProgressBarTheme {
    Gradient,
    Rainbow,
    Plain,
    Pulse,
}

const PROGRESS_CHARS: [char; 9] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█', ' '];
const SPINNER_CHARS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const LEFT_BRACKET: &str = "┃";
const RIGHT_BRACKET: &str = "┃";

pub fn run_timer(
    duration: Duration,
    name: &str,
    verbose: bool,
    mut theme: ProgressBarTheme,
    bell: bool,
    notify: bool,
) -> Result<()> {
    // If NO_COLOR environment variable is set, override theme to Plain
    if !should_use_color() {
        theme = ProgressBarTheme::Plain;
        yansi::disable();
    } else {
        yansi::enable();
    }

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
    })?;

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

        let spinner_paint = match theme {
            ProgressBarTheme::Rainbow => {
                let colors = [
                    Color::Red,
                    Color::Yellow,
                    Color::Green,
                    Color::Cyan,
                    Color::Blue,
                    Color::Magenta,
                ];
                Paint::new(SPINNER_CHARS[spinner_idx]).fg(colors[(spinner_idx / 2) % colors.len()])
            }
            ProgressBarTheme::Gradient => Paint::new(SPINNER_CHARS[spinner_idx]).fg(Color::Cyan),
            ProgressBarTheme::Plain => Paint::new(SPINNER_CHARS[spinner_idx]),
            ProgressBarTheme::Pulse => {
                let colors = [Color::Cyan, Color::BrightCyan];
                Paint::new(SPINNER_CHARS[spinner_idx]).fg(colors[spinner_idx % colors.len()])
            }
        };
        print!("{} ", spinner_paint);
        spinner_idx = (spinner_idx + 1) % SPINNER_CHARS.len();

        print!("{}", LEFT_BRACKET);

        match theme {
            ProgressBarTheme::Gradient => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        let color = if position < 0.33 {
                            Color::Green
                        } else if position < 0.66 {
                            Color::Yellow
                        } else {
                            Color::BrightRed
                        };

                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(color));
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}", Paint::new(PROGRESS_CHARS[idx]).fg(Color::BrightGreen));
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            }
            ProgressBarTheme::Rainbow => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        let color_idx = (i * 6 / bar_width) % 6;
                        let color = match color_idx {
                            0 => Color::Red,
                            1 => Color::Yellow,
                            2 => Color::Green,
                            3 => Color::Cyan,
                            4 => Color::Blue,
                            _ => Color::Magenta,
                        };

                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(color));
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}", Paint::new(PROGRESS_CHARS[idx]).fg(Color::BrightWhite));
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            }
            ProgressBarTheme::Plain => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        print!("{}", PROGRESS_CHARS[7]);
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}", PROGRESS_CHARS[idx]);
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            }
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
                            Color::BrightCyan
                        } else if brightness > 0.3 {
                            Color::Cyan
                        } else {
                            Color::Blue
                        };

                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(color));
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!("{}", Paint::new(PROGRESS_CHARS[idx]).fg(Color::BrightBlue));
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            }
        }

        print!("{}", RIGHT_BRACKET);

        let percent_color = match theme {
            ProgressBarTheme::Plain => None,
            ProgressBarTheme::Gradient => {
                if percent < 33.0 {
                    Some(Color::Green)
                } else if percent < 66.0 {
                    Some(Color::Yellow)
                } else {
                    Some(Color::BrightRed)
                }
            }
            ProgressBarTheme::Rainbow => Some(Color::BrightWhite),
            ProgressBarTheme::Pulse => Some(Color::BrightCyan),
        };
        let percent_str = format!("{:.1}%", percent);
        let percent_paint = match percent_color {
            Some(c) => Paint::new(percent_str).bold().fg(c),
            None => Paint::new(percent_str).bold(),
        };
        print!(" {}", percent_paint);

        if verbose {
            let remaining = duration
                .checked_sub(elapsed)
                .unwrap_or(Duration::from_secs(0));
            let time_color = match theme {
                ProgressBarTheme::Plain => None,
                _ => Some(Color::BrightWhite),
            };
            let time_str = format!("({})", format_simple_duration(remaining));
            let time_paint = match time_color {
                Some(c) => Paint::new(time_str).fg(c),
                None => Paint::new(time_str),
            };
            print!(" {}{}", time_paint, Paint::new(name).bold());
        }

        stdout().flush()?;
        sleep(update_frequency);
    }

    let total_elapsed = start_time.elapsed();

    if bell {
        print!("\x07");
    }

    print!("\r\x1B[K");

    let complete_color = match theme {
        ProgressBarTheme::Plain => None,
        ProgressBarTheme::Gradient => Some(Color::BrightGreen),
        ProgressBarTheme::Rainbow => Some(Color::BrightCyan),
        ProgressBarTheme::Pulse => Some(Color::BrightCyan),
    };
    let complete_paint = match complete_color {
        Some(c) => Paint::new(name).bold().fg(c),
        None => Paint::new(name).bold(),
    };
    println!(
        "{} completed! (took {})",
        complete_paint,
        format_simple_duration(total_elapsed)
    );

    if notify {
        send_notification(name, total_elapsed)?;
    }

    Ok(())
}
