//! Rendering. Reads `App`, draws frames. Dense single-line header, plain
//! titled panels on a dark background, a keybinding footer, animated spinners,
//! and a bird's-eye dashboard: Topics, a topic Detail/Config flip pane, a live
//! events Graph, and Logs. The focused panel gets a cyan border; `z` zooms.
//!
//! Rendering is pure and never blocks: expensive data (watermarks, groups)
//! shows a "loading…" placeholder until the worker delivers it.

use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Padding, Paragraph, Row, Table, TableState,
    Wrap,
};
use ratatui::Frame;

use crate::app::{App, Modal, Screen, View};
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
    }

    if app.connecting.is_some() {
        render_connecting(frame, app);
    }
    render_modal(frame, app);
    render_toast(frame, app);
}

/// Transient top-right notification.
fn render_toast(frame: &mut Frame, app: &App) {
    let Some(t) = &app.toast else { return };
    let area = frame.area();
    let w = (area.width / 3)
        .clamp(24, 54)
        .min(area.width.saturating_sub(2));

    let (label, color) = match t.level {
        crate::app::ToastLevel::Info => (" INFO ", theme::ACCENT),
        crate::app::ToastLevel::Success => (" OK ", theme::SUCCESS),
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
    // No background fill - transparent outline on the app's dark bg.
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
    if !status.is_empty() {
        spans.push(Span::styled(
            format!("│  {status}"),
            Style::default().fg(theme::ACCENT_LIGHT),
        ));
    }
    let used = Line::from(spans.clone()).width() as u16;
    // Transparent (no filled bar) - sits on the app bg.
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::APP_BG)),
        area,
    );

    // Brand + version, bottom-right, muted - only when it doesn't collide.
    let brand = format!("{} v{} ", theme::NAME, theme::VERSION);
    let bw = brand.chars().count() as u16;
    if area.width > used + bw + 2 {
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
    let block_h = (theme::WORDMARK.len() as u16 + 4) + (n + 2);
    let area = centered_fixed(
        58,
        block_h.min(frame.area().height.saturating_sub(2)),
        frame.area(),
    );

    let rows = Layout::vertical([
        Constraint::Length(theme::WORDMARK.len() as u16 + 3), // wordmark + tagline
        Constraint::Min(1),                                   // env list
    ])
    .split(area);

    // ── Branded masthead ──
    frame.render_widget(Clear, area);
    let mut brand_lines = vec![Line::from("")];
    for w in theme::WORDMARK {
        brand_lines.push(Line::from(Span::styled(
            *w,
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
    }
    brand_lines.push(Line::from(Span::styled(
        format!("  {}", theme::TAGLINE),
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
            // Same-width placeholder keeps the host column aligned.
            spans.push(if e.prod {
                Span::styled(
                    " PROD ",
                    Style::default()
                        .fg(theme::PANEL_BG)
                        .bg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("      ")
            });
            // 56 inner cols - "▶ " - "n·" - name(12) - badge(6) - gap(2) = 32.
            spans.push(Span::styled(
                format!("  {}", truncate(&host_only(&e.bootstrap), 32)),
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
        if app.connected.is_some() {
            &[
                ("↑↓", "select"),
                ("↵ / 1-9", "connect"),
                ("esc", "back"),
                ("q", "quit"),
            ]
        } else {
            &[("↑↓", "select"), ("↵ / 1-9", "connect"), ("q", "quit")]
        },
        "",
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

// ── Main screen: [list | detail] for the active view ─────────────────────

fn render_main(frame: &mut Frame, app: &mut App) {
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, head, app);

    let list_w = (body.width * 36 / 100).clamp(30, 56);
    let [left, right] =
        Layout::horizontal([Constraint::Length(list_w), Constraint::Min(1)]).areas(body);
    match app.view {
        View::Topics => {
            render_topic_list(frame, left, app);
            render_topic_detail(frame, right, app);
        }
        View::Groups => {
            render_group_list(frame, left, app);
            render_group_detail(frame, right, app);
        }
    }

    let hints: &[(&str, &str)] = match app.view {
        View::Topics => &[
            ("↑↓", "move"),
            ("↵", "messages"),
            ("/", "filter"),
            ("⇥", "groups"),
            ("c", "create"),
            ("x", "more"),
            ("?", "help"),
        ],
        View::Groups => &[
            ("↑↓", "move"),
            ("↵", "open topic"),
            ("/", "filter"),
            ("⇥", "topics"),
            ("d", "delete"),
            ("x", "more"),
            ("?", "help"),
        ],
    };
    footer(frame, foot, None, hints, &app.status);
}

/// Tabs on the left, environments on the right (active one filled; prod red).
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let tab = |label: String, active: bool| {
        if active {
            Span::styled(
                format!(" {label} "),
                Style::default()
                    .bg(theme::ACCENT)
                    .fg(theme::PANEL_BG)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!(" {label} "), Style::default().fg(theme::TEXT_MUTED))
        }
    };
    let groups = if app.groups_loaded {
        app.groups.len().to_string()
    } else {
        "…".into()
    };
    let tabs = Line::from(vec![
        Span::raw(" "),
        tab(
            format!("Topics {}", app.topic_count()),
            app.view == View::Topics,
        ),
        Span::raw(" "),
        tab(
            format!("Consumer groups {groups}"),
            app.view == View::Groups,
        ),
    ]);
    frame.render_widget(Paragraph::new(tabs), area);

    let active = app.current_env_index();
    let mut envs: Vec<Span> = Vec::new();
    for (i, e) in app.config.envs.iter().enumerate().take(9) {
        // ● marks the active env in text too, not only by colour.
        let label = format!(
            " {}{} {}{} ",
            if active == Some(i) { "●" } else { " " },
            i + 1,
            e.name,
            if e.prod { " PROD" } else { "" }
        );
        let style = match (active == Some(i), e.prod) {
            (true, true) => Style::default()
                .bg(theme::ERROR)
                .fg(theme::PANEL_BG)
                .add_modifier(Modifier::BOLD),
            (true, false) => Style::default()
                .fg(theme::SUCCESS)
                .add_modifier(Modifier::BOLD),
            (false, true) => Style::default().fg(theme::ERROR),
            (false, false) => Style::default().fg(theme::TEXT_MUTED),
        };
        envs.push(Span::styled(label, style));
    }
    // Only as many envs as fit after the tabs; the active one always shows.
    let room = area.width.saturating_sub(48) as usize;
    let mut used = 0;
    let shown: Vec<Span> = envs
        .into_iter()
        .enumerate()
        .filter(|(i, s)| {
            used += s.width();
            used <= room || active == Some(*i)
        })
        .map(|(_, s)| s)
        .collect();
    frame.render_widget(
        Paragraph::new(Line::from(shown)).alignment(Alignment::Right),
        area,
    );
}

fn list_title(app: &App, noun: &str, total: usize) -> String {
    if app.filtering || !app.filter.is_empty() {
        format!(
            "{noun} · /{}{}",
            app.filter,
            if app.filtering { "▌" } else { "" }
        )
    } else {
        format!("{noun} · {total}")
    }
}

fn empty_list(frame: &mut Frame, area: Rect, title: &str, msg: String) {
    frame.render_widget(
        Paragraph::new(msg)
            .style(Style::default().fg(theme::TEXT_MUTED))
            .block(panel(title, true)),
        area,
    );
}

fn render_topic_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.filtered_topics();
    let title = list_title(app, "Topics", app.topic_count());
    if visible.is_empty() {
        let msg = if app.filter.is_empty() {
            "\n  this cluster has no topics yet\n\n  c  create one".to_string()
        } else {
            format!(
                "\n  no topics match \"{}\"\n  esc clears the filter",
                app.filter
            )
        };
        return empty_list(frame, area, &title, msg);
    }
    // borders 2 + highlight 1 + count column 9 + gap 1
    let name_w = area.width.saturating_sub(13) as usize;
    let rows: Vec<Row> = visible
        .iter()
        .map(|&i| {
            let t = &app.meta[i];
            let count = app
                .counts
                .get(&t.name)
                .map(|c| fmt_count(*c))
                .unwrap_or_default();
            Row::new(vec![
                Span::styled(truncate(&t.name, name_w), Style::default().fg(theme::TEXT)),
                Span::styled(
                    format!("{count:>9}"),
                    Style::default().fg(theme::TEXT_MUTED),
                ),
            ])
        })
        .collect();
    let table = Table::new(rows, [Constraint::Min(10), Constraint::Length(9)])
        .block(panel(&title, true))
        .row_highlight_style(
            Style::default()
                .bg(theme::ROW_SELECTED_BG)
                .fg(theme::ROW_SELECTED_FG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌");
    let mut state = TableState::default().with_selected(app.topic_state.selected());
    *state.offset_mut() = app.topic_state.offset();
    frame.render_stateful_widget(table, area, &mut state);
    *app.topic_state.offset_mut() = state.offset();
}

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default()
            .fg(theme::TEXT_MUTED)
            .add_modifier(Modifier::BOLD),
    ))
}

fn loading_line(app: &App, what: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {} {what}", spinner(app)),
        Style::default().fg(theme::WARNING),
    ))
}

fn render_topic_detail(frame: &mut Frame, area: Rect, app: &App) {
    let Some(d) = &app.detail else {
        return empty_list(frame, area, "Topic", "\n  no topic selected".into());
    };
    let muted = Style::default().fg(theme::TEXT_MUTED);
    let strong = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);

    // ── Headline numbers ──
    let messages = if d.watermarks_loaded {
        Span::styled(fmt_count(d.total_messages()), strong)
    } else {
        Span::styled(
            format!("{} ", spinner(app)),
            Style::default().fg(theme::WARNING),
        )
    };
    let rf = d.partitions.first().map_or(0, |p| p.replicas);
    let under = d.partitions.iter().filter(|p| p.isr < p.replicas).count();
    let mut stats = vec![
        Span::raw("  "),
        messages,
        Span::styled(" messages   ", muted),
        Span::styled(d.partitions.len().to_string(), strong),
        Span::styled(" partitions   ", muted),
        Span::styled(format!("RF {rf}"), strong),
    ];
    if under > 0 {
        stats.push(Span::styled(
            format!("   ⚠ {under} under-replicated"),
            Style::default()
                .fg(theme::WARNING)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if app.rate.iter().all(|&r| r == 0) {
        if !app.rate.is_empty() {
            stats.push(Span::styled("   idle", muted));
        }
    } else if let Some(now) = app.rate.last() {
        stats.push(Span::styled("   ", muted));
        stats.push(Span::styled(
            format!("{} msg/s ", fmt_count(*now as i64)),
            Style::default()
                .fg(theme::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ));
        stats.push(Span::styled(
            spark(&app.rate, 24),
            Style::default().fg(theme::ACCENT),
        ));
    }
    let mut lines = vec![Line::from(""), Line::from(stats)];

    // ── Config, in human units ──
    match &app.topic_config {
        Some((t, entries)) if *t == d.name => {
            let get = |k: &str| {
                entries
                    .iter()
                    .find(|(key, _)| key == k)
                    .map(|(_, v)| v.as_str())
            };
            let mut parts: Vec<String> = Vec::new();
            if let Some(v) = get("retention.ms") {
                parts.push(format!("retention {}", human_ms(v)));
            }
            if let Some(v) = get("retention.bytes").filter(|v| *v != "-1") {
                parts.push(format!("max size {}", human_bytes(v)));
            }
            if let Some(v) = get("cleanup.policy") {
                parts.push(format!("cleanup {v}"));
            }
            if let Some(v) = get("min.insync.replicas") {
                parts.push(format!("min ISR {v}"));
            }
            if let Some(v) = get("max.message.bytes") {
                parts.push(format!("max message {}", human_bytes(v)));
            }
            if let Some(v) = get("compression.type").filter(|v| *v != "producer") {
                parts.push(format!("compression {v}"));
            }
            if parts.is_empty() {
                // e.g. "(unavailable)" when DescribeConfigs isn't permitted
                parts = entries.iter().map(|(k, v)| format!("{k} {v}")).collect();
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", parts.join(" · ")),
                muted,
            )));
        }
        _ => lines.push(loading_line(app, "loading config…")),
    }

    // ── Who reads it ──
    // Name column takes whatever the fixed columns leave (borders 2, indent 2).
    let inner = area.width.saturating_sub(4) as usize;
    let gname_w = inner.saturating_sub(9 + 10).max(12);
    lines.push(Line::from(""));
    lines.push(heading(&format!(
        "{:<gname_w$}{:>9}  state",
        "CONSUMER GROUPS", "lag"
    )));
    let consumers = app.consumers_of(&d.name);
    if !app.groups_loaded {
        lines.push(loading_line(app, "loading consumer groups…"));
    } else if consumers.is_empty() {
        lines.push(Line::from(Span::styled(
            "  none - no group has read this topic",
            muted,
        )));
    }
    for (g, lag) in consumers {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<gname_w$}", truncate(&g.name, gname_w)),
                Style::default().fg(theme::TEXT),
            ),
            lag_span(lag, 9),
            Span::styled(format!("  {}", g.state), state_style(&g.state)),
        ]));
    }

    // ── Partitions ── (narrow panes drop "first offset", the least useful)
    let wide = inner >= 70;
    lines.push(Line::from(""));
    lines.push(heading(&if wide {
        format!(
            "{:<10}{:>10}{:>18}{:>16}{:>14}",
            "PARTITION", "in-sync", "first offset", "next offset", "messages"
        )
    } else {
        format!(
            "{:<6}{:>9}{:>14}{:>13}",
            "PART", "in-sync", "next offset", "messages"
        )
    }));
    let cell = |v: i64| {
        if v < 0 {
            "…".to_string()
        } else {
            fmt_count(v)
        }
    };
    for p in &d.partitions {
        let isr_style = if p.isr < p.replicas {
            Style::default().fg(theme::WARNING)
        } else {
            muted
        };
        let msgs = if p.high < 0 { -1 } else { p.high - p.low };
        let isr = format!("{}/{}", p.isr, p.replicas);
        let text = Style::default().fg(theme::TEXT);
        lines.push(Line::from(if wide {
            vec![
                Span::styled(format!("  {:<10}", p.id), text),
                Span::styled(format!("{isr:>10}"), isr_style),
                Span::styled(format!("{:>18}", cell(p.low)), muted),
                Span::styled(format!("{:>16}", cell(p.high)), muted),
                Span::styled(format!("{:>14}", cell(msgs)), text),
            ]
        } else {
            vec![
                Span::styled(format!("  {:<6}", p.id), text),
                Span::styled(format!("{isr:>9}"), isr_style),
                Span::styled(format!("{:>14}", cell(p.high)), muted),
                Span::styled(format!("{:>13}", cell(msgs)), text),
            ]
        }));
    }

    let title = truncate(&d.name, area.width.saturating_sub(6) as usize);
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(&title, false))
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0)),
        area,
    );
}

fn state_style(state: &str) -> Style {
    Style::default().fg(match state {
        "Stable" => theme::SUCCESS,
        "Empty" | "Dead" => theme::INACTIVE,
        _ => theme::WARNING,
    })
}

/// Right-aligned lag; amber when non-zero, "…" while loading, "-" if none.
fn lag_span(lag: Option<i64>, width: usize) -> Span<'static> {
    match lag {
        Some(0) => Span::styled(
            format!("{:>width$}", 0),
            Style::default().fg(theme::SUCCESS),
        ),
        Some(n) => Span::styled(
            format!("{:>width$}", fmt_count(n)),
            Style::default()
                .fg(theme::WARNING)
                .add_modifier(Modifier::BOLD),
        ),
        None => Span::styled(
            format!("{:>width$}", "-"),
            Style::default().fg(theme::TEXT_MUTED),
        ),
    }
}

fn render_group_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = list_title(app, "Consumer groups", app.groups.len());
    if !app.groups_loaded {
        return empty_list(
            frame,
            area,
            &title,
            format!("\n  {} loading consumer groups…", spinner(app)),
        );
    }
    let visible = app.filtered_groups();
    if visible.is_empty() {
        let msg = if app.filter.is_empty() {
            "\n  no consumer groups on this cluster".to_string()
        } else {
            format!(
                "\n  no groups match \"{}\"\n  esc clears the filter",
                app.filter
            )
        };
        return empty_list(frame, area, &title, msg);
    }
    let name_w = area.width.saturating_sub(12) as usize;
    let rows: Vec<Row> = visible
        .iter()
        .map(|&i| {
            let g = &app.groups[i];
            let lag = if app.lags.contains_key(&g.name) {
                lag_span(app.group_lag(&g.name), 8)
            } else {
                Span::styled(
                    format!("{:>8}", spinner(app)),
                    Style::default().fg(theme::TEXT_MUTED),
                )
            };
            Row::new(vec![
                Span::styled(truncate(&g.name, name_w), Style::default().fg(theme::TEXT)),
                lag,
            ])
        })
        .collect();
    let table = Table::new(rows, [Constraint::Min(10), Constraint::Length(8)])
        .header(Row::new(vec![
            Span::raw(""),
            Span::styled("     lag", Style::default().fg(theme::TEXT_MUTED)),
        ]))
        .block(panel(&title, true))
        .row_highlight_style(
            Style::default()
                .bg(theme::ROW_SELECTED_BG)
                .fg(theme::ROW_SELECTED_FG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌");
    frame.render_stateful_widget(table, area, &mut app.group_state);
}

fn render_group_detail(frame: &mut Frame, area: Rect, app: &App) {
    let Some(g) = app.selected_group() else {
        return empty_list(frame, area, "Group", String::new());
    };
    let muted = Style::default().fg(theme::TEXT_MUTED);
    let strong = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);
    let lag = app.group_lag(&g.name);
    let mut summary = vec![
        Span::raw("  "),
        Span::styled(
            g.state.clone(),
            state_style(&g.state).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ", muted),
        Span::styled(g.members.to_string(), strong),
        Span::styled(
            if g.members == 1 {
                " member"
            } else {
                " members"
            },
            muted,
        ),
        Span::styled("   lag ", muted),
        lag_span(lag, 0),
    ];
    if !g.protocol.is_empty() {
        summary.push(Span::styled(format!("   {} assignor", g.protocol), muted));
    }
    let mut lines = vec![Line::from(""), Line::from(summary)];
    if g.members == 0 && lag.unwrap_or(0) > 0 {
        lines.push(Line::from(Span::styled(
            "  ⚠ no active members - nothing is consuming this group",
            Style::default().fg(theme::WARNING),
        )));
    }
    let topics = app.group_topics(g);
    lines.push(Line::from(vec![
        Span::styled("  reads  ", muted),
        Span::styled(
            if topics.is_empty() {
                "-".to_string()
            } else {
                topics.join(", ")
            },
            Style::default().fg(theme::ACCENT_LIGHT),
        ),
    ]));

    // Narrow panes drop the "end" column; lag already says how far behind.
    let inner = area.width.saturating_sub(4) as usize;
    let wide = inner >= 64;
    let topic_w = if wide {
        inner.saturating_sub(10 + 13 + 13 + 10 + 1)
    } else {
        inner.saturating_sub(6 + 12 + 10 + 1)
    }
    .max(10);
    lines.push(Line::from(""));
    lines.push(heading(&if wide {
        format!(
            "{:<topic_w$}{:>10}{:>13}{:>13}{:>10}",
            "TOPIC", "partition", "committed", "end", "lag"
        )
    } else {
        format!(
            "{:<topic_w$}{:>6}{:>12}{:>10}",
            "TOPIC", "part", "committed", "lag"
        )
    }));
    match app.lags.get(&g.name) {
        None => lines.push(loading_line(app, "loading offsets…")),
        Some(parts) if parts.is_empty() => lines.push(Line::from(Span::styled(
            "  no committed offsets yet",
            muted,
        ))),
        Some(parts) => {
            let mut parts: Vec<_> = parts.iter().collect();
            parts.sort_by(|a, b| {
                b.lag()
                    .cmp(&a.lag())
                    .then(a.topic.cmp(&b.topic))
                    .then(a.partition.cmp(&b.partition))
            });
            for p in parts {
                let topic = Span::styled(
                    format!("  {:<topic_w$}", truncate(&p.topic, topic_w)),
                    Style::default().fg(theme::TEXT),
                );
                lines.push(Line::from(if wide {
                    vec![
                        topic,
                        Span::styled(format!("{:>10}", p.partition), muted),
                        Span::styled(format!("{:>13}", fmt_count(p.committed)), muted),
                        Span::styled(format!("{:>13}", fmt_count(p.end)), muted),
                        lag_span(Some(p.lag()), 10),
                    ]
                } else {
                    vec![
                        topic,
                        Span::styled(format!("{:>6}", p.partition), muted),
                        Span::styled(format!("{:>12}", fmt_count(p.committed)), muted),
                        lag_span(Some(p.lag()), 10),
                    ]
                }));
            }
        }
    }
    let title = truncate(&g.name, area.width.saturating_sub(6) as usize);
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(&title, false))
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0)),
        area,
    );
}

/// Text sparkline of the last `n` samples (one line, no widget needed).
fn spark(v: &[u64], n: usize) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let tail = &v[v.len().saturating_sub(n)..];
    let max = tail.iter().copied().max().unwrap_or(0).max(1);
    tail.iter()
        .map(|&x| BARS[((x * 7) / max) as usize])
        .collect()
}

/// "604800000" (ms) → "7d"; "-1" → "forever".
fn human_ms(v: &str) -> String {
    let Ok(ms) = v.parse::<i64>() else {
        return v.to_string();
    };
    if ms < 0 {
        return "forever".into();
    }
    let s = ms / 1000;
    match s {
        _ if s >= 86_400 && s % 86_400 == 0 => format!("{}d", s / 86_400),
        _ if s >= 3_600 && s % 3_600 == 0 => format!("{}h", s / 3_600),
        _ if s >= 60 && s % 60 == 0 => format!("{}m", s / 60),
        _ if ms % 1000 == 0 => format!("{s}s"),
        _ => format!("{ms}ms"),
    }
}

/// "1048588" → "1.0 MiB"; "-1" → "unlimited".
fn human_bytes(v: &str) -> String {
    let Ok(b) = v.parse::<i64>() else {
        return v.to_string();
    };
    if b < 0 {
        return "unlimited".into();
    }
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut x = b as f64;
    let mut u = 0;
    while x >= 1024.0 && u < units.len() - 1 {
        x /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{b} B")
    } else {
        format!("{x:.1} {}", units[u])
    }
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
                "Keys",
                vec![
                    Line::from(""),
                    head("Everywhere"),
                    row("⇥", "switch Topics ⟷ Consumer groups"),
                    row("↑↓ j k", "move · g / End top / bottom"),
                    row("PgUp PgDn", "scroll the detail on the right"),
                    row("/", "filter the list · esc clears"),
                    row("r", "refresh from the cluster"),
                    row("1–9 · e", "switch environment · picker"),
                    row("x", "all actions for this view"),
                    row("L", "activity log"),
                    Line::from(""),
                    head("Topics"),
                    row("↵ / m", "latest messages (r reload · y copy)"),
                    row("c a d", "create · add partitions · delete"),
                    row("y", "copy topic name"),
                    Line::from(""),
                    head("Consumer groups"),
                    row("↵", "open the topic the group reads"),
                    row("d · y", "delete group · copy name"),
                    Line::from(""),
                    row("q · ctrl-c", "quit"),
                ],
                theme::ACCENT,
                25,
            );
        }
        Modal::Error(msg) => {
            let mut lines = vec![Line::from("")];
            lines.extend(msg.lines().map(|l| {
                Line::from(Span::styled(
                    l.to_string(),
                    Style::default().fg(theme::ERROR),
                ))
            }));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "press any key to dismiss",
                Style::default().fg(theme::TEXT_MUTED),
            )));
            // Grow with the wrapped message so long errors aren't cut off
            // (inner width = popup minus borders and padding).
            let inner = popup_width(frame.area()).saturating_sub(4);
            let rows = Paragraph::new(lines.clone())
                .wrap(Wrap { trim: false })
                .line_count(inner) as u16;
            popup(frame, "Error", lines, theme::ERROR, rows + 2);
        }
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
            let mut lines = vec![
                Line::from(""),
                field("name", &f.name, f.focus == 0),
                field("partitions", &f.partitions, f.focus == 1),
                field("replication", &f.replication, f.focus == 2),
            ];
            push_form_error(&mut lines, &f.error);
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ⇥ next   ↵ create   esc cancel",
                Style::default().fg(theme::TEXT_MUTED),
            )));
            let h = lines.len() as u16 + 2;
            popup(frame, "Create topic", lines, theme::ACCENT, h);
        }
        Modal::AddPartitions(f) => {
            let input = |val: &str, focused: bool| match (val.is_empty(), focused) {
                // A whitespace-only line trips ratatui's word-wrapper and drops
                // the next line, so an empty unfocused field shows a hint.
                (true, false) => Span::styled("⇥ to type", Style::default().fg(theme::TEXT_MUTED)),
                _ => Span::styled(
                    format!("{val}{}", if focused { "▌" } else { "" }),
                    Style::default()
                        .fg(theme::ACCENT_LIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
            };
            let muted = Style::default().fg(theme::TEXT_MUTED);
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  topic  ", muted),
                    Span::styled(f.topic.clone(), Style::default().fg(theme::ACCENT_LIGHT)),
                    Span::styled(format!("   (now {} partitions)", f.current), muted),
                ]),
                Line::from(vec![
                    Span::styled("  total  ", muted),
                    input(&f.total, f.focus == 0),
                ]),
            ];
            if f.is_prod {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  ⚠ PROD - irreversible. Type the topic name to confirm:",
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    input(&f.confirm, f.focus == 1),
                ]));
            }
            push_form_error(&mut lines, &f.error);
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if f.is_prod {
                    "  partitions can only increase   ⇥ next   ↵ apply   esc cancel"
                } else {
                    "  partitions can only increase   ↵ apply   esc cancel"
                },
                Style::default().fg(theme::WARNING),
            )));
            let h = lines.len() as u16 + 2;
            let accent = if f.is_prod {
                theme::ERROR
            } else {
                theme::ACCENT
            };
            popup(frame, "Add partitions", lines, accent, h);
        }
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
            // Kafka refuses to delete a group with live members; say so up front.
            let members = app
                .groups
                .iter()
                .find(|g| matches!(f.kind, crate::app::DeleteKind::Group) && g.name == f.target)
                .map_or(0, |g| g.members);
            if members > 0 {
                lines.push(Line::from(Span::styled(
                    format!("  ⚠ {members} active member(s) - Kafka will refuse until they stop"),
                    Style::default().fg(theme::WARNING),
                )));
                lines.push(Line::from(""));
            }
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
            push_form_error(&mut lines, &f.error);
            let h = lines.len() as u16 + 2;
            popup(frame, &format!("Delete {noun}"), lines, theme::ERROR, h);
        }
        Modal::Peek {
            topic,
            records,
            sel,
            scroll,
        } => render_peek(frame, topic, records, *sel, *scroll),
        Modal::Logs(back) => {
            let a = frame.area();
            let h = (a.height * 80 / 100).max(8);
            let inner_h = h.saturating_sub(4) as usize;
            let end = app.logs.len().saturating_sub(*back as usize);
            let start = end.saturating_sub(inner_h);
            let mut lines: Vec<Line> = app.logs[start..end]
                .iter()
                .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(theme::TEXT))))
                .collect();
            if lines.is_empty() {
                lines.push(Line::from(Span::styled(
                    "no activity yet",
                    Style::default().fg(theme::TEXT_MUTED),
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "↑↓ / PgUp PgDn scroll · any other key closes",
                Style::default().fg(theme::TEXT_MUTED),
            )));
            popup(frame, "Activity log", lines, theme::ACCENT, h);
        }
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
fn render_peek(
    frame: &mut Frame,
    topic: &str,
    records: &[crate::kafka::EventRecord],
    sel: usize,
    scroll: u16,
) {
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
            format!(" {topic} · {} newest messages ", records.len()),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " ↑↓ select · PgUp/PgDn scroll payload · y copy · Y copy key · r reload · esc close ",
            Style::default().fg(theme::TEXT_MUTED),
        ))
        .style(Style::default().bg(theme::PANEL_BG));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if records.is_empty() {
        frame.render_widget(
            Paragraph::new("  no messages in this topic yet")
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
                format!(
                    "{}  ",
                    r.timestamp
                        .map_or_else(|| "--:--:--".into(), |t| crate::app::local_time(t, false))
                ),
                base.fg(theme::TEXT_MUTED),
            ),
            Span::styled(
                format!("{:<12}", format!("p{} @{}", r.partition, r.offset)),
                base.fg(theme::ACCENT),
            ),
            Span::styled(
                format!(
                    "{:<18}",
                    truncate(if r.key.is_empty() { "∅" } else { &r.key }, 16)
                ),
                base.fg(theme::WARNING),
            ),
            Span::styled(
                truncate(&r.payload, (parts[0].width as usize).saturating_sub(45)),
                base.fg(theme::TEXT),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(list_lines), parts[0]);

    // ── Selected payload (pretty-printed if JSON) ──
    let r = &records[sel];
    let ts = r
        .timestamp
        .map(|t| crate::app::local_time(t, true))
        .unwrap_or_else(|| "-".into());
    let body: Vec<Line> = pretty_json(&r.payload)
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_string(),
                Style::default().fg(theme::TEXT),
            ))
        })
        .collect();
    let [head, rest] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(parts[1]);
    let body = Paragraph::new(body).wrap(Wrap { trim: false });
    // Long payloads scroll instead of being silently cut off.
    let total = body.line_count(rest.width) as u16;
    let max_scroll = total.saturating_sub(rest.height);
    let scroll = scroll.min(max_scroll);
    let more = if max_scroll > 0 {
        format!(
            " · lines {}-{} of {total} (PgDn/PgUp)",
            scroll + 1,
            (scroll + rest.height).min(total)
        )
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("─ payload  ", Style::default().fg(theme::SEPARATOR)),
            Span::styled(
                format!(
                    "partition {} · offset {} · ts {} · key {}{more}",
                    r.partition,
                    r.offset,
                    ts,
                    if r.key.is_empty() { "∅" } else { &r.key }
                ),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ])),
        head,
    );
    frame.render_widget(body.scroll((scroll, 0)), rest);
}

/// Pretty-print a payload as JSON if it parses; otherwise return it verbatim.
fn pretty_json(s: &str) -> String {
    serde_json::from_str::<serde_json::Value>(s)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or_else(|_| s.to_string())
}

/// Inline validation message for a form that stays open.
fn push_form_error(lines: &mut Vec<Line>, error: &Option<String>) {
    if let Some(e) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  ✗ {e}"),
            Style::default()
                .fg(theme::ERROR)
                .add_modifier(Modifier::BOLD),
        )));
    }
}

fn popup_width(a: Rect) -> u16 {
    (a.width * 7 / 10).clamp(40, 96)
}

/// Centered modal. Height grows to fit the wrapped content (`rows` is a
/// minimum), so a long name never pushes the key hints out of view.
fn popup(frame: &mut Frame, title: &str, lines: Vec<Line>, accent: Color, rows: u16) {
    let a = frame.area();
    let w = popup_width(a);
    // inner width = borders 2 + padding 2
    let wrapped = Paragraph::new(lines.clone())
        .wrap(Wrap { trim: false })
        .line_count(w.saturating_sub(4)) as u16;
    let h = (wrapped + 2)
        .max(rows)
        .min(a.height.saturating_sub(2))
        .max(5);
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
        .padding(Padding::horizontal(1))
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
    use crate::app::{App, Modal, Screen};
    use crate::config::{Config, EnvProfile};
    use crate::kafka::{EventRecord, PartMeta, PartitionInfo, TopicDetail, TopicMeta};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn env(name: &str, prod: bool) -> EnvProfile {
        EnvProfile {
            name: name.into(),
            bootstrap: format!("b-1.{name}.xxxx.c2.kafka.eu-central-1.amazonaws.com:9092"),
            region: "eu-central-1".into(),
            auth: crate::config::Auth::Plaintext,
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
        let out = dump(&mut app, 120, 30);
        println!("\n===== TOPICS (120x30) =====\n{out}");
        for want in [
            "Topics 12",
            "messages",
            "retention 7d",
            "CONSUMER GROUPS",
            "billing-consumer",
            "PARTITION",
        ] {
            assert!(out.contains(want), "missing {want:?}");
        }
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
        app.view = crate::app::View::Groups;
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
        app.lags.insert(
            "billing-consumer".into(),
            vec![crate::kafka::PartitionLag {
                topic: "service.events.v2".into(),
                partition: 3,
                committed: 900,
                end: 1200,
            }],
        );
        let out = dump(&mut app, 120, 24);
        println!("\n===== GROUPS VIEW =====\n{out}");
        assert!(
            out.contains("300") && out.contains("committed"),
            "lag not shown"
        );
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
            topic: "service.events.v2".into(),
            scroll: 0,
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

    #[test]
    fn ctrl_c_quits_even_while_filtering() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.on_key(KeyEvent::from(KeyCode::Char('/'))).unwrap();
        assert!(app.filtering);
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
            .unwrap();
        assert!(app.should_quit);
        assert_eq!(app.filter, "");
    }

    fn press(app: &mut App, keys: &str) {
        use crossterm::event::{KeyCode, KeyEvent};
        for c in keys.chars() {
            let code = match c {
                '\n' => KeyCode::Enter,
                '\t' => KeyCode::Tab,
                c => KeyCode::Char(c),
            };
            app.on_key(KeyEvent::from(code)).unwrap();
        }
    }

    #[test]
    fn prod_mismatch_keeps_dialog_open_so_keys_cannot_reach_dashboard() {
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.connected = Some(env("prod", true));
        // Wrong confirmation, then keys that would be dashboard actions.
        press(&mut app, "d\nq");
        assert!(matches!(app.modal, Modal::Delete(ref f) if f.error.is_some()));
        assert!(!app.should_quit);
    }

    #[test]
    fn prod_add_partitions_needs_typed_name_and_must_increase() {
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.connected = Some(env("prod", true));
        // service.events.v2 has 5 partitions in demo_app.
        press(&mut app, "a3\n");
        assert!(
            matches!(app.modal, Modal::AddPartitions(ref f) if f.error.as_deref().unwrap().contains("current 5"))
        );
        press(&mut app, "\u{8}");
        let Modal::AddPartitions(f) = &mut app.modal else {
            panic!()
        };
        f.total = "8".into();
        press(&mut app, "\n");
        assert!(
            matches!(app.modal, Modal::AddPartitions(ref f) if f.focus == 1 && f.error.is_some())
        );
        press(&mut app, "service.events.v2\n");
        assert!(matches!(app.modal, Modal::None));
    }

    #[test]
    fn create_defaults_replication_to_broker_count() {
        let mut app = demo_app();
        app.screen = Screen::Main;
        app.brokers = 2;
        press(&mut app, "c");
        assert!(matches!(app.modal, Modal::Create(ref f) if f.replication == "2"));
    }

    #[test]
    fn pretty_json_keeps_original_key_order() {
        let out = pretty_json(r#"{"id":1,"amount":2,"currency":"EUR"}"#);
        let pos = |k: &str| out.find(k).unwrap();
        assert!(
            pos("id") < pos("amount") && pos("amount") < pos("currency"),
            "{out}"
        );
    }
}
