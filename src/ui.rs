//! Rendering. Reads `App`, draws frames. Styling follows vortix: a dense
//! single-line header, rounded titled panels, a keybinding footer, animated
//! spinners - and a bird's-eye dashboard where Topics, Detail, and Groups are
//! all visible at once. The focused panel gets a cyan border; z zooms it.
//!
//! Rendering is pure and never blocks: expensive data (watermarks, groups)
//! shows a "loading…" placeholder until the worker delivers it.

use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Row, Sparkline, Table, Wrap,
};
use ratatui::Frame;

use crate::app::{App, Modal, Panel, Screen};
use crate::theme;

const MIN_W: u16 = 72;
const MIN_H: u16 = 16;
const SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::APP_BG)),
        area,
    );

    if area.width < MIN_W || area.height < MIN_H {
        let msg = format!(
            "terminal too small ({}×{})\nresize to at least {}×{}",
            area.width, area.height, MIN_W, MIN_H
        );
        frame.render_widget(
            Paragraph::new(msg)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme::WARNING)),
            centered(60, 20, area),
        );
        return;
    }

    match app.screen {
        Screen::EnvSelect => render_env_select(frame, app),
        Screen::Main => render_main(frame, app),
        Screen::Groups => render_groups_screen(frame, app),
    }

    if app.connecting.is_some() {
        render_connecting(frame, app);
    }
    render_modal(frame, app);
    render_toast(frame, app);
}

/// Transient top-right notification (carried from vortix).
fn render_toast(frame: &mut Frame, app: &App) {
    let Some(t) = &app.toast else { return };
    let area = frame.area();
    let w = (area.width / 3)
        .clamp(24, 54)
        .min(area.width.saturating_sub(2));

    let (label, color) = match t.level {
        crate::app::ToastLevel::Info => (" INFO ", theme::ACCENT),
        crate::app::ToastLevel::Success => (" OK ", theme::SUCCESS),
        crate::app::ToastLevel::Warning => (" WARN ", theme::WARNING),
        crate::app::ToastLevel::Error => (" ERROR ", theme::ERROR),
    };

    let inner_w = w.saturating_sub(4).max(1) as usize;
    let text_lines = (t.message.chars().count() / inner_w.max(1)) as u16 + 1;
    let h = text_lines + 2;
    let toast_area = Rect {
        x: area.width.saturating_sub(w + 1),
        y: 1,
        width: w,
        height: h,
    };

    frame.render_widget(Clear, toast_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            label,
            Style::default()
                .fg(theme::PANEL_BG)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::PANEL_BG));
    frame.render_widget(
        Paragraph::new(t.message.clone())
            .block(block)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme::TEXT)),
        toast_area,
    );
}

// ── Shared building blocks ────────────────────────────────────────────────

fn panel(title: &str, focused: bool) -> Block<'static> {
    let border = if focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER
    };
    // No background fill - transparent outline on the app's dark bg, like vortix.
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(if focused {
                    theme::ACCENT_LIGHT
                } else {
                    theme::TEXT_MUTED
                })
                .add_modifier(Modifier::BOLD),
        ))
}

fn highlight(focused: bool) -> (Style, &'static str) {
    if focused {
        (
            Style::default()
                .bg(theme::ROW_SELECTED_BG)
                .fg(theme::ROW_SELECTED_FG)
                .add_modifier(Modifier::BOLD),
            "▶ ",
        )
    } else {
        (Style::default().fg(theme::TEXT_MUTED), "  ")
    }
}

fn sep() -> Span<'static> {
    Span::styled("  │  ", Style::default().fg(theme::SEPARATOR))
}

fn spinner(app: &App) -> &'static str {
    let ms = app
        .connecting
        .as_ref()
        .map(|c| c.started.elapsed().as_millis())
        .unwrap_or(0);
    SPINNER[((ms / 120) % 4) as usize]
}

fn footer(frame: &mut Frame, area: Rect, lead: Option<&str>, hints: &[(&str, &str)], status: &str) {
    let mut spans = vec![Span::raw(" ")];
    if let Some(l) = lead {
        spans.push(Span::styled(
            format!(" {l} "),
            Style::default()
                .bg(theme::ACCENT)
                .fg(theme::PANEL_BG)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw("  "));
    }
    for (k, d) in hints {
        spans.push(Span::styled(
            *k,
            Style::default()
                .fg(theme::KEY_HINT)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {d}"),
            Style::default().fg(theme::TEXT_MUTED),
        ));
        spans.push(Span::styled("   ", Style::default().fg(theme::SEPARATOR)));
    }
    spans.push(Span::styled(
        format!("│  {status}"),
        Style::default().fg(theme::ACCENT_LIGHT),
    ));
    // Transparent (no filled bar) - sits on the app bg like vortix.
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::APP_BG)),
        area,
    );

    // Brand + version, bottom-right, muted (moved here from the header).
    let brand = format!("{} v{} ", crate::brand::NAME, crate::brand::VERSION);
    let bw = brand.chars().count() as u16;
    if area.width > bw + 6 {
        let br = Rect::new(area.right().saturating_sub(bw), area.y, bw, 1);
        frame.render_widget(
            Paragraph::new(Span::styled(brand, Style::default().fg(theme::TEXT_MUTED)))
                .alignment(Alignment::Right)
                .style(Style::default().bg(theme::APP_BG)),
            br,
        );
    }
}

// ── Env select ─────────────────────────────────────────────────────────

fn render_env_select(frame: &mut Frame, app: &mut App) {
    let n = app.config.envs.len() as u16;
    let block_h = (crate::brand::WORDMARK.len() as u16 + 4) + (n + 2);
    let area = centered_fixed(
        58,
        block_h.min(frame.area().height.saturating_sub(2)),
        frame.area(),
    );

    let rows = Layout::vertical([
        Constraint::Length(crate::brand::WORDMARK.len() as u16 + 3), // wordmark + tagline
        Constraint::Min(1),                                          // env list
    ])
    .split(area);

    // ── Branded masthead ──
    frame.render_widget(Clear, area);
    let mut brand_lines = vec![Line::from("")];
    for w in crate::brand::WORDMARK {
        brand_lines.push(Line::from(Span::styled(
            *w,
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
    }
    brand_lines.push(Line::from(Span::styled(
        format!("  {}", crate::brand::TAGLINE),
        Style::default().fg(theme::TEXT_MUTED),
    )));
    frame.render_widget(
        Paragraph::new(brand_lines).style(Style::default().bg(theme::APP_BG)),
        rows[0],
    );

    // ── Environment list ──
    let items: Vec<ListItem> = app
        .config
        .envs
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut spans = vec![
                Span::styled(
                    format!("{}·", i + 1),
                    Style::default().fg(theme::TEXT_MUTED),
                ),
                Span::styled(
                    format!("{:<12}", e.name),
                    Style::default()
                        .fg(theme::env_color(e.prod))
                        .add_modifier(Modifier::BOLD),
                ),
            ];
            if e.prod {
                spans.push(Span::styled(
                    " PROD ",
                    Style::default()
                        .fg(theme::PANEL_BG)
                        .bg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            spans.push(Span::styled(
                format!("  {}", host_only(&e.bootstrap)),
                Style::default().fg(theme::TEXT_MUTED),
            ));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(panel("select environment", true))
        .highlight_style(
            Style::default()
                .bg(theme::ROW_SELECTED_BG)
                .fg(theme::ROW_SELECTED_FG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, rows[1], &mut app.env_state);

    let fa = frame.area();
    footer(
        frame,
        Rect::new(fa.x, fa.bottom() - 1, fa.width, 1),
        None,
        &[("↑↓", "select"), ("↵", "connect"), ("q", "quit")],
        &format!("v{}", crate::brand::VERSION),
    );
}

fn render_connecting(frame: &mut Frame, app: &App) {
    let Some(conn) = &app.connecting else { return };
    let area = centered_fixed(44, 5, frame.area());
    frame.render_widget(Clear, area);
    let elapsed = conn.started.elapsed().as_secs();
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", spinner(app)),
                Style::default()
                    .fg(theme::ACCENT_LIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("connecting to ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                conn.profile.name.clone(),
                Style::default()
                    .fg(theme::env_color(conn.profile.prod))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {elapsed}s"),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ])
        .alignment(Alignment::Center),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(theme::ACCENT))
                .style(Style::default().bg(theme::PANEL_BG)),
        ),
        area,
    );
}

// ── Main dashboard ─────────────────────────────────────────────────────

fn render_main(frame: &mut Frame, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_header(frame, rows[0], app);

    if app.zoom {
        match app.focus {
            Panel::Topics => render_topics(frame, rows[1], app, true),
            Panel::Graph => render_graph(frame, rows[1], app, true),
            Panel::Detail if app.flip.showing_back() => render_config(frame, rows[1], app, true),
            Panel::Detail => render_detail(frame, rows[1], app, true),
            Panel::Logs => render_logs(frame, rows[1], app, true),
        }
    } else {
        // Aligned 2×2 grid: split into rows first, then each row at the same x -
        // so the left/right pane boundaries line up. Left column is narrower.
        let grid = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        let split = [Constraint::Percentage(37), Constraint::Percentage(63)];
        let top = Layout::horizontal(split).split(grid[0]);
        let bot = Layout::horizontal(split).split(grid[1]);

        render_topics(frame, top[0], app, app.focus == Panel::Topics);
        render_graph(frame, top[1], app, app.focus == Panel::Graph);
        render_flip_pane(frame, bot[0], app, app.focus == Panel::Detail);
        render_logs(frame, bot[1], app, app.focus == Panel::Logs);
    }

    // Footer stays lean - the essentials for this pane. Everything else lives
    // behind `x` (actions) and `?` (help).
    let (pane_name, hints): (&str, &[(&str, &str)]) = match app.focus {
        Panel::Topics => (
            "TOPICS",
            &[
                ("↑↓", "move"),
                ("⇥", "pane"),
                ("/", "find"),
                ("p", "peek"),
                ("x", "actions"),
                ("?", "help"),
            ],
        ),
        Panel::Graph => (
            "GRAPH",
            &[
                ("⇥", "pane"),
                ("w", "track"),
                ("G", "groups"),
                ("x", "actions"),
                ("?", "help"),
            ],
        ),
        Panel::Detail if app.flip.showing_back() => (
            "CONFIG",
            &[
                ("f", "flip→detail"),
                ("⇥", "pane"),
                ("x", "actions"),
                ("?", "help"),
            ],
        ),
        Panel::Detail => (
            "DETAIL",
            &[
                ("↑↓", "scroll"),
                ("f", "flip→config"),
                ("w", "track"),
                ("p", "peek"),
                ("x", "actions"),
                ("?", "help"),
            ],
        ),
        Panel::Logs => (
            "LOGS",
            &[
                ("↑↓", "scroll"),
                ("⇥", "pane"),
                ("x", "actions"),
                ("?", "help"),
            ],
        ),
    };
    footer(frame, rows[2], Some(pane_name), hints, &app.status);
}

/// Header = environment switcher strip + counts + live dot. Active env
/// highlighted; prod red. Press 1-9 to hot-switch. (Brand lives in the footer.)
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(
        " env ",
        Style::default().fg(theme::TEXT_MUTED),
    )];

    let active = app.current_env_index();
    let env_style = |i: usize, prod: bool| {
        if active == Some(i) {
            Style::default()
                .bg(if prod { theme::ERROR } else { theme::ACCENT })
                .fg(theme::PANEL_BG)
                .add_modifier(Modifier::BOLD)
        } else if prod {
            Style::default().fg(theme::ERROR)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
        }
    };
    let label = |i: usize, e: &crate::config::EnvProfile| {
        format!(" {}·{}{} ", i + 1, e.name, if e.prod { " ⚠" } else { "" })
    };

    // Budget the strip so it never clips the counts / live dot; overflow → +N.
    let counts_text = if app.groups_loaded {
        format!("{} topics · {} groups", app.topic_count(), app.groups.len())
    } else {
        format!("{} topics", app.topic_count())
    };
    let tail =
        counts_text.chars().count() + 8 /* " ● live" + sep */ + if app.zoom { 10 } else { 0 };
    let budget = (area.width as usize).saturating_sub(5 + tail + 6);

    let mut used = 0usize;
    let mut shown = 0usize;
    for (i, e) in app.config.envs.iter().enumerate() {
        let lbl = label(i, e);
        let w = lbl.chars().count();
        if used + w > budget {
            break;
        }
        used += w;
        shown += 1;
        spans.push(Span::styled(lbl, env_style(i, e.prod)));
    }
    // Guarantee the active env is visible even if it fell past the budget.
    if let Some(a) = active {
        if a >= shown {
            let e = &app.config.envs[a];
            spans.push(Span::styled(label(a, e), env_style(a, e.prod)));
        }
    }
    let dropped = app.config.envs.len().saturating_sub(shown);
    if dropped > 0 {
        spans.push(Span::styled(
            format!(" +{dropped}"),
            Style::default().fg(theme::TEXT_MUTED),
        ));
    }

    if app.zoom {
        spans.push(Span::styled(
            "  ⛶ zoomed",
            Style::default().fg(theme::WARNING),
        ));
    }
    spans.push(sep());
    spans.push(Span::styled(
        counts_text,
        Style::default().fg(theme::TEXT_MUTED),
    ));
    spans.push(Span::styled(
        "   ● live",
        Style::default()
            .fg(theme::SUCCESS)
            .add_modifier(Modifier::BOLD),
    ));

    // Transparent header - no filled bar (vortix look).
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::APP_BG)),
        area,
    );
}

fn render_topics(frame: &mut Frame, area: Rect, app: &mut App, focused: bool) {
    let visible = app.filtered_topics();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let (name, parts) = app.topic_row(i);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<34}", truncate(name, 34)),
                    Style::default().fg(theme::TEXT),
                ),
                Span::styled(format!("{parts:>3}p"), Style::default().fg(theme::ACCENT)),
            ]))
        })
        .collect();

    let title = if app.filtering || !app.filter.is_empty() {
        format!("Topics · /{}", app.filter)
    } else {
        format!("Topics · {}", app.topic_count())
    };
    let (hl, sym) = highlight(focused);
    let list = List::new(items)
        .block(panel(&title, focused))
        .highlight_style(hl)
        .highlight_symbol(sym);
    frame.render_stateful_widget(list, area, &mut app.topic_state);
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App, focused: bool) {
    let Some(d) = &app.detail else {
        frame.render_widget(
            Paragraph::new("  select a topic")
                .style(Style::default().fg(theme::TEXT_MUTED))
                .block(panel("Detail", focused)),
            area,
        );
        return;
    };

    let kv = |k: &str, v: Span<'static>| {
        Line::from(vec![
            Span::styled(format!("  {k:<12}"), Style::default().fg(theme::TEXT_MUTED)),
            v,
        ])
    };

    let events = if d.watermarks_loaded {
        Span::styled(
            fmt_count(d.total_messages()),
            Style::default().fg(theme::SUCCESS),
        )
    } else if app.loading_watermarks {
        Span::styled(
            format!("{} loading…", spinner(app)),
            Style::default().fg(theme::WARNING),
        )
    } else {
        Span::styled("w to load", Style::default().fg(theme::TEXT_MUTED))
    };

    let cell = |v: i64| {
        if v < 0 {
            "-".to_string()
        } else {
            v.to_string()
        }
    };

    // Consumer groups actually subscribed to this topic.
    let consumers = {
        let g = app.groups_for_topic(&d.name);
        if !app.groups_loaded {
            Span::styled(
                format!("{} loading…", spinner(app)),
                Style::default().fg(theme::WARNING),
            )
        } else if g.is_empty() {
            Span::styled("none", Style::default().fg(theme::TEXT_MUTED))
        } else {
            let count = g.len();
            Span::styled(
                format!("{count} · {}", truncate(&g.join(", "), 16)),
                Style::default().fg(theme::ACCENT),
            )
        }
    };

    let mut lines = vec![
        kv(
            "topic",
            Span::styled(
                d.name.clone(),
                Style::default()
                    .fg(theme::ACCENT_LIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
        ),
        kv(
            "partitions",
            Span::styled(
                d.partitions.len().to_string(),
                Style::default().fg(theme::TEXT),
            ),
        ),
        kv("~events", events),
        kv("consumers", consumers),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {:<3}{:<6}{:>11}{:>11}", "id", "isr", "low", "high"),
            Style::default()
                .fg(theme::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    for p in &d.partitions {
        lines.push(Line::from(format!(
            "  {:<3}{:<6}{:>11}{:>11}",
            p.id,
            format!("{}/{}", p.isr, p.replicas),
            cell(p.low),
            cell(p.high)
        )));
    }

    // No wrap: long lines clip cleanly instead of wrapping in the narrow pane.
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("Detail · f→config", focused))
            .scroll((app.detail_scroll, 0)),
        area,
    );
}

fn render_groups(frame: &mut Frame, area: Rect, app: &mut App, focused: bool) {
    if !app.groups_loaded {
        let msg = if app.loading_groups {
            format!("  {} loading consumer groups…", spinner(app))
        } else {
            "  focus this panel (⇥) to load consumer groups".to_string()
        };
        frame.render_widget(
            Paragraph::new(msg)
                .style(Style::default().fg(theme::TEXT_MUTED))
                .block(panel("Consumer groups", focused)),
            area,
        );
        return;
    }

    let header = Row::new(vec!["group", "state", "members", "protocol"]).style(
        Style::default()
            .fg(theme::TEXT_MUTED)
            .add_modifier(Modifier::BOLD),
    );
    let rows: Vec<Row> = app
        .groups
        .iter()
        .map(|g| {
            let state_color = match g.state.as_str() {
                "Stable" => theme::SUCCESS,
                "Empty" | "Dead" => theme::INACTIVE,
                _ => theme::WARNING,
            };
            Row::new(vec![
                Span::styled(truncate(&g.name, 40), Style::default().fg(theme::TEXT)),
                Span::styled(g.state.clone(), Style::default().fg(state_color)),
                Span::styled(g.members.to_string(), Style::default().fg(theme::ACCENT)),
                Span::styled(g.protocol.clone(), Style::default().fg(theme::TEXT_MUTED)),
            ])
        })
        .collect();
    let (hl, sym) = highlight(focused);
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(48),
            Constraint::Length(12),
            Constraint::Length(9),
            Constraint::Min(6),
        ],
    )
    .header(header)
    .block(panel(
        &format!("Consumer groups · {}", app.groups.len()),
        focused,
    ))
    .row_highlight_style(hl)
    .highlight_symbol(sym);
    frame.render_stateful_widget(table, area, &mut app.group_state);
}

/// Topic config panel (top-right) - the day-to-day "what's this topic set to?"
/// glance: retention, cleanup policy, min ISR, etc. Follows the highlight.
fn render_config(frame: &mut Frame, area: Rect, app: &App, focused: bool) {
    let lines: Vec<Line> = match &app.topic_config {
        _ if app.detail.is_none() => {
            vec![Line::from(Span::styled(
                "  select a topic",
                Style::default().fg(theme::TEXT_MUTED),
            ))]
        }
        Some((topic, entries))
            if app
                .detail
                .as_ref()
                .map(|d| &d.name == topic)
                .unwrap_or(false) =>
        {
            entries
                .iter()
                .map(|(k, v)| {
                    Line::from(vec![
                        Span::styled(format!("  {k:<20}"), Style::default().fg(theme::TEXT_MUTED)),
                        Span::styled(v.clone(), Style::default().fg(theme::TEXT)),
                    ])
                })
                .collect()
        }
        _ => vec![Line::from(Span::styled(
            format!("  {} loading…", spinner(app)),
            Style::default().fg(theme::WARNING),
        ))],
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("Config · f→detail", focused))
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// Live incoming-events graph (top-right). Sparkline of events/interval for the
/// topic opted into via `w`, plus current + peak.
fn render_graph(frame: &mut Frame, area: Rect, app: &App, focused: bool) {
    let title = match &app.rate_topic {
        Some(t) => format!("Events/s · {}", truncate(t, 28)),
        None => "Events/s".to_string(),
    };
    let block = panel(&title, focused);

    if app.rate_topic.is_none() {
        frame.render_widget(
            Paragraph::new("\n  press w on a topic to track its\n  incoming event rate live")
                .style(Style::default().fg(theme::TEXT_MUTED))
                .block(block),
            area,
        );
        return;
    }
    if app.rate.len() < 2 {
        frame.render_widget(
            Paragraph::new(format!("\n  {} sampling…", spinner(app)))
                .style(Style::default().fg(theme::WARNING))
                .block(block),
            area,
        );
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cur = *app.rate.last().unwrap_or(&0);
    let peak = app.rate.iter().copied().max().unwrap_or(0);
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  now ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                fmt_count(cur as i64),
                Style::default()
                    .fg(theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   peak ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(fmt_count(peak as i64), Style::default().fg(theme::ACCENT)),
            Span::styled("   /3.5s window", Style::default().fg(theme::SEPARATOR)),
        ])),
        rows[0],
    );
    frame.render_widget(
        Sparkline::default()
            .data(&app.rate)
            .style(Style::default().fg(theme::ACCENT_LIGHT)),
        rows[1],
    );
}

/// Bottom-left flip pane: Detail (front) ⟷ Config (back), with the
/// horizontal-squish card-flip animation driven by `app.flip` (FlipState).
/// Renders the narrowing manually (like vortix) so the face closures can read
/// `app` immutably while the state ticks in `App::tick`.
fn render_flip_pane(frame: &mut Frame, area: Rect, app: &App, focused: bool) {
    let render_face = |frame: &mut Frame, r: Rect| {
        if app.flip.showing_back() {
            render_config(frame, r, app, focused);
        } else {
            render_detail(frame, r, app, focused);
        }
    };

    if !app.flip.is_animating() {
        render_face(frame, area);
        return;
    }

    // Repaint the app bg over the whole slot (erasing the wider previous face)
    // rather than Clear, which would drop back to the terminal's default bg.
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::APP_BG)),
        area,
    );
    let narrow = narrowed_rect(area, app.flip.width_ratio());
    if narrow.width >= 24 {
        render_face(frame, narrow);
    } else {
        // Collapsed to a sliver - draw the flip edge (a glowing seam).
        let buf = frame.buffer_mut();
        let mid = narrow.x + narrow.width / 2;
        for y in narrow.y..narrow.y.saturating_add(narrow.height) {
            if let Some(cell) = buf.cell_mut((mid, y)) {
                cell.set_symbol("│")
                    .set_style(Style::default().fg(theme::ACCENT_LIGHT));
            }
        }
    }
}

/// Centre-shrink `area` to `ratio` of its width (for the flip animation).
fn narrowed_rect(area: Rect, ratio: f32) -> Rect {
    let w = ((area.width as f32) * ratio).max(1.0) as u16;
    let w = w.min(area.width);
    let x = area.x + area.width.saturating_sub(w) / 2;
    Rect::new(x, area.y, w, area.height)
}

/// Activity/debug log panel - global by nature (vortix-style). Tails newest at
/// the bottom; ↑↓ scrolls back when focused.
fn render_logs(frame: &mut Frame, area: Rect, app: &App, focused: bool) {
    let inner_h = area.height.saturating_sub(2) as usize;
    let total = app.logs.len();
    let end = total.saturating_sub(app.logs_scroll as usize);
    let start = end.saturating_sub(inner_h);

    let lines: Vec<Line> = if total == 0 {
        vec![Line::from(Span::styled(
            "  no activity yet",
            Style::default().fg(theme::TEXT_MUTED),
        ))]
    } else {
        app.logs[start..end]
            .iter()
            .map(|l| {
                Line::from(Span::styled(
                    format!("  {l}"),
                    Style::default().fg(theme::TEXT_MUTED),
                ))
            })
            .collect()
    };
    let title = if app.logs_scroll > 0 {
        format!("Logs · ↑{}", app.logs_scroll)
    } else {
        "Logs".to_string()
    };
    frame.render_widget(Paragraph::new(lines).block(panel(&title, focused)), area);
}

/// Full-screen cluster-wide consumer groups (reached with `G`).
fn render_groups_screen(frame: &mut Frame, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());
    render_header(frame, rows[0], app);
    render_groups(frame, rows[1], app, true);
    footer(
        frame,
        rows[2],
        Some("GROUPS"),
        &[
            ("↑↓", "move"),
            ("d", "delete"),
            ("x", "actions"),
            ("esc", "back"),
            ("?", "help"),
        ],
        &app.status,
    );
}

// ── Modals ───────────────────────────────────────────────────────────────

fn render_modal(frame: &mut Frame, app: &App) {
    match &app.modal {
        Modal::None => {}
        Modal::Help => {
            let row = |keys: &str, desc: &str| {
                Line::from(vec![
                    Span::styled(
                        format!("  {keys:<14}"),
                        Style::default()
                            .fg(theme::KEY_HINT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(desc.to_string(), Style::default().fg(theme::TEXT)),
                ])
            };
            let head = |t: &str| {
                Line::from(Span::styled(
                    format!("  {t}"),
                    Style::default()
                        .fg(theme::ACCENT_LIGHT)
                        .add_modifier(Modifier::BOLD),
                ))
            };
            popup(
                frame,
                "Shortcuts",
                vec![
                    Line::from(""),
                    head("Environments"),
                    row("1–9", "switch to that environment"),
                    row("e", "open environment picker"),
                    Line::from(""),
                    head("Panes"),
                    row("⇥ / h l", "focus: Topics · Graph · Detail · Logs"),
                    row("f", "flip bottom-left: Detail ⟷ Config"),
                    row("z", "zoom the focused pane"),
                    row("g", "jump to top of a list"),
                    Line::from(""),
                    head("Topics"),
                    row("↑↓ / j k", "move · detail follows the selection"),
                    row("/", "filter topics"),
                    row("w", "event counts + live graph"),
                    row("p", "peek events (y copy payload · Y copy key)"),
                    row("y", "copy topic name to clipboard"),
                    row("c / a / d", "create / add partitions / delete"),
                    Line::from(""),
                    head("Cluster"),
                    row("G", "consumer groups (full screen)"),
                    row("r", "refresh"),
                    Line::from(""),
                    head("General"),
                    row("? / esc", "close this help  ·  q  quit"),
                ],
                theme::ACCENT,
                27,
            );
        }
        Modal::Error(msg) => popup(
            frame,
            "Error",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {msg}"),
                    Style::default().fg(theme::ERROR),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  press any key to dismiss",
                    Style::default().fg(theme::TEXT_MUTED),
                )),
            ],
            theme::ERROR,
            9,
        ),
        Modal::Create(f) => {
            let field = |label: &str, val: &str, focused: bool| {
                let vstyle = if focused {
                    Style::default()
                        .fg(theme::ACCENT_LIGHT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };
                Line::from(vec![
                    Span::styled(
                        format!("  {} {label:<12}", if focused { "▸" } else { " " }),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                    Span::styled(format!("{val}{}", if focused { "▌" } else { "" }), vstyle),
                ])
            };
            popup(
                frame,
                "Create topic",
                vec![
                    Line::from(""),
                    field("name", &f.name, f.focus == 0),
                    field("partitions", &f.partitions, f.focus == 1),
                    field("replication", &f.replication, f.focus == 2),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ⇥ next   ↵ create   esc cancel",
                        Style::default().fg(theme::TEXT_MUTED),
                    )),
                ],
                theme::ACCENT,
                9,
            );
        }
        Modal::AddPartitions(f) => popup(
            frame,
            "Add partitions",
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  topic  ", Style::default().fg(theme::TEXT_MUTED)),
                    Span::styled(f.topic.clone(), Style::default().fg(theme::ACCENT_LIGHT)),
                ]),
                Line::from(vec![
                    Span::styled("  total  ", Style::default().fg(theme::TEXT_MUTED)),
                    Span::styled(
                        format!("{}▌", f.total),
                        Style::default()
                            .fg(theme::ACCENT_LIGHT)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  partitions can only increase   ↵ apply   esc cancel",
                    Style::default().fg(theme::WARNING),
                )),
            ],
            theme::ACCENT,
            9,
        ),
        Modal::Delete(f) => {
            let noun = match f.kind {
                crate::app::DeleteKind::Topic => "topic",
                crate::app::DeleteKind::Group => "group",
            };
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("  delete {noun} "),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                    Span::styled(
                        f.target.clone(),
                        Style::default()
                            .fg(theme::ERROR)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ?", Style::default().fg(theme::TEXT_MUTED)),
                ]),
                Line::from(""),
            ];
            if f.is_prod {
                lines.push(Line::from(Span::styled(
                    format!("  ⚠ PROD - type the {noun} name to confirm:"),
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{}▌", f.confirm),
                        Style::default().fg(theme::ACCENT_LIGHT),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  ↵ confirm   esc cancel",
                    Style::default().fg(theme::TEXT_MUTED),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  ↵ confirm delete   esc cancel",
                    Style::default().fg(theme::WARNING),
                )));
            }
            let h = lines.len() as u16 + 2;
            popup(frame, &format!("Delete {noun}"), lines, theme::ERROR, h);
        }
        Modal::Peek { records, sel } => render_peek(frame, records, *sel),
        Modal::Actions { items, sel } => {
            let mut lines = vec![Line::from("")];
            for (i, (k, label)) in items.iter().enumerate() {
                let selected = i == *sel;
                let base = if selected {
                    Style::default()
                        .bg(theme::ROW_SELECTED_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        if selected { "  ▶ " } else { "    " },
                        base.fg(theme::ACCENT_LIGHT),
                    ),
                    Span::styled(
                        format!("{k}  "),
                        base.fg(theme::KEY_HINT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled((*label).to_string(), base.fg(theme::TEXT)),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ↑↓ select · ↵ run · esc close",
                Style::default().fg(theme::TEXT_MUTED),
            )));
            let h = lines.len() as u16 + 2;
            popup(frame, "Actions", lines, theme::ACCENT, h);
        }
    }
}

/// Interactive event browser: event list (top) + full pretty-printed payload of
/// the selected event (bottom). y copies the payload, Y the key.
fn render_peek(frame: &mut Frame, records: &[crate::kafka::EventRecord], sel: usize) {
    let a = frame.area();
    let w = (a.width * 85 / 100).clamp(50, 130);
    let h = (a.height * 85 / 100).clamp(12, 44);
    let area = centered_fixed(w, h, a);
    frame.render_widget(Clear, area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            format!(
                " Peek · {} events   ↑↓ select · y copy payload · Y copy key · esc close ",
                records.len()
            ),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::PANEL_BG));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if records.is_empty() {
        frame.render_widget(
            Paragraph::new("  no events in this topic")
                .style(Style::default().fg(theme::TEXT_MUTED)),
            inner,
        );
        return;
    }

    let parts =
        Layout::vertical([Constraint::Percentage(45), Constraint::Percentage(55)]).split(inner);

    // ── Event list (windowed around the selection) ──
    let list_h = parts[0].height as usize;
    let start = sel
        .saturating_sub(list_h / 2)
        .min(records.len().saturating_sub(list_h));
    let mut list_lines = Vec::new();
    for (i, r) in records.iter().enumerate().skip(start).take(list_h) {
        let selected = i == sel;
        let base = if selected {
            Style::default()
                .bg(theme::ROW_SELECTED_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        list_lines.push(Line::from(vec![
            Span::styled(
                if selected { " ▶ " } else { "   " },
                base.fg(theme::ACCENT_LIGHT),
            ),
            Span::styled(
                format!("p{:<2} @{:<10} ", r.partition, r.offset),
                base.fg(theme::ACCENT),
            ),
            Span::styled(
                format!("{:<20}", truncate(&r.key, 18)),
                base.fg(theme::WARNING),
            ),
            Span::styled(
                truncate(&r.payload, (parts[0].width as usize).saturating_sub(40)),
                base.fg(theme::TEXT),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(list_lines), parts[0]);

    // ── Selected payload (pretty-printed if JSON) ──
    let r = &records[sel];
    let ts = r
        .timestamp
        .map(|t| t.to_string())
        .unwrap_or_else(|| "-".into());
    let mut detail = vec![Line::from(vec![
        Span::styled("─ payload  ", Style::default().fg(theme::SEPARATOR)),
        Span::styled(
            format!(
                "partition {} · offset {} · ts {} · key {}",
                r.partition,
                r.offset,
                ts,
                if r.key.is_empty() { "∅" } else { &r.key }
            ),
            Style::default().fg(theme::TEXT_MUTED),
        ),
    ])];
    for line in pretty_json(&r.payload)
        .lines()
        .take(parts[1].height.saturating_sub(1) as usize)
    {
        detail.push(Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(theme::TEXT),
        )));
    }
    frame.render_widget(Paragraph::new(detail).wrap(Wrap { trim: false }), parts[1]);
}

/// Pretty-print a payload as JSON if it parses; otherwise return it verbatim.
fn pretty_json(s: &str) -> String {
    serde_json::from_str::<serde_json::Value>(s)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or_else(|_| s.to_string())
}

fn popup(frame: &mut Frame, title: &str, lines: Vec<Line>, accent: Color, rows: u16) {
    let a = frame.area();
    let w = (a.width * 7 / 10).clamp(40, 96);
    let h = rows.min(a.height.saturating_sub(2)).max(5);
    let area = centered_fixed(w, h, a);
    frame.render_widget(Clear, area);
    let b = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::PANEL_BG));
    frame.render_widget(
        Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
        area,
    );
}

// ── Geometry + text helpers ───────────────────────────────────────────────

fn centered(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let [v] = Layout::vertical([Constraint::Percentage(pct_y)])
        .flex(Flex::Center)
        .areas(r);
    let [h] = Layout::horizontal([Constraint::Percentage(pct_x)])
        .flex(Flex::Center)
        .areas(v);
    h
}

fn centered_fixed(w: u16, h: u16, r: Rect) -> Rect {
    let [v] = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .areas(r);
    let [out] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(v);
    out
}

fn host_only(bootstrap: &str) -> String {
    bootstrap.split(',').next().unwrap_or(bootstrap).to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

fn fmt_count(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    let rev: String = out.chars().rev().collect();
    if n < 0 {
        format!("-{rev}")
    } else {
        rev
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Modal, Panel, Screen};
    use crate::config::{Config, EnvProfile};
    use crate::kafka::{EventRecord, PartMeta, PartitionInfo, TopicDetail, TopicMeta};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn env(name: &str, prod: bool) -> EnvProfile {
        EnvProfile {
            name: name.into(),
            bootstrap: format!("b-1.{name}.xxxx.c2.kafka.eu-central-1.amazonaws.com:9092"),
            region: "eu-central-1".into(),
            auth: "plaintext".into(),
            aws_profile: None,
            prod,
        }
    }

    fn demo_app() -> App {
        let cfg = Config {
            envs: vec![env("stag", false), env("prod", true)],
        };
        let mut app = App::new(cfg);
        app.connected = Some(env("stag", false));
        app.meta = (0..12)
            .map(|i| TopicMeta {
                name: format!("service.events.v{i}"),
                partitions: (0..(3 + i % 4))
                    .map(|id| PartMeta {
                        id,
                        leader: 1 + id % 3,
                        replicas: 3,
                        isr: 3,
                    })
                    .collect(),
            })
            .collect();
        app.topic_state.select(Some(2));
        app.detail = Some(TopicDetail {
            name: "service.events.v2".into(),
            partitions: (0..4)
                .map(|id| PartitionInfo {
                    id,
                    leader: 1 + id % 3,
                    replicas: 3,
                    isr: 3,
                    low: 0,
                    high: 148_233 + id as i64 * 5000,
                })
                .collect(),
            watermarks_loaded: true,
        });
        app.groups = vec![
            crate::kafka::GroupSummary {
                name: "billing-consumer".into(),
                state: "Stable".into(),
                members: 4,
                protocol: "range".into(),
                topics: vec!["service.events.v2".into()],
            },
            crate::kafka::GroupSummary {
                name: "analytics-etl".into(),
                state: "Stable".into(),
                members: 2,
                protocol: "range".into(),
                topics: vec!["service.events.v2".into(), "service.events.v5".into()],
            },
        ];
        app.groups_loaded = true;
        app.topic_config = Some((
            "service.events.v2".into(),
            vec![
                ("cleanup.policy".into(), "delete".into()),
                ("retention.ms".into(), "604800000".into()),
                ("retention.bytes".into(), "-1".into()),
                ("max.message.bytes".into(), "1048588".into()),
                ("min.insync.replicas".into(), "2".into()),
                ("segment.ms".into(), "604800000".into()),
                ("compression.type".into(), "producer".into()),
            ],
        ));
        app.rate_topic = Some("service.events.v2".into());
        app.rate = vec![
            12, 40, 33, 58, 71, 49, 88, 64, 95, 120, 77, 60, 44, 90, 110, 130, 85, 52,
        ];
        app.logs = vec![
            "10:02:11  connected to stag · 12 topics".into(),
            "10:02:19  tracking service.events.v2 - graph is live".into(),
            "10:03:04  peeked 50 events".into(),
        ];
        app
    }

    #[test]
    fn render_dashboard_smoke() {
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.focus = Panel::Topics;
        println!("\n===== DASHBOARD (Detail + Logs, 108x26) =====");
        println!("{}", dump(&mut app, 108, 26));
        assert!(dump(&mut app, 108, 26).contains("service.events"));

        app.flip.set_showing_back(true);
        println!("\n===== FLIPPED (bottom-left → Config) =====");
        println!("{}", dump(&mut app, 108, 20));
    }

    #[test]
    fn render_actions_and_groups_smoke() {
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.modal = Modal::Actions {
            items: vec![
                ('w', "load event counts"),
                ('p', "peek events  (y copy payload)"),
                ('c', "create topic"),
                ('d', "delete topic"),
                ('G', "consumer groups"),
                ('e', "switch environment"),
            ],
            sel: 3,
        };
        println!("\n===== ACTIONS MENU (x) =====");
        println!("{}", dump(&mut app, 100, 24));

        app.modal = Modal::None;
        app.screen = Screen::Groups;
        app.groups = vec![
            crate::kafka::GroupSummary {
                name: "billing-consumer".into(),
                state: "Stable".into(),
                members: 4,
                protocol: "range".into(),
                topics: vec!["service.events.v2".into()],
            },
            crate::kafka::GroupSummary {
                name: "audit-sink".into(),
                state: "Empty".into(),
                members: 0,
                protocol: String::new(),
                topics: vec![],
            },
        ];
        app.groups_loaded = true;
        app.group_state.select(Some(0));
        println!("\n===== GROUPS SCREEN (G) =====");
        println!("{}", dump(&mut app, 100, 24));
    }

    #[test]
    fn render_brand_smoke() {
        let mut app = demo_app();

        app.screen = Screen::EnvSelect;
        println!("\n===== LANDING (env select) =====");
        println!("{}", dump(&mut app, 90, 20));

        app.screen = Screen::Main;
        app.toast = Some(crate::app::Toast {
            message: "copied payload (31 bytes)".into(),
            level: crate::app::ToastLevel::Success,
            born: std::time::Instant::now(),
        });
        println!("\n===== HEADER + TOAST =====");
        println!("{}", dump(&mut app, 104, 20));
    }

    #[test]
    fn render_peek_smoke() {
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.modal = Modal::Peek {
            records: vec![
                EventRecord {
                    partition: 0,
                    offset: 148231,
                    key: "user-42".into(),
                    payload: r#"{"event":"click","x":10,"y":20}"#.into(),
                    timestamp: Some(1784405965782),
                },
                EventRecord {
                    partition: 1,
                    offset: 153230,
                    key: "user-7".into(),
                    payload: r#"{"event":"scroll","depth":3}"#.into(),
                    timestamp: Some(1784405969111),
                },
            ],
            sel: 0,
        };
        println!("\n===== PEEK (events + pretty payload, 108x26) =====");
        println!("{}", dump(&mut app, 108, 26));
    }

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }
}
