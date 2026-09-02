use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;
use std::sync::{Mutex, OnceLock};

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Bar, BarChart, Block, Borders, Chart, Clear, Dataset, Gauge, GraphType, List, ListItem,
    ListState, Paragraph, Row, Sparkline, StatefulWidget, Table, TableState, Tabs, Widget, Wrap,
};
use unicode_width::UnicodeWidthStr;

mod commands;

const ABI_VERSION: u32 = 1;
const OK: i32 = 0;
const INVALID_ARGUMENT: i32 = -1;
const BUFFER_TOO_SMALL: i32 = -2;
const PANIC: i32 = -3;

static LAST_ERROR: OnceLock<Mutex<String>> = OnceLock::new();

fn last_error_slot() -> &'static Mutex<String> {
    LAST_ERROR.get_or_init(|| Mutex::new(String::new()))
}

fn set_last_error(message: impl Into<String>) {
    if let Ok(mut value) = last_error_slot().lock() {
        *value = message.into();
    }
}

unsafe fn utf8_arg<'a>(pointer: *const u8, length: usize, name: &str) -> Result<&'a str, String> {
    if length == 0 {
        return Ok("");
    }
    if pointer.is_null() {
        return Err(format!("{name} is NULL but its length is not zero"));
    }
    // SAFETY: The caller promises that `pointer` addresses `length` readable bytes.
    let bytes = unsafe { slice::from_raw_parts(pointer, length) };
    std::str::from_utf8(bytes).map_err(|error| format!("{name} is not valid UTF-8: {error}"))
}

fn render_dashboard(
    title: &str,
    body: &str,
    width: u16,
    height: u16,
    ansi: bool,
) -> Result<Vec<u8>, String> {
    if !(24..=500).contains(&width) {
        return Err("width must be between 24 and 500 cells".into());
    }
    if !(9..=200).contains(&height) {
        return Err("height must be between 9 and 200 cells".into());
    }

    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    Paragraph::new(" Harbour + Rust C ABI + Ratatui ")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .render(rows[0], &mut buffer);

    Paragraph::new(body)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
        .render(rows[1], &mut buffer);

    Gauge::default()
        .block(Block::default().title(" FFI status ").borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .ratio(1.0)
        .label("Ratatui rendering: OK")
        .render(rows[2], &mut buffer);

    Ok(buffer_to_text(&buffer, area, ansi))
}

struct ShowcaseOptions {
    tick: u32,
    selected: usize,
    table_selected: usize,
    focus: usize,
    menu: usize,
    menu_item: usize,
    checked_mask: u32,
    expanded_mask: u32,
    menu_open: bool,
    width: u16,
    height: u16,
    ansi: bool,
}

fn render_showcase(options: ShowcaseOptions) -> Result<Vec<u8>, String> {
    let ShowcaseOptions {
        tick,
        selected,
        table_selected,
        focus,
        menu,
        menu_item,
        checked_mask,
        expanded_mask,
        menu_open,
        width,
        height,
        ansi,
    } = options;
    if !(100..=240).contains(&width) {
        return Err("showcase width must be between 100 and 240 cells".into());
    }
    if !(32..=80).contains(&height) {
        return Err("showcase height must be between 32 and 80 cells".into());
    }

    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let phase = f64::from(tick) / 7.0;
    let pulse = ((phase.sin() + 1.0) * 0.5).clamp(0.0, 1.0);
    let cpu = (0.42 + pulse * 0.46).clamp(0.0, 1.0);
    let memory = (0.71 + (phase / 2.0).cos() * 0.08).clamp(0.0, 1.0);

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(23),
        Constraint::Length(3),
    ])
    .split(area);

    let header = Line::from(vec![
        Span::styled(" ◆ ", Style::default().fg(Color::Rgb(255, 104, 180))),
        Span::styled(
            "HARBOUR",
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ↔  ", Style::default().fg(Color::Rgb(130, 170, 255))),
        Span::styled(
            "RATATUI",
            Style::default()
                .fg(Color::Rgb(114, 239, 221))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("     live frame {tick:06}"),
            Style::default().fg(Color::Rgb(170, 180, 205)),
        ),
    ]);
    Paragraph::new(header)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(18, 24, 38)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(67, 217, 173))),
        )
        .render(rows[0], &mut buffer);

    Tabs::new([" File ", " View ", " Tools ", " Help "])
        .select(menu % 4)
        .divider(Span::styled(" • ", Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .render(rows[1], &mut buffer);

    let columns = Layout::horizontal([Constraint::Length(34), Constraint::Min(64)])
        .spacing(1)
        .split(rows[2]);
    let left = Layout::vertical([
        Constraint::Length(11),
        Constraint::Length(8),
        Constraint::Min(6),
    ])
    .split(columns[0]);

    let tree_active = focus % 2 == 0;
    let checked = |bit: u32| {
        if checked_mask & (1 << bit) != 0 {
            "☑"
        } else {
            "☐"
        }
    };
    let toolchains_expanded = expanded_mask & 0b01 != 0;
    let widgets_expanded = expanded_mask & 0b10 != 0;
    let mut tree_entries = vec![
        (0usize, "◆ Workspace".to_owned(), Color::LightCyan),
        (1, format!("  {} Harbour VM", checked(0)), Color::LightGreen),
        (
            2,
            format!(
                "  [{}] Toolchains",
                if toolchains_expanded { "-" } else { "+" }
            ),
            Color::LightCyan,
        ),
    ];
    if toolchains_expanded {
        tree_entries.extend([
            (3, format!("      {} Zig64", checked(1)), Color::LightGreen),
            (
                4,
                format!("      {} MinGW64", checked(2)),
                Color::LightGreen,
            ),
            (
                5,
                format!("      {} MinGW32", checked(3)),
                Color::LightMagenta,
            ),
        ]);
    }
    tree_entries.push((
        6,
        format!("  [{}] Widgets", if widgets_expanded { "-" } else { "+" }),
        Color::LightCyan,
    ));
    if widgets_expanded {
        tree_entries.extend([
            (
                7,
                format!("      {} Charts", checked(4)),
                Color::LightYellow,
            ),
            (
                8,
                format!("      {} Tables", checked(5)),
                Color::LightYellow,
            ),
        ]);
    }
    let selected_row = tree_entries
        .iter()
        .position(|(id, _, _)| *id == selected)
        .unwrap_or(0);
    let tree_items = tree_entries.iter().map(|(_, label, color)| {
        ListItem::new(Span::styled(label.as_str(), Style::default().fg(*color)))
    });
    let tree = List::new(tree_items)
        .highlight_symbol(if tree_active { "▶ " } else { "  " })
        .highlight_style(if tree_active {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(42, 63, 92))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .block(
            Block::default()
                .title(if tree_active {
                    " ▶ Tree: hierarchy + ticks "
                } else {
                    " Tree: hierarchy + ticks "
                })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if tree_active {
                    Color::LightCyan
                } else {
                    Color::Blue
                })),
        );
    let mut list_state = ListState::default().with_selected(Some(selected_row));
    StatefulWidget::render(tree, left[0], &mut buffer, &mut list_state);

    let meter_rows =
        Layout::vertical([Constraint::Length(4), Constraint::Length(4)]).split(left[1]);
    Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(255, 105, 180))
                .bg(Color::Rgb(38, 32, 55))
                .add_modifier(Modifier::BOLD),
        )
        .ratio(cpu)
        .label(format!("{:>3}%", (cpu * 100.0).round() as u8))
        .render(meter_rows[0], &mut buffer);
    Gauge::default()
        .block(Block::default().title(" Memory ").borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(80, 210, 170))
                .bg(Color::Rgb(25, 48, 50)),
        )
        .ratio(memory)
        .label(format!("{:.1} / 16 GiB", memory * 16.0))
        .render(meter_rows[1], &mut buffer);

    Paragraph::new(vec![
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::LightGreen)),
            Span::raw("UTF-8: Здравей • こんにちは"),
        ]),
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::LightGreen)),
            Span::raw("TrueColor RGB via Win32 VT"),
        ]),
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::LightGreen)),
            Span::raw("Zig64 / MinGW64 / MinGW32"),
        ]),
    ])
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .title(" Capabilities ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta)),
    )
    .render(left[2], &mut buffer);

    let right = Layout::vertical([Constraint::Length(10), Constraint::Min(13)])
        .spacing(1)
        .split(columns[1]);
    let request_count = 9_420u64 + u64::from(tick) * 17;
    let table_rows = [
        Row::new(["Harbour VM", "RUNNING", "1.8 ms", "99.99%"]),
        Row::new(["Ratatui FFI", "RUNNING", "0.3 ms", "100.0%"]),
        Row::new(["DBF index", "SYNCING", "4.7 ms", "99.95%"]),
        Row::new([
            String::from("Events"),
            format!("{request_count}"),
            String::from("0.6 ms"),
            String::from("live"),
        ]),
    ];
    let table_active = focus % 2 == 1;
    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(34),
            Constraint::Percentage(23),
            Constraint::Percentage(20),
            Constraint::Percentage(23),
        ],
    )
    .header(
        Row::new(["COMPONENT", "STATE", "LATENCY", "HEALTH"])
            .style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(114, 239, 221))
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
    )
    .row_highlight_style(if table_active {
        Style::default()
            .fg(Color::LightYellow)
            .bg(Color::Rgb(45, 45, 65))
    } else {
        Style::default()
    })
    .highlight_symbol(if table_active { "▸ " } else { "  " })
    .column_spacing(1)
    .block(
        Block::default()
            .title(if table_active {
                " ▶ Table + live data "
            } else {
                " Table + live data "
            })
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if table_active {
                Color::LightYellow
            } else {
                Color::Cyan
            })),
    );
    let mut table_state = TableState::default().with_selected(Some(table_selected % 4));
    StatefulWidget::render(table, right[0], &mut buffer, &mut table_state);

    let charts = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
        .spacing(1)
        .split(right[1]);
    let throughput: Vec<(f64, f64)> = (0..50)
        .map(|x| {
            let x = f64::from(x);
            let y = 58.0 + (x / 4.2 + phase).sin() * 22.0 + (x / 2.7).cos() * 7.0;
            (x, y)
        })
        .collect();
    let latency: Vec<(f64, f64)> = (0..50)
        .map(|x| {
            let x = f64::from(x);
            let y = 24.0 + (x / 5.5 + phase * 0.7).cos() * 11.0;
            (x, y)
        })
        .collect();
    let datasets = vec![
        Dataset::default()
            .name("throughput")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::LightCyan))
            .data(&throughput),
        Dataset::default()
            .name("latency")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::LightMagenta))
            .data(&latency),
    ];
    Chart::new(datasets)
        .block(
            Block::default()
                .title(" Braille Chart ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightBlue)),
        )
        .x_axis(
            Axis::default()
                .bounds([0.0, 49.0])
                .labels(["-50s", "now"])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(["0", "50", "100"])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .render(charts[0], &mut buffer);

    let small_charts =
        Layout::vertical([Constraint::Length(6), Constraint::Min(7)]).split(charts[1]);
    let spark_data: Vec<u64> = (0..42)
        .map(|x| {
            let wave = (f64::from(x) / 3.2 + phase).sin();
            (50.0 + wave * 38.0).round().clamp(0.0, 100.0) as u64
        })
        .collect();
    Sparkline::default()
        .data(spark_data)
        .max(100)
        .style(Style::default().fg(Color::Rgb(255, 210, 90)))
        .block(
            Block::default()
                .title(" Sparkline ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .render(small_charts[0], &mut buffer);

    let compiler_bars = vec![
        Bar::with_label("Zig64", 94).style(Style::default().fg(Color::LightCyan)),
        Bar::with_label("M64", 87).style(Style::default().fg(Color::LightGreen)),
        Bar::with_label("M32", 72).style(Style::default().fg(Color::LightMagenta)),
        Bar::with_label("ABI", 100).style(Style::default().fg(Color::LightYellow)),
    ];
    BarChart::new(compiler_bars)
        .bar_width(6)
        .bar_gap(2)
        .max(100)
        .value_style(
            Style::default()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .title(" BarChart / toolchains ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .render(small_charts[1], &mut buffer);

    if menu_open {
        let menu_items: &[&str] = match menu % 4 {
            0 => &["New dashboard", "Open project…", "Save snapshot", "Exit"],
            1 => &["Overview", "Services", "Performance", "Unicode"],
            2 => &[
                "ABI inspector",
                "DBF monitor",
                "Theme editor",
                "Diagnostics",
            ],
            _ => &[
                "Keyboard help",
                "FFI reference",
                "About Ratatui",
                "About Harbour",
            ],
        };
        let menu_x = [2, 15, 29, 44][menu % 4];
        let dropdown = Rect::new(menu_x, rows[2].y, 32, 7.min(rows[2].height));
        Clear.render(dropdown, &mut buffer);
        let dropdown_list = List::new(
            menu_items
                .iter()
                .map(|item| ListItem::new(format!("  {item}"))),
        )
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(114, 239, 221))
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(" Menu ")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(18, 24, 38)))
                .border_style(Style::default().fg(Color::LightCyan)),
        );
        let mut dropdown_state =
            ListState::default().with_selected(Some(menu_item % menu_items.len()));
        StatefulWidget::render(dropdown_list, dropdown, &mut buffer, &mut dropdown_state);
    }

    Paragraph::new(Line::from(vec![
        Span::styled(
            " M/TAB ",
            Style::default().fg(Color::Black).bg(Color::LightCyan),
        ),
        Span::raw(" menu  "),
        Span::styled(
            " F6 ",
            Style::default().fg(Color::Black).bg(Color::LightMagenta),
        ),
        Span::raw(" focus  "),
        Span::styled(
            " ARROWS ",
            Style::default().fg(Color::Black).bg(Color::LightGreen),
        ),
        Span::raw(" move  "),
        Span::styled(
            " +/- ",
            Style::default().fg(Color::Black).bg(Color::LightCyan),
        ),
        Span::raw(" fold  "),
        Span::styled(
            " SPACE ",
            Style::default().fg(Color::Black).bg(Color::LightYellow),
        ),
        Span::raw(" tick  "),
        Span::styled(" P ", Style::default().fg(Color::White).bg(Color::Blue)),
        Span::raw(" pause  "),
        Span::styled(
            " Q / ESC ",
            Style::default().fg(Color::White).bg(Color::Red),
        ),
        Span::raw(" close "),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .render(rows[3], &mut buffer);

    Ok(buffer_to_text(&buffer, area, ansi))
}

pub(crate) fn buffer_to_text(buffer: &Buffer, area: Rect, ansi: bool) -> Vec<u8> {
    let mut output = String::with_capacity(area.width as usize * area.height as usize * 2);
    let mut active_style: Option<(Color, Color, Modifier)> = None;

    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            let cell = &buffer[(x, y)];
            if ansi {
                let style = (cell.fg, cell.bg, cell.modifier);
                if active_style != Some(style) {
                    output.push_str("\x1b[0m");
                    push_ansi_color(&mut output, cell.fg, true);
                    push_ansi_color(&mut output, cell.bg, false);
                    if cell.modifier.contains(Modifier::BOLD) {
                        output.push_str("\x1b[1m");
                    }
                    if cell.modifier.contains(Modifier::DIM) {
                        output.push_str("\x1b[2m");
                    }
                    if cell.modifier.contains(Modifier::ITALIC) {
                        output.push_str("\x1b[3m");
                    }
                    if cell.modifier.contains(Modifier::UNDERLINED) {
                        output.push_str("\x1b[4m");
                    }
                    if cell.modifier.contains(Modifier::SLOW_BLINK) {
                        output.push_str("\x1b[5m");
                    }
                    if cell.modifier.contains(Modifier::RAPID_BLINK) {
                        output.push_str("\x1b[6m");
                    }
                    if cell.modifier.contains(Modifier::REVERSED) {
                        output.push_str("\x1b[7m");
                    }
                    if cell.modifier.contains(Modifier::HIDDEN) {
                        output.push_str("\x1b[8m");
                    }
                    if cell.modifier.contains(Modifier::CROSSED_OUT) {
                        output.push_str("\x1b[9m");
                    }
                    active_style = Some(style);
                }
            }
            output.push_str(cell.symbol());
            let display_width = UnicodeWidthStr::width(cell.symbol()).max(1);
            x = x.saturating_add(u16::try_from(display_width).unwrap_or(u16::MAX));
        }
        if ansi {
            output.push_str("\x1b[0m");
            active_style = None;
        }
        if y + 1 < area.bottom() {
            output.push_str("\r\n");
        }
    }
    if ansi {
        output.push_str("\x1b[0m");
    }
    output.into_bytes()
}

fn push_ansi_color(output: &mut String, color: Color, foreground: bool) {
    let base = if foreground { 30 } else { 40 };
    match color {
        Color::Reset => {}
        Color::Black => output.push_str(&format!("\x1b[{}m", base)),
        Color::Red => output.push_str(&format!("\x1b[{}m", base + 1)),
        Color::Green => output.push_str(&format!("\x1b[{}m", base + 2)),
        Color::Yellow => output.push_str(&format!("\x1b[{}m", base + 3)),
        Color::Blue => output.push_str(&format!("\x1b[{}m", base + 4)),
        Color::Magenta => output.push_str(&format!("\x1b[{}m", base + 5)),
        Color::Cyan => output.push_str(&format!("\x1b[{}m", base + 6)),
        Color::Gray => output.push_str(&format!("\x1b[{}m", base + 7)),
        Color::DarkGray => output.push_str(&format!("\x1b[{}m", base + 60)),
        Color::LightRed => output.push_str(&format!("\x1b[{}m", base + 61)),
        Color::LightGreen => output.push_str(&format!("\x1b[{}m", base + 62)),
        Color::LightYellow => output.push_str(&format!("\x1b[{}m", base + 63)),
        Color::LightBlue => output.push_str(&format!("\x1b[{}m", base + 64)),
        Color::LightMagenta => output.push_str(&format!("\x1b[{}m", base + 65)),
        Color::LightCyan => output.push_str(&format!("\x1b[{}m", base + 66)),
        Color::White => output.push_str(&format!("\x1b[{}m", base + 67)),
        Color::Indexed(index) => output.push_str(&format!(
            "\x1b[{};5;{}m",
            if foreground { 38 } else { 48 },
            index
        )),
        Color::Rgb(red, green, blue) => output.push_str(&format!(
            "\x1b[{};2;{};{};{}m",
            if foreground { 38 } else { 48 },
            red,
            green,
            blue
        )),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hrui_abi_version() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
/// Renders a dashboard into a caller-owned byte buffer.
///
/// # Safety
///
/// Non-null input pointers must address their declared number of readable
/// bytes. `output_length` must be writable. When non-null, `output` must
/// address at least `output_capacity` writable bytes.
pub unsafe extern "C" fn hrui_render_dashboard(
    title: *const u8,
    title_length: usize,
    body: *const u8,
    body_length: usize,
    width: u16,
    height: u16,
    ansi: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, String> {
        if output_length.is_null() {
            return Err("output_length must not be NULL".into());
        }
        // SAFETY: Pointers and byte counts are validated by `utf8_arg`.
        let title = unsafe { utf8_arg(title, title_length, "title")? };
        let body = unsafe { utf8_arg(body, body_length, "body")? };
        let rendered = render_dashboard(title, body, width, height, ansi != 0)?;
        // SAFETY: `output_length` was checked above and is owned by the caller.
        unsafe { ptr::write(output_length, rendered.len()) };
        if output.is_null() || output_capacity < rendered.len() {
            return Ok(BUFFER_TOO_SMALL);
        }
        // SAFETY: The caller provides at least `output_capacity` writable bytes.
        unsafe { ptr::copy_nonoverlapping(rendered.as_ptr(), output, rendered.len()) };
        Ok(OK)
    }));

    match result {
        Ok(Ok(OK)) => {
            set_last_error("");
            OK
        }
        Ok(Ok(BUFFER_TOO_SMALL)) => {
            set_last_error("output buffer is too small");
            BUFFER_TOO_SMALL
        }
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            set_last_error(message);
            INVALID_ARGUMENT
        }
        Err(_) => {
            set_last_error("Rust panic was caught at the Harbour/Ratatui ABI boundary");
            PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Renders the animated multi-widget showcase into a caller-owned buffer.
///
/// # Safety
///
/// `output_length` must be writable. When non-null, `output` must address at
/// least `output_capacity` writable bytes.
pub unsafe extern "C" fn hrui_render_showcase(
    tick: u32,
    selected: usize,
    width: u16,
    height: u16,
    ansi: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, String> {
        if output_length.is_null() {
            return Err("output_length must not be NULL".into());
        }
        let rendered = render_showcase(ShowcaseOptions {
            tick,
            selected,
            table_selected: 0,
            focus: 0,
            menu: 0,
            menu_item: 0,
            checked_mask: 0b11_1111,
            expanded_mask: 0b11,
            menu_open: false,
            width,
            height,
            ansi: ansi != 0,
        })?;
        // SAFETY: `output_length` was checked above and is owned by the caller.
        unsafe { ptr::write(output_length, rendered.len()) };
        if output.is_null() || output_capacity < rendered.len() {
            return Ok(BUFFER_TOO_SMALL);
        }
        // SAFETY: The caller provides at least `output_capacity` writable bytes.
        unsafe { ptr::copy_nonoverlapping(rendered.as_ptr(), output, rendered.len()) };
        Ok(OK)
    }));

    match result {
        Ok(Ok(OK)) => {
            set_last_error("");
            OK
        }
        Ok(Ok(BUFFER_TOO_SMALL)) => {
            set_last_error("output buffer is too small");
            BUFFER_TOO_SMALL
        }
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            set_last_error(message);
            INVALID_ARGUMENT
        }
        Err(_) => {
            set_last_error("Rust panic was caught at the Harbour/Ratatui ABI boundary");
            PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Renders the interactive RGB showcase with menu and tree state.
///
/// # Safety
///
/// `output_length` must be writable. When non-null, `output` must address at
/// least `output_capacity` writable bytes.
pub unsafe extern "C" fn hrui_render_showcase_v2(
    tick: u32,
    selected: usize,
    menu: usize,
    menu_item: usize,
    checked_mask: u32,
    menu_open: u8,
    width: u16,
    height: u16,
    ansi: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, String> {
        if output_length.is_null() {
            return Err("output_length must not be NULL".into());
        }
        let rendered = render_showcase(ShowcaseOptions {
            tick,
            selected,
            table_selected: 0,
            focus: 0,
            menu,
            menu_item,
            checked_mask,
            expanded_mask: 0b11,
            menu_open: menu_open != 0,
            width,
            height,
            ansi: ansi != 0,
        })?;
        // SAFETY: `output_length` was checked above and is owned by the caller.
        unsafe { ptr::write(output_length, rendered.len()) };
        if output.is_null() || output_capacity < rendered.len() {
            return Ok(BUFFER_TOO_SMALL);
        }
        // SAFETY: The caller provides at least `output_capacity` writable bytes.
        unsafe { ptr::copy_nonoverlapping(rendered.as_ptr(), output, rendered.len()) };
        Ok(OK)
    }));

    match result {
        Ok(Ok(OK)) => {
            set_last_error("");
            OK
        }
        Ok(Ok(BUFFER_TOO_SMALL)) => {
            set_last_error("output buffer is too small");
            BUFFER_TOO_SMALL
        }
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            set_last_error(message);
            INVALID_ARGUMENT
        }
        Err(_) => {
            set_last_error("Rust panic was caught at the Harbour/Ratatui ABI boundary");
            PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Renders the interactive showcase with independent tree/table focus state.
///
/// # Safety
///
/// `output_length` must be writable. When non-null, `output` must address at
/// least `output_capacity` writable bytes.
pub unsafe extern "C" fn hrui_render_showcase_v3(
    tick: u32,
    tree_selected: usize,
    table_selected: usize,
    focus: usize,
    menu: usize,
    menu_item: usize,
    checked_mask: u32,
    menu_open: u8,
    width: u16,
    height: u16,
    ansi: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, String> {
        if output_length.is_null() {
            return Err("output_length must not be NULL".into());
        }
        let rendered = render_showcase(ShowcaseOptions {
            tick,
            selected: tree_selected,
            table_selected,
            focus,
            menu,
            menu_item,
            checked_mask,
            expanded_mask: 0b11,
            menu_open: menu_open != 0,
            width,
            height,
            ansi: ansi != 0,
        })?;
        // SAFETY: `output_length` was checked above and is owned by the caller.
        unsafe { ptr::write(output_length, rendered.len()) };
        if output.is_null() || output_capacity < rendered.len() {
            return Ok(BUFFER_TOO_SMALL);
        }
        // SAFETY: The caller provides at least `output_capacity` writable bytes.
        unsafe { ptr::copy_nonoverlapping(rendered.as_ptr(), output, rendered.len()) };
        Ok(OK)
    }));

    match result {
        Ok(Ok(OK)) => {
            set_last_error("");
            OK
        }
        Ok(Ok(BUFFER_TOO_SMALL)) => {
            set_last_error("output buffer is too small");
            BUFFER_TOO_SMALL
        }
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            set_last_error(message);
            INVALID_ARGUMENT
        }
        Err(_) => {
            set_last_error("Rust panic was caught at the Harbour/Ratatui ABI boundary");
            PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Renders the showcase with independent focus and collapsible tree groups.
///
/// # Safety
///
/// `output_length` must be writable. When non-null, `output` must address at
/// least `output_capacity` writable bytes.
pub unsafe extern "C" fn hrui_render_showcase_v4(
    tick: u32,
    tree_selected: usize,
    table_selected: usize,
    focus: usize,
    menu: usize,
    menu_item: usize,
    checked_mask: u32,
    expanded_mask: u32,
    menu_open: u8,
    width: u16,
    height: u16,
    ansi: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, String> {
        if output_length.is_null() {
            return Err("output_length must not be NULL".into());
        }
        let rendered = render_showcase(ShowcaseOptions {
            tick,
            selected: tree_selected,
            table_selected,
            focus,
            menu,
            menu_item,
            checked_mask,
            expanded_mask,
            menu_open: menu_open != 0,
            width,
            height,
            ansi: ansi != 0,
        })?;
        // SAFETY: `output_length` was checked above and is owned by the caller.
        unsafe { ptr::write(output_length, rendered.len()) };
        if output.is_null() || output_capacity < rendered.len() {
            return Ok(BUFFER_TOO_SMALL);
        }
        // SAFETY: The caller provides at least `output_capacity` writable bytes.
        unsafe { ptr::copy_nonoverlapping(rendered.as_ptr(), output, rendered.len()) };
        Ok(OK)
    }));

    match result {
        Ok(Ok(OK)) => {
            set_last_error("");
            OK
        }
        Ok(Ok(BUFFER_TOO_SMALL)) => {
            set_last_error("output buffer is too small");
            BUFFER_TOO_SMALL
        }
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            set_last_error(message);
            INVALID_ARGUMENT
        }
        Err(_) => {
            set_last_error("Rust panic was caught at the Harbour/Ratatui ABI boundary");
            PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Renders a versioned Harbour command buffer into caller-owned output.
///
/// # Safety
///
/// `commands` must address `commands_length` readable bytes when non-null.
/// `output_length` must be writable. When non-null, `output` must address at
/// least `output_capacity` writable bytes.
pub unsafe extern "C" fn hrui_render_commands(
    commands: *const u8,
    commands_length: usize,
    ansi: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, String> {
        if output_length.is_null() {
            return Err("output_length must not be NULL".into());
        }
        if commands.is_null() && commands_length != 0 {
            return Err("commands is NULL but commands_length is not zero".into());
        }
        let input = if commands_length == 0 {
            &[]
        } else {
            // SAFETY: The caller promises a readable command buffer.
            unsafe { slice::from_raw_parts(commands, commands_length) }
        };
        let rendered = commands::render_commands(input, ansi != 0)?;
        // SAFETY: `output_length` was checked above and is owned by the caller.
        unsafe { ptr::write(output_length, rendered.len()) };
        if output.is_null() || output_capacity < rendered.len() {
            return Ok(BUFFER_TOO_SMALL);
        }
        // SAFETY: The caller provides at least `output_capacity` writable bytes.
        unsafe { ptr::copy_nonoverlapping(rendered.as_ptr(), output, rendered.len()) };
        Ok(OK)
    }));

    match result {
        Ok(Ok(OK)) => {
            set_last_error("");
            OK
        }
        Ok(Ok(BUFFER_TOO_SMALL)) => {
            set_last_error("output buffer is too small");
            BUFFER_TOO_SMALL
        }
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            set_last_error(message);
            INVALID_ARGUMENT
        }
        Err(_) => {
            set_last_error("Rust panic was caught at the Harbour/Ratatui ABI boundary");
            PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Copies the most recent error as a NUL-terminated UTF-8 string.
///
/// # Safety
///
/// When `output` is non-null, it must address at least `output_capacity`
/// writable bytes.
pub unsafe extern "C" fn hrui_last_error(output: *mut c_char, output_capacity: usize) -> usize {
    let message = last_error_slot()
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "last-error lock is poisoned".into());
    let required = message.len();
    if !output.is_null() && output_capacity > 0 {
        let copied = required.min(output_capacity - 1);
        // SAFETY: The caller provides `output_capacity` writable bytes.
        unsafe {
            ptr::copy_nonoverlapping(message.as_ptr(), output.cast::<u8>(), copied);
            ptr::write(output.add(copied), 0);
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_real_ratatui_buffer() {
        let rendered = render_dashboard("Zig64", "Harbour calls Ratatui", 50, 11, false).unwrap();
        let text = String::from_utf8(rendered).unwrap();
        assert!(text.contains("Harbour + Rust C ABI + Ratatui"));
        assert!(text.contains("Harbour calls Ratatui"));
        assert!(text.contains("Ratatui rendering: OK"));
        assert_eq!(text.lines().count(), 11);
    }

    #[test]
    fn keeps_utf8_text() {
        let rendered = render_dashboard("Unicode", "Здравей, Harbour!", 40, 10, false).unwrap();
        assert!(
            String::from_utf8(rendered)
                .unwrap()
                .contains("Здравей, Harbour!")
        );
    }

    #[test]
    fn rejects_impossible_geometry() {
        assert!(render_dashboard("test", "test", 3, 3, false).is_err());
    }

    #[test]
    fn ffi_supports_the_two_pass_buffer_contract() {
        let title = b"ABI test";
        let body = b"Rendered by Ratatui";
        let mut required = 0usize;

        // SAFETY: All input pointers are valid for their stated lengths and
        // `required` is a valid writable pointer.
        let probe = unsafe {
            hrui_render_dashboard(
                title.as_ptr(),
                title.len(),
                body.as_ptr(),
                body.len(),
                40,
                10,
                0,
                ptr::null_mut(),
                0,
                &mut required,
            )
        };
        assert_eq!(probe, BUFFER_TOO_SMALL);
        assert!(required > 0);

        let mut output = vec![0u8; required];
        let mut written = 0usize;
        // SAFETY: `output` and `written` satisfy the exported ABI contract.
        let status = unsafe {
            hrui_render_dashboard(
                title.as_ptr(),
                title.len(),
                body.as_ptr(),
                body.len(),
                40,
                10,
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        };
        assert_eq!(status, OK);
        assert_eq!(written, required);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Rendered by Ratatui")
        );
    }

    #[test]
    fn command_ffi_supports_the_two_pass_buffer_contract() {
        let mut commands = b"HRC1".to_vec();
        commands.extend(1u16.to_le_bytes());
        commands.extend(24u16.to_le_bytes());
        commands.extend(9u16.to_le_bytes());
        commands.extend(0u16.to_le_bytes());
        let mut required = 0usize;

        // SAFETY: `commands` and `required` satisfy the exported ABI contract.
        let probe = unsafe {
            hrui_render_commands(
                commands.as_ptr(),
                commands.len(),
                0,
                ptr::null_mut(),
                0,
                &mut required,
            )
        };
        assert_eq!(probe, BUFFER_TOO_SMALL);
        assert!(required > 0);

        let mut output = vec![0u8; required];
        let mut written = 0usize;
        // SAFETY: `output` and `written` satisfy the exported ABI contract.
        let status = unsafe {
            hrui_render_commands(
                commands.as_ptr(),
                commands.len(),
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        };
        assert_eq!(status, OK);
        assert_eq!(written, required);
        assert_eq!(String::from_utf8(output).unwrap().lines().count(), 9);
    }

    #[test]
    fn showcase_contains_the_feature_widgets() {
        let rendered = render_showcase(ShowcaseOptions {
            tick: 17,
            selected: 2,
            table_selected: 0,
            focus: 0,
            menu: 1,
            menu_item: 2,
            checked_mask: 0b10_1011,
            expanded_mask: 0b11,
            menu_open: false,
            width: 120,
            height: 38,
            ansi: false,
        })
        .unwrap();
        let text = String::from_utf8(rendered).unwrap();
        assert!(text.contains("Tree: hierarchy + ticks"));
        assert!(text.contains("☑"));
        assert!(text.contains("☐"));
        assert!(text.contains("Table + live data"));
        assert!(text.contains("Braille Chart"));
        assert!(text.contains("Sparkline"));
        assert!(text.contains("BarChart / toolchains"));
        assert!(text.contains("Здравей"));
        assert_eq!(text.lines().count(), 38);

        let menu = render_showcase(ShowcaseOptions {
            tick: 17,
            selected: 2,
            table_selected: 0,
            focus: 0,
            menu: 1,
            menu_item: 2,
            checked_mask: 0b10_1011,
            expanded_mask: 0b11,
            menu_open: true,
            width: 120,
            height: 38,
            ansi: false,
        })
        .unwrap();
        let menu_text = String::from_utf8(menu).unwrap();
        assert!(menu_text.contains("Menu"));
        assert!(menu_text.contains("Performance"));
    }

    #[test]
    fn showcase_emits_truecolor_sgr_when_requested() {
        let rendered = render_showcase(ShowcaseOptions {
            tick: 3,
            selected: 0,
            table_selected: 0,
            focus: 0,
            menu: 0,
            menu_item: 0,
            checked_mask: 0b11_1111,
            expanded_mask: 0b11,
            menu_open: false,
            width: 120,
            height: 38,
            ansi: true,
        })
        .unwrap();
        let text = String::from_utf8(rendered).unwrap();
        assert!(text.contains("\x1b[38;2;255;104;180m"));
        assert!(text.contains("\x1b[48;2;18;24;38m"));
    }

    #[test]
    fn showcase_plain_text_rows_keep_the_requested_display_width() {
        let rendered = render_showcase(ShowcaseOptions {
            tick: 3,
            selected: 0,
            table_selected: 0,
            focus: 0,
            menu: 0,
            menu_item: 0,
            checked_mask: 0b11_1111,
            expanded_mask: 0b11,
            menu_open: false,
            width: 120,
            height: 38,
            ansi: false,
        })
        .unwrap();
        let text = String::from_utf8(rendered).unwrap();

        for (line_number, line) in text.lines().enumerate() {
            assert_eq!(
                UnicodeWidthStr::width(line),
                120,
                "row {} has the wrong terminal display width: {line:?}",
                line_number + 1
            );
        }
    }

    #[test]
    fn showcase_marks_only_the_focused_data_panel_active() {
        let rendered = render_showcase(ShowcaseOptions {
            tick: 3,
            selected: 2,
            table_selected: 1,
            focus: 1,
            menu: 0,
            menu_item: 0,
            checked_mask: 0b11_1111,
            expanded_mask: 0b11,
            menu_open: false,
            width: 120,
            height: 38,
            ansi: false,
        })
        .unwrap();
        let text = String::from_utf8(rendered).unwrap();

        assert!(text.contains("▶ Table + live data"));
        assert!(!text.contains("▶ Tree: hierarchy + ticks"));
    }

    #[test]
    fn showcase_collapses_tree_groups_without_losing_group_rows() {
        let rendered = render_showcase(ShowcaseOptions {
            tick: 3,
            selected: 2,
            table_selected: 0,
            focus: 0,
            menu: 0,
            menu_item: 0,
            checked_mask: 0b11_1111,
            expanded_mask: 0,
            menu_open: false,
            width: 120,
            height: 38,
            ansi: false,
        })
        .unwrap();
        let text = String::from_utf8(rendered).unwrap();

        assert!(text.contains("[+] Toolchains"));
        assert!(text.contains("[+] Widgets"));
        assert!(!text.contains("      ☑ Zig64"));
        assert!(!text.contains("      ☑ Charts"));
    }
}
