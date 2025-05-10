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
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};
use std::io::stdout;
use std::time::{Duration, Instant};

use crate::utils::{format_simple_duration, send_notification, should_use_color};
use crate::{ProgressBarTheme, Result};

static BIG_DIGITS: [&[&str]; 11] = [
    &[" ███ ", "█   █", "█   █", "█   █", " ███ "], // 0
    &["  █  ", " ██  ", "  █  ", "  █  ", " ███ "], // 1
    &[" ███ ", "    █", " ███ ", "█    ", "█████"], // 2
    &["████ ", "    █", " ███ ", "    █", "████ "], // 3
    &["█   █", "█   █", "█████", "    █", "    █"], // 4
    &["█████", "█    ", "████ ", "    █", "████ "], // 5
    &[" ███ ", "█    ", "████ ", "█   █", " ███ "], // 6
    &["█████", "    █", "   █ ", "  █  ", "  █  "], // 7
    &[" ███ ", "█   █", " ███ ", "█   █", " ███ "], // 8
    &[" ███ ", "█   █", " ████", "    █", " ███ "], // 9
    &["     ", "  ░  ", "     ", "  ░  ", "     "], // :
];

pub fn render_big_time(time: &str) -> Vec<String> {
    let mut lines = vec![String::new(); 5];
    for ch in time.chars() {
        let idx = match ch {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            ':' => 10,
            _ => 10,
        };
        for (i, l) in BIG_DIGITS[idx].iter().enumerate() {
            lines[i].push_str(l);
            lines[i].push(' ');
        }
    }
    lines
}

pub struct FocusModeApp {
    duration: Duration,
    name: String,
    theme: ProgressBarTheme,
    start_time: Instant,
    paused: bool,
    pause_time: Option<Instant>,
    total_pause_duration: Duration,
    notify_remaining: bool,
    notify_threshold: Duration,
    notified: bool,
    last_duration: Duration,
}

impl FocusModeApp {
    pub fn new(duration: Duration, name: &str, theme: ProgressBarTheme) -> Self {
        Self {
            duration,
            name: name.to_string(),
            theme,
            start_time: Instant::now(),
            paused: false,
            pause_time: None,
            total_pause_duration: Duration::from_secs(0),
            notify_remaining: false,
            notify_threshold: Duration::from_secs(60),
            notified: false,
            last_duration: duration,
        }
    }

    fn get_color(&self, progress: f64) -> Color {
        match self.theme {
            ProgressBarTheme::Plain => Color::White,
            ProgressBarTheme::Gradient => {
                let gradient: colorgrad::LinearGradient = colorgrad::GradientBuilder::new()
                    .colors(&[
                        colorgrad::Color::new(0.0, 1.0, 0.0, 1.0), // Green
                        colorgrad::Color::new(1.0, 1.0, 0.0, 1.0), // Yellow
                        colorgrad::Color::new(1.0, 0.0, 0.0, 1.0), // Red
                    ])
                    .build()
                    .expect("Failed to build gradient");
                let color = gradient.at(progress as f32).to_rgba8();
                Color::Rgb(color[0], color[1], color[2])
            }
            ProgressBarTheme::Color => {
                // This is the old "Gradient" theme behavior
                if progress < 0.33 {
                    Color::Green
                } else if progress < 0.66 {
                    Color::Yellow
                } else {
                    Color::Red
                }
            }
            ProgressBarTheme::Rainbow => Color::Cyan,
            ProgressBarTheme::Pulse => Color::Cyan,
        }
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if self.paused {
            self.pause_time = Some(Instant::now());
        } else if let Some(pause_start) = self.pause_time {
            self.total_pause_duration += pause_start.elapsed();
            self.pause_time = None;
        }
    }

    fn add_time(&mut self, amount: i64) {
        if amount > 0 || self.duration > Duration::from_secs(amount.unsigned_abs()) {
            if amount > 0 {
                self.duration += Duration::from_secs(amount as u64);
            } else {
                self.duration -= Duration::from_secs(amount.unsigned_abs());
            }
        }
    }

    fn elapsed(&self) -> Duration {
        if self.paused {
            if let Some(pause_start) = self.pause_time {
                return pause_start.duration_since(self.start_time) - self.total_pause_duration;
            }
        }
        self.start_time.elapsed() - self.total_pause_duration
    }

    fn remaining(&self) -> Duration {
        if self.elapsed() >= self.duration {
            Duration::from_secs(0)
        } else {
            self.duration - self.elapsed()
        }
    }

    fn progress(&self) -> f64 {
        let progress = self.elapsed().as_secs_f64() / self.duration.as_secs_f64();
        progress.min(1.0)
    }

    fn restart(&mut self) {
        self.start_time = Instant::now();
        self.paused = false;
        self.pause_time = None;
        self.total_pause_duration = Duration::from_secs(0);
        self.notified = false;
        self.duration = self.last_duration;
    }

    fn toggle_notify_remaining(&mut self) {
        self.notify_remaining = !self.notify_remaining;
        self.notified = false;
    }

    fn adjust_notify_threshold(&mut self, delta_secs: i64) {
        let new = if delta_secs.is_positive() {
            self.notify_threshold + Duration::from_secs(delta_secs as u64)
        } else {
            self.notify_threshold
                .saturating_sub(Duration::from_secs((-delta_secs) as u64))
        };
        self.notify_threshold = new.max(Duration::from_secs(1));
        self.notified = false;
    }
}

pub fn run_focus_mode(
    duration: Duration,
    name: &str,
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

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = FocusModeApp::new(duration, name, theme);

    let tick_rate = Duration::from_millis(100);
    let res = run_app(&mut terminal, &mut app, tick_rate, bell, notify);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut FocusModeApp,
    tick_rate: Duration,
    bell: bool,
    notify: bool,
) -> Result<()> {
    let mut last_tick = Instant::now();

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

            let progress = app.progress();

            let border_color = if app.notify_remaining && app.remaining() <= app.notify_threshold && !app.paused { Color::Red } else { app.get_color(progress) };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    " 🕰️ FOCUS MODE ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));

            f.render_widget(block.clone(), timer_area);

            let inner_area = timer_area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            });
            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),    // name
                        Constraint::Length(1),    // progress bar
                        Constraint::Length(1),    // time text
                        Constraint::Length(1),    // controls
                    ]
                    .as_ref(),
                )
                .split(inner_area);

            let name_text = Paragraph::new(app.name.clone())
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(name_text, inner_chunks[0]);

            // --- Progress Bar: fills from left to right, percentage always centered, text color changes on fill ---
            let bar_width: usize = inner_area.width as usize;
            let percent = (progress * 100.0).min(100.0);
            let percent_text = format!("{:.1}%", percent);
            let percent_pos = (bar_width.saturating_sub(percent_text.len())) / 2;
            let bar_color = border_color;
            let filled = (progress * bar_width as f64).round() as usize;
            let mut bar_spans = Vec::with_capacity(bar_width);
            for i in 0..bar_width {
                if i >= percent_pos && i < percent_pos + percent_text.len() {
                    let c = percent_text.chars().nth(i - percent_pos).unwrap_or(' ');
                    // If the percent text is over the filled part, use black fg, else bar color fg
                    let style = if i < filled {
                        Style::default().fg(Color::Black).bg(bar_color).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(bar_color).add_modifier(Modifier::BOLD)
                    };
                    bar_spans.push(Span::styled(c.to_string(), style));
                } else if i < filled {
                    // Filled part
                    bar_spans.push(Span::styled(" ", Style::default().bg(bar_color)));
                } else {
                    // Empty part
                    bar_spans.push(Span::raw(" "));
                }
            }
            let bar_paragraph = Paragraph::new(Text::from(vec![Line::from(bar_spans)]))
                .alignment(Alignment::Left);
            f.render_widget(bar_paragraph, inner_chunks[1]);

            let mut time_text = if app.paused {
                format!(
                    "PAUSED - {} remaining",
                    format_simple_duration(app.remaining())
                )
            } else {
                format!("{} remaining", format_simple_duration(app.remaining()))
            };

            if app.notify_remaining {
                time_text.push_str(&format!(" | notif: {}s", app.notify_threshold.as_secs()));
            }

            let time_paragraph = Paragraph::new(time_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(if app.paused {
                    Color::Yellow
                } else {
                    Color::White
                }).add_modifier(Modifier::BOLD));
            f.render_widget(time_paragraph, inner_chunks[2]);

            let controls_text = "p: pause | +: add 1m | -: subtract 1m | r: restart | n: notif | <: -10s notif | >: +10s notif | q/ESC: quit";
            let controls_paragraph = Paragraph::new(controls_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(controls_paragraph, inner_chunks[3]);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if app.notify_remaining
            && !app.notified
            && app.remaining() <= app.notify_threshold
            && !app.paused
        {
            app.notified = true;
        }

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('p') => app.toggle_pause(),
                    KeyCode::Char('+') => app.add_time(60),
                    KeyCode::Char('-') => app.add_time(-60),
                    KeyCode::Char('r') => app.restart(),
                    KeyCode::Char('n') => app.toggle_notify_remaining(),
                    KeyCode::Char('<') => app.adjust_notify_threshold(-10),
                    KeyCode::Char('>') => app.adjust_notify_threshold(10),
                    KeyCode::Esc => return Ok(()),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if !app.paused && app.elapsed() >= app.duration {
            if bell {
                print!("\x07");
            }

            terminal.draw(|f| {
                let size = f.area();

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(2)
                    .constraints(
                        [
                            Constraint::Percentage(40),
                            Constraint::Length(3),
                            Constraint::Percentage(40),
                        ]
                        .as_ref(),
                    )
                    .split(size);

                let completion_text = vec![
                    Line::from(Span::styled(
                        format!("{} completed!", app.name),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        "Press any key to exit",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];

                let completion_paragraph = Paragraph::new(completion_text)
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Green)),
                    );

                f.render_widget(completion_paragraph, chunks[1]);
            })?;

            if notify {
                send_notification(&app.name, app.duration)?;
            }

            if event::poll(Duration::from_secs(u64::MAX))? {
                let _ = event::read()?;
            }

            return Ok(());
        }
    }
}
