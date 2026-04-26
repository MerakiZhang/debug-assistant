use super::state::{FlasherMethod, FlasherState, FlasherSubScreen};
use crate::features::protocols::{jtag, swd, uart};
use crate::ui::theme;
use ratatui::{
    layout::{Alignment, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render(frame: &mut Frame, state: &FlasherState) {
    match state.method {
        FlasherMethod::UsartIsp => uart::isp::ui::render(frame, state),
        FlasherMethod::Jtag => jtag::flash::ui::render(frame, state),
        FlasherMethod::Swd => swd::flash::ui::render(frame, state),
    }
}

pub(crate) fn render_workbench_title(
    frame: &mut Frame,
    area: Rect,
    state: &FlasherState,
    title: &str,
) {
    let title_color = if state.sub_screen == FlasherSubScreen::Progress {
        if state.op_done {
            if state.op_ok {
                theme::SUCCESS
            } else {
                theme::DANGER
            }
        } else {
            theme::WARNING
        }
    } else {
        theme::ACCENT
    };
    let status = match state.sub_screen {
        FlasherSubScreen::Config => "Ready",
        FlasherSubScreen::Progress if state.op_done && state.op_ok => "Complete",
        FlasherSubScreen::Progress if state.op_done => "Failed",
        FlasherSubScreen::Progress => "Running",
    };
    frame.render_widget(
        Paragraph::new(format!(" {} | STATUS: {} ", title, status))
            .style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(title_color)),
            ),
        area,
    );
}

pub(crate) fn setup_cycle_line(
    label: &'static str,
    value: &str,
    focused: bool,
    style: Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<7}", label), theme::title_style()),
        Span::styled(if focused { "◄ ► " } else { "" }, style),
        Span::styled(value.to_string(), style),
    ])
}

pub(crate) fn setup_value_line(
    label: &'static str,
    value: &str,
    focused: bool,
    style: Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<7}", label), theme::title_style()),
        Span::styled(if focused { "▌ " } else { "" }, style),
        Span::styled(value.to_string(), style),
    ])
}

pub(crate) fn render_firmware_file_input(
    frame: &mut Frame,
    state: &FlasherState,
    area: Rect,
    file_path: &str,
    file_cursor: usize,
    focused: bool,
) {
    let active = state.sub_screen == FlasherSubScreen::Config;
    let border_color = if focused { theme::ACCENT } else { theme::MUTED };
    let block = Block::new()
        .title(" Firmware File (.bin/.hex) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prefix = "  ";
    let prefix_width = UnicodeWidthStr::width(prefix) as u16;
    let value_width = inner.width.saturating_sub(prefix_width).saturating_sub(1);
    let (value, cursor_col) = clipped_text_with_cursor(file_path, file_cursor, value_width);

    let value_style = if file_path.is_empty() {
        theme::muted_style()
    } else if focused {
        theme::selected_style()
    } else if active {
        Style::default()
    } else {
        theme::muted_style()
    };
    let display_value = if file_path.is_empty() {
        "Type firmware path here".to_string()
    } else {
        value
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(prefix),
            Span::styled(display_value, value_style),
        ])),
        inner,
    );

    if focused {
        let x = inner
            .x
            .saturating_add(prefix_width)
            .saturating_add(cursor_col)
            .min(inner.x + inner.width.saturating_sub(1));
        frame.set_cursor_position((x, inner.y));
    }
}

fn clipped_text_with_cursor(text: &str, cursor: usize, max_width: u16) -> (String, u16) {
    if max_width == 0 {
        return (String::new(), 0);
    }

    let cursor = cursor.min(text.len());
    let total_width = UnicodeWidthStr::width(text);
    let max_width = max_width as usize;
    if total_width <= max_width {
        return (
            text.to_string(),
            UnicodeWidthStr::width(&text[..cursor]) as u16,
        );
    }

    if max_width <= 3 {
        return (".".repeat(max_width), 0);
    }

    let cursor_width = UnicodeWidthStr::width(&text[..cursor]);
    let content_width = max_width - 3;

    if cursor_width <= content_width {
        let mut visible = take_width(text, content_width);
        visible.push_str("...");
        return (visible, cursor_width as u16);
    }

    let start_width = cursor_width.saturating_sub(content_width);
    let start = byte_index_at_width(text, start_width);
    let tail = take_width(&text[start..], content_width);
    let cursor_col = 3 + UnicodeWidthStr::width(&text[start..cursor]);
    (format!("...{}", tail), cursor_col as u16)
}

fn take_width(text: &str, max_width: usize) -> String {
    let mut used = 0;
    let mut out = String::new();
    for ch in text.chars() {
        let width = ch.width().unwrap_or(0);
        if used + width > max_width {
            break;
        }
        used += width;
        out.push(ch);
    }
    out
}

fn byte_index_at_width(text: &str, target_width: usize) -> usize {
    let mut used = 0;
    for (idx, ch) in text.char_indices() {
        if used >= target_width {
            return idx;
        }
        used += ch.width().unwrap_or(0);
    }
    text.len()
}

pub(crate) fn render_operation_log(
    frame: &mut Frame,
    state: &FlasherState,
    area: Rect,
    title: &'static str,
) {
    let log_block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::MUTED));
    let log_inner = log_block.inner(area);
    state.log_visible_rows.set(log_inner.height);
    let visible = log_inner.height as usize;

    let log_lines: Vec<Line<'static>> = if state.log.is_empty() {
        vec![Line::from(Span::styled(
            "  No operation yet. Press Enter to start flashing.",
            theme::muted_style(),
        ))]
    } else {
        state
            .log
            .iter()
            .map(|s| {
                Line::from(Span::styled(
                    format!("  {}", s),
                    Style::default().fg(Color::Gray),
                ))
            })
            .collect()
    };

    let total = log_lines.len();
    let scroll = state.log_scroll.min(total.saturating_sub(visible)) as u16;

    frame.render_widget(
        Paragraph::new(log_lines)
            .block(log_block)
            .scroll((scroll, 0)),
        area,
    );

    if total > visible {
        let mut sb = ScrollbarState::new(total.saturating_sub(visible)).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut sb,
        );
    }
}

pub(crate) fn render_progress_gauge(frame: &mut Frame, state: &FlasherState, area: Rect) {
    let (pct, gauge_label, gauge_color) = if state.sub_screen == FlasherSubScreen::Config {
        (0u16, " Ready ".to_string(), theme::ACCENT)
    } else if state.op_done {
        if state.op_ok {
            (100u16, " Complete ".to_string(), theme::SUCCESS)
        } else {
            (
                state.progress_pct.unwrap_or(0) as u16,
                " Failed ".to_string(),
                theme::DANGER,
            )
        }
    } else {
        let p = state.progress_pct.unwrap_or(0) as u16;
        (p, format!(" {}% ", p), theme::WARNING)
    };

    let gauge = Gauge::default()
        .block(
            Block::new()
                .title(" Progress ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(gauge_color)),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(theme::MUTED))
        .label(gauge_label)
        .percent(pct);
    frame.render_widget(gauge, area);
}
