use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::stdout;
use std::time::{Duration, Instant};
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::utils::{format_simple_duration, send_notification, should_use_color};
use crate::{ProgressBarTheme, Result};

pub struct FocusModeApp {
    duration: Duration,
    name: String,
    theme: ProgressBarTheme,
    start_time: Instant,
    paused: bool,
    pause_time: Option<Instant>,
    total_pause_duration: Duration,
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
        }
    }

    fn get_color(&self, progress: f64) -> Color {
        match self.theme {
            ProgressBarTheme::Plain => Color::White,
            ProgressBarTheme::Gradient => {
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
        if amount > 0 || self.duration > Duration::from_secs(amount.unsigned_abs() as u64) {
            if amount > 0 {
                self.duration += Duration::from_secs(amount as u64);
            } else {
                self.duration -= Duration::from_secs(amount.unsigned_abs() as u64);
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

fn run_app<B: tui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut FocusModeApp,
    tick_rate: Duration,
    bell: bool,
    notify: bool,
) -> Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let size = f.size();

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

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.get_color(progress)))
                .title(Span::styled(
                    " 🕰️ FOCUS MODE ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));

            f.render_widget(block.clone(), timer_area);

            let inner_area = timer_area.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            });
            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
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

            let progress_label = format!("{:.1}%", progress * 100.0);
            let gauge = Gauge::default()
                .block(Block::default())
                .gauge_style(Style::default().fg(app.get_color(progress)))
                .ratio(progress)
                .label(progress_label);
            f.render_widget(gauge, inner_chunks[1]);

            let time_text = if app.paused {
                format!(
                    "PAUSED - {} remaining",
                    format_simple_duration(app.remaining())
                )
            } else {
                format!("{} remaining", format_simple_duration(app.remaining()))
            };

            let time_paragraph = Paragraph::new(time_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(if app.paused {
                    Color::Yellow
                } else {
                    Color::White
                }));
            f.render_widget(time_paragraph, inner_chunks[2]);

            let controls_text = "p: pause | +: add 1m | -: subtract 1m | q/ESC: quit";
            let controls_paragraph = Paragraph::new(controls_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(controls_paragraph, inner_chunks[3]);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('p') => app.toggle_pause(),
                    KeyCode::Char('+') => app.add_time(60),
                    KeyCode::Char('-') => app.add_time(-60),
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
                let size = f.size();

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
                    Spans::from(Span::styled(
                        format!("{} completed!", app.name),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Spans::from(Span::styled(
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
