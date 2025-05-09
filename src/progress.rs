use crate::Result;
use crate::focus_mode::render_big_time;
use crate::utils::{format_simple_duration, send_notification, should_use_color};
use chrono::{DateTime, Local};
use colorgrad;
use colorgrad::Gradient;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
};
use std::f64::consts::PI;
use std::io::Write;
use std::io::stdout;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use yansi::{Color as YansiColor, Paint};

#[derive(Debug, Clone, Copy)]
pub enum ProgressBarTheme {
    Gradient,
    Rainbow,
    Plain,
    Pulse,
    Color,
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
    use_12h: bool,
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
    let start_system_time = SystemTime::now();
    let start_datetime: DateTime<Local> = start_system_time.into();
    let start_time_str = if use_12h {
        start_datetime.format("%I:%M:%S %p").to_string()
    } else {
        start_datetime.format("%H:%M:%S").to_string()
    };

    print!("\x1B[?25l"); // hide cursor
    stdout().flush()?;

    // This will be updated in-place to show the progress bar
    println!("");

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

        print!("\x1B[1A\r\x1B[K"); // move cursor up one line

        // Display the header with start time, name, and remaining time
        let remaining = duration
            .checked_sub(elapsed)
            .unwrap_or(Duration::from_secs(0));

        let header_color = match theme {
            ProgressBarTheme::Plain => None,
            _ => Some(YansiColor::BrightWhite),
        };

        let start_time_paint = match header_color {
            Some(c) => Paint::new(&start_time_str).fg(c),
            None => Paint::new(&start_time_str),
        };

        let name_paint = match header_color {
            Some(c) => Paint::new(name).bold().fg(c),
            None => Paint::new(name).bold(),
        };

        let remaining_str = format_simple_duration(remaining);
        let remaining_paint = match header_color {
            Some(c) => Paint::new(&remaining_str).fg(c),
            None => Paint::new(&remaining_str),
        };

        print!(
            "{} | {} | {} remaining",
            start_time_paint, name_paint, remaining_paint
        );

        print!("\n\r\x1B[K"); // move cursor down one line and clear it

        let spinner_paint = match theme {
            ProgressBarTheme::Rainbow => {
                let colors = [
                    YansiColor::Red,
                    YansiColor::Yellow,
                    YansiColor::Green,
                    YansiColor::Cyan,
                    YansiColor::Blue,
                    YansiColor::Magenta,
                ];
                Paint::new(SPINNER_CHARS[spinner_idx]).fg(colors[(spinner_idx / 2) % colors.len()])
            }
            ProgressBarTheme::Gradient => {
                Paint::new(SPINNER_CHARS[spinner_idx]).fg(YansiColor::Cyan)
            }
            ProgressBarTheme::Color => Paint::new(SPINNER_CHARS[spinner_idx]).fg(YansiColor::Cyan),
            ProgressBarTheme::Plain => Paint::new(SPINNER_CHARS[spinner_idx]),
            ProgressBarTheme::Pulse => {
                let colors = [YansiColor::Cyan, YansiColor::BrightCyan];
                Paint::new(SPINNER_CHARS[spinner_idx]).fg(colors[spinner_idx % colors.len()])
            }
        };
        print!("{} ", spinner_paint);
        spinner_idx = (spinner_idx + 1) % SPINNER_CHARS.len();

        print!("{}", LEFT_BRACKET);

        match theme {
            ProgressBarTheme::Gradient => {
                let gradient: colorgrad::LinearGradient = colorgrad::GradientBuilder::new()
                    .colors(&[
                        colorgrad::Color::new(0.0, 1.0, 0.0, 1.0), // Green
                        colorgrad::Color::new(1.0, 1.0, 0.0, 1.0), // Yellow
                        colorgrad::Color::new(1.0, 0.0, 0.0, 1.0), // Red
                    ])
                    .build()
                    .unwrap();
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;
                    if position < progress_ratio {
                        let rel_pos = position / progress_ratio.max(0.01);
                        let color = gradient.at(rel_pos as f32).to_rgba8();
                        let yansi_color = YansiColor::Rgb(color[0], color[1], color[2]);
                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(yansi_color));
                    } else if i == (progress_ratio * bar_width as f64) as usize && progress_ratio < 1.0 {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        let color = gradient.at(0.0).to_rgba8();
                        let yansi_color = YansiColor::Rgb(color[0], color[1], color[2]);
                        print!("{}", Paint::new(PROGRESS_CHARS[idx]).fg(yansi_color));
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
                            0 => YansiColor::Red,
                            1 => YansiColor::Yellow,
                            2 => YansiColor::Green,
                            3 => YansiColor::Cyan,
                            4 => YansiColor::Blue,
                            _ => YansiColor::Magenta,
                        };

                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(color));
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!(
                            "{}",
                            Paint::new(PROGRESS_CHARS[idx]).fg(YansiColor::BrightWhite)
                        );
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
                        let brightness = (pulse_position * PI).sin().abs();

                        let color = if brightness > 0.7 {
                            YansiColor::BrightCyan
                        } else if brightness > 0.3 {
                            YansiColor::Cyan
                        } else {
                            YansiColor::Blue
                        };

                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(color));
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!(
                            "{}",
                            Paint::new(PROGRESS_CHARS[idx]).fg(YansiColor::BrightBlue)
                        );
                    } else {
                        print!("{}", PROGRESS_CHARS[8]);
                    }
                }
            }
            ProgressBarTheme::Color => {
                for i in 0..bar_width {
                    let position = i as f64 / bar_width as f64;

                    if position < progress_ratio {
                        let color = if position < 0.33 {
                            YansiColor::Green
                        } else if position < 0.66 {
                            YansiColor::Yellow
                        } else {
                            YansiColor::BrightRed
                        };

                        print!("{}", Paint::new(PROGRESS_CHARS[7]).fg(color));
                    } else if i == (progress_ratio * bar_width as f64) as usize
                        && progress_ratio < 1.0
                    {
                        let partial = (progress_ratio * bar_width as f64)
                            - (progress_ratio * bar_width as f64).floor();
                        let idx = (partial * (PROGRESS_CHARS.len() - 1) as f64).floor() as usize;
                        print!(
                            "{}",
                            Paint::new(PROGRESS_CHARS[idx]).fg(YansiColor::BrightGreen)
                        );
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
                let gradient: colorgrad::LinearGradient = colorgrad::GradientBuilder::new()
                    .colors(&[
                        colorgrad::Color::new(0.0, 1.0, 0.0, 1.0), // Green
                        colorgrad::Color::new(1.0, 1.0, 0.0, 1.0), // Yellow
                        colorgrad::Color::new(1.0, 0.0, 0.0, 1.0), // Red
                    ])
                    .build()
                    .unwrap();
                let color = gradient.at((percent / 100.0) as f32).to_rgba8();
                Some(YansiColor::Rgb(color[0], color[1], color[2]))
            }
            ProgressBarTheme::Color => {
                // Keep the original "Gradient" behavior
                if percent < 33.0 {
                    Some(YansiColor::Green)
                } else if percent < 66.0 {
                    Some(YansiColor::Yellow)
                } else {
                    Some(YansiColor::BrightRed)
                }
            }
            ProgressBarTheme::Rainbow => Some(YansiColor::BrightWhite),
            ProgressBarTheme::Pulse => Some(YansiColor::BrightCyan),
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
                _ => Some(YansiColor::BrightWhite),
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
        ProgressBarTheme::Gradient => Some(YansiColor::BrightGreen),
        ProgressBarTheme::Rainbow => Some(YansiColor::BrightCyan),
        ProgressBarTheme::Pulse => Some(YansiColor::BrightCyan),
        ProgressBarTheme::Color => Some(YansiColor::BrightGreen),
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

pub fn run_big_clock(duration: Duration, name: &str, bell: bool) -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let start_time = Instant::now();
    let mut paused = false;
    let mut pause_time: Option<Instant> = None;
    let mut total_pause_duration = Duration::from_secs(0);
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Percentage(40),
                        Constraint::Length(7),
                        Constraint::Percentage(40),
                    ]
                    .as_ref(),
                )
                .split(size);
            let timer_area = chunks[1];
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .title(Span::styled(
                    format!(" ⏲️ {} ", name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            f.render_widget(block.clone(), timer_area);
            let inner_area = timer_area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            });
            let rem = if paused {
                if let Some(pause_start) = pause_time {
                    pause_start.duration_since(start_time) - total_pause_duration
                } else {
                    start_time.elapsed() - total_pause_duration
                }
            } else {
                start_time.elapsed() - total_pause_duration
            };
            let remaining = if rem >= duration {
                Duration::from_secs(0)
            } else {
                duration - rem
            };
            let big_time = if remaining.as_secs() >= 3600 {
                format!(
                    "{:02}:{:02}:{:02}",
                    remaining.as_secs() / 3600,
                    (remaining.as_secs() % 3600) / 60,
                    remaining.as_secs() % 60
                )
            } else {
                format!(
                    "{:02}:{:02}",
                    (remaining.as_secs() % 3600) / 60,
                    remaining.as_secs() % 60
                )
            };
            let big_lines = render_big_time(&big_time);
            let big_block = Paragraph::new(big_lines.join("\n"))
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(big_block, inner_area);
        })?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('p') => {
                        paused = !paused;
                        if paused {
                            pause_time = Some(Instant::now());
                        } else if let Some(pause_start) = pause_time {
                            total_pause_duration += pause_start.elapsed();
                            pause_time = None;
                        }
                    }
                    KeyCode::Char('r') => {
                        pause_time = None;
                        total_pause_duration = Duration::from_secs(0);
                        paused = false;
                    }
                    _ => {}
                }
            }
        }
        let rem = if paused {
            if let Some(pause_start) = pause_time {
                pause_start.duration_since(start_time) - total_pause_duration
            } else {
                start_time.elapsed() - total_pause_duration
            }
        } else {
            start_time.elapsed() - total_pause_duration
        };
        if rem >= duration {
            if bell {
                print!("\x07");
            }
            break;
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
