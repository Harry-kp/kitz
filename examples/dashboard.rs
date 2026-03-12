use std::time::Duration;

use kitz::prelude::*;
use ratatui::layout::Direction;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Sparkline};

struct App {
    current_theme: Theme,
    cpu_history: Vec<u64>,
    mem_history: Vec<u64>,
    events: Vec<String>,
    ticks: u64,
}

#[derive(Debug, Clone)]
enum Msg {
    Tick,
    CycleTheme,
}

impl App {
    fn new() -> Self {
        Self {
            current_theme: Theme::default(),
            cpu_history: vec![0; 60],
            mem_history: vec![0; 60],
            events: vec!["Dashboard started".into()],
            ticks: 0,
        }
    }
}

impl Application for App {
    type Message = Msg;

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("left", Constraint::Percentage(50)),
            ("right", Constraint::Percentage(50)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "left" => "System Monitor",
            "right" => "Events Log",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        match id {
            "left" => self.render_monitor(frame, area),
            "right" => self.render_events(frame, area),
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "left" => vec![KeyHint::new("t", "Cycle theme")],
            _ => vec![],
        }
    }

    fn panel_handle_key(&mut self, _id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        if key.code == KeyCode::Char('t') {
            return EventResult::Message(Msg::CycleTheme);
        }
        EventResult::Ignored
    }

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Tick => {
                self.ticks += 1;
                let cpu = simulated_cpu(self.ticks);
                let mem = simulated_mem(self.ticks);

                self.cpu_history.push(cpu);
                if self.cpu_history.len() > 60 {
                    self.cpu_history.remove(0);
                }
                self.mem_history.push(mem);
                if self.mem_history.len() > 60 {
                    self.mem_history.remove(0);
                }

                if self.ticks % 10 == 0 {
                    self.events.push(format!(
                        "[tick {}] CPU: {}%, MEM: {}%",
                        self.ticks, cpu, mem
                    ));

                    if cpu > 80 {
                        ctx.toast(format!("High CPU: {}%", cpu), ToastLevel::Warning);
                    }
                }
            }
            Msg::CycleTheme => {
                self.current_theme = self.current_theme.next();
                ctx.toast(
                    format!("Theme: {}", self.current_theme.name),
                    ToastLevel::Info,
                );
            }
        }
        Command::none()
    }

    fn theme(&self) -> Theme {
        self.current_theme.clone()
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![Subscription::every(
            "tick",
            Duration::from_millis(500),
            || Msg::Tick,
        )]
    }
}

impl App {
    fn render_monitor(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(5),
                Constraint::Length(2),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(area);

        // CPU label
        let cpu_val = self.cpu_history.last().copied().unwrap_or(0);
        let cpu_label = Line::from(vec![
            Span::styled("  CPU ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{}%", cpu_val),
                Style::default().fg(self.current_theme.accent),
            ),
        ]);
        frame.render_widget(Paragraph::new(cpu_label), chunks[0]);

        // CPU sparkline
        let cpu_spark = Sparkline::default()
            .block(Block::default().borders(Borders::NONE))
            .data(&self.cpu_history)
            .max(100)
            .style(Style::default().fg(self.current_theme.accent));
        frame.render_widget(cpu_spark, chunks[1]);

        // MEM label
        let mem_val = self.mem_history.last().copied().unwrap_or(0);
        let mem_label = Line::from(vec![
            Span::styled("  MEM ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{}%", mem_val),
                Style::default().fg(self.current_theme.success),
            ),
        ]);
        frame.render_widget(Paragraph::new(mem_label), chunks[2]);

        // MEM sparkline
        let mem_spark = Sparkline::default()
            .block(Block::default().borders(Borders::NONE))
            .data(&self.mem_history)
            .max(100)
            .style(Style::default().fg(self.current_theme.success));
        frame.render_widget(mem_spark, chunks[3]);

        // Theme info
        let theme_info = Line::styled(
            format!("  Theme: {} (press 't' to cycle)", self.current_theme.name),
            Style::default().fg(self.current_theme.text_muted),
        );
        frame.render_widget(Paragraph::new(theme_info), chunks[4]);
    }

    fn render_events(&self, frame: &mut Frame, area: Rect) {
        let visible = area.height as usize;
        let start = self.events.len().saturating_sub(visible);
        let lines: Vec<Line> = self.events[start..]
            .iter()
            .map(|e| {
                Line::styled(
                    format!("  {}", e),
                    Style::default().fg(self.current_theme.text),
                )
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), area);
    }
}

fn simulated_cpu(tick: u64) -> u64 {
    let base = 40.0 + 30.0 * (tick as f64 * 0.1).sin();
    let noise = ((tick * 7 + 13) % 20) as f64;
    (base + noise).clamp(0.0, 100.0) as u64
}

fn simulated_mem(tick: u64) -> u64 {
    let base = 55.0 + 15.0 * (tick as f64 * 0.05).cos();
    let noise = ((tick * 3 + 7) % 10) as f64;
    (base + noise).clamp(0.0, 100.0) as u64
}

fn main() -> Result<()> {
    kitz::run(App::new())
}
