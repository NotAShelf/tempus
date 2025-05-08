use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::Result;
use crate::utils::{format_simple_duration, send_notification};

#[derive(Debug, Clone, Copy)]
pub enum ProgressBarTheme {
    Gradient,
    Rainbow,
    Simple,
    Pulse,
}

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
const COLOR_BRIGHT_WHITE: &str = "\x1B[97m";

const STYLE_BOLD: &str = "\x1B[1m";

pub fn run_timer(duration: Duration, name: &str, verbose: bool, theme: ProgressBarTheme, bell: bool, notify: bool) -> Result<()> {
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

        match theme {
            ProgressBarTheme::Simple => print!("{}", RIGHT_BRACKET),
            _ => print!("{}", RIGHT_BRACKET),
        }

        let percent_color = match theme {
            ProgressBarTheme::Simple => COLOR_RESET,
            ProgressBarTheme::Gradient => {
                if percent < 33.0 {
                    COLOR_GREEN
                } else if percent < 66.0 {
                    COLOR_YELLOW
                } else {
                    COLOR_BRIGHT_RED
                }
            },
            ProgressBarTheme::Rainbow => COLOR_BRIGHT_WHITE,
            ProgressBarTheme::Pulse => COLOR_BRIGHT_CYAN,
        };

        print!(" {}{}{:.1}%{}", STYLE_BOLD, percent_color, percent, COLOR_RESET);

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

    let total_elapsed = start_time.elapsed();

    if bell {
        print!("\x07");
    }

    print!("\r\x1B[K");

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

    if notify {
        send_notification(name, total_elapsed)?;
    }

    Ok(())
}