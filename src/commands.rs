use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Bar, BarChart, BarGroup, Block, Borders, Chart, Clear, Dataset, Gauge, GraphType, List,
    ListItem, ListState, Paragraph, Row, Sparkline, StatefulWidget, Table, TableState, Tabs,
    Widget, Wrap,
};

use crate::buffer_to_text;

const MAGIC: &[u8; 4] = b"HRC1";
const VERSION: u16 = 1;
const OPTIONAL: u8 = 1;
const MAX_COMMANDS: usize = 4096;
const MAX_ITEMS: usize = 4096;
const MAX_INPUT: usize = 8 * 1024 * 1024;

const CLEAR: u8 = 1;
const BLOCK: u8 = 2;
const PARAGRAPH: u8 = 3;
const TABS: u8 = 4;
const LIST: u8 = 5;
const GAUGE: u8 = 6;
const TABLE: u8 = 7;
const SPARKLINE: u8 = 8;
const BAR_CHART: u8 = 9;
const CHART: u8 = 10;

// Compact style mask used by the HRC1 Paragraph command.  The values are
// deliberately independent from Ratatui's internal bit positions so the
// Harbour-facing protocol remains a small, stable byte.
const MOD_BOLD: u8 = 1;
const MOD_DIM: u8 = 2;
const MOD_ITALIC: u8 = 4;
const MOD_UNDERLINE: u8 = 8;
const MOD_BLINK: u8 = 16;
const MOD_REVERSE: u8 = 32;
const MOD_CROSSED: u8 = 64;
const MOD_RAPID_BLINK: u8 = 128;

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| "command length overflow".to_owned())?;
        if end > self.bytes.len() {
            return Err(format!(
                "truncated command buffer: need {length} bytes, have {}",
                self.remaining()
            ));
        }
        let value = &self.bytes[self.position..end];
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        let bytes: [u8; 2] = self.take(2)?.try_into().expect("length checked");
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let bytes: [u8; 4] = self.take(4)?.try_into().expect("length checked");
        Ok(u32::from_le_bytes(bytes))
    }

    fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }

    fn string(&mut self) -> Result<String, String> {
        let length = usize::try_from(self.u32()?).map_err(|_| "string length overflow")?;
        let bytes = self.take(length)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| format!("command string is not UTF-8: {error}"))
    }

    fn rgb(&mut self) -> Result<Color, String> {
        Ok(Color::Rgb(self.u8()?, self.u8()?, self.u8()?))
    }

    fn rect(&mut self, frame: Rect) -> Result<Rect, String> {
        let rect = Rect::new(self.u16()?, self.u16()?, self.u16()?, self.u16()?);
        let right = rect.x.checked_add(rect.width);
        let bottom = rect.y.checked_add(rect.height);
        if rect.width == 0
            || rect.height == 0
            || right.is_none_or(|value| value > frame.width)
            || bottom.is_none_or(|value| value > frame.height)
        {
            return Err(format!(
                "widget rectangle ({}, {}, {}, {}) is outside {}x{} frame",
                rect.x, rect.y, rect.width, rect.height, frame.width, frame.height
            ));
        }
        Ok(rect)
    }

    fn finish(self) -> Result<(), String> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(format!(
                "command payload has {} trailing bytes",
                self.remaining()
            ))
        }
    }
}

fn block(title: String, border: Color, background: Color) -> Block<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(background));
    if title.is_empty() {
        block
    } else {
        block.title(format!(" {title} "))
    }
}

fn modifiers_from_mask(mask: u8) -> Modifier {
    let mut modifiers = Modifier::empty();
    if mask & MOD_BOLD != 0 {
        modifiers |= Modifier::BOLD;
    }
    if mask & MOD_DIM != 0 {
        modifiers |= Modifier::DIM;
    }
    if mask & MOD_ITALIC != 0 {
        modifiers |= Modifier::ITALIC;
    }
    if mask & MOD_UNDERLINE != 0 {
        modifiers |= Modifier::UNDERLINED;
    }
    if mask & MOD_BLINK != 0 {
        modifiers |= Modifier::SLOW_BLINK;
    }
    if mask & MOD_RAPID_BLINK != 0 {
        modifiers |= Modifier::RAPID_BLINK;
    }
    if mask & MOD_REVERSE != 0 {
        modifiers |= Modifier::REVERSED;
    }
    if mask & MOD_CROSSED != 0 {
        modifiers |= Modifier::CROSSED_OUT;
    }
    modifiers
}

pub(crate) fn render_commands(input: &[u8], ansi: bool) -> Result<Vec<u8>, String> {
    if input.len() > MAX_INPUT {
        return Err(format!("command buffer exceeds {MAX_INPUT} bytes"));
    }
    let mut reader = Reader::new(input);
    if reader.take(4)? != MAGIC {
        return Err("invalid command-buffer magic; expected HRC1".into());
    }
    let version = reader.u16()?;
    if version != VERSION {
        return Err(format!(
            "unsupported command-buffer version {version}; expected {VERSION}"
        ));
    }
    let width = reader.u16()?;
    let height = reader.u16()?;
    if !(24..=500).contains(&width) || !(9..=200).contains(&height) {
        return Err("command frame must be 24..500 by 9..200 cells".into());
    }
    let command_count = usize::from(reader.u16()?);
    if command_count > MAX_COMMANDS {
        return Err(format!("too many commands: {command_count}"));
    }

    let frame = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(frame);
    for command_index in 0..command_count {
        let opcode = reader.u8()?;
        let flags = reader.u8()?;
        let payload_length = usize::try_from(reader.u32()?).map_err(|_| "payload overflow")?;
        let mut payload = Reader::new(reader.take(payload_length)?);
        let result = match opcode {
            CLEAR => render_clear(&mut payload, frame, &mut buffer),
            BLOCK => render_block(&mut payload, frame, &mut buffer),
            PARAGRAPH => render_paragraph(&mut payload, frame, &mut buffer),
            TABS => render_tabs(&mut payload, frame, &mut buffer),
            LIST => render_list(&mut payload, frame, &mut buffer),
            GAUGE => render_gauge(&mut payload, frame, &mut buffer),
            TABLE => render_table(&mut payload, frame, &mut buffer),
            SPARKLINE => render_sparkline(&mut payload, frame, &mut buffer),
            BAR_CHART => render_bar_chart(&mut payload, frame, &mut buffer),
            CHART => render_chart(&mut payload, frame, &mut buffer),
            _ if flags & OPTIONAL != 0 => Ok(()),
            _ => Err(format!("unknown required command opcode {opcode}")),
        };
        result.map_err(|error| format!("command {}: {error}", command_index + 1))?;
        if opcode <= CHART {
            payload
                .finish()
                .map_err(|error| format!("command {}: {error}", command_index + 1))?;
        }
    }
    reader.finish()?;
    Ok(buffer_to_text(&buffer, frame, ansi))
}

fn render_clear(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let background = reader.rgb()?;
    Clear.render(area, buffer);
    Block::default()
        .style(Style::default().bg(background))
        .render(area, buffer);
    Ok(())
}

fn render_block(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let border = reader.rgb()?;
    let background = reader.rgb()?;
    block(title, border, background).render(area, buffer);
    Ok(())
}

fn render_paragraph(
    reader: &mut Reader<'_>,
    frame: Rect,
    buffer: &mut Buffer,
) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let bordered = reader.u8()? != 0;
    let title = reader.string()?;
    let text = reader.string()?;
    let foreground = reader.rgb()?;
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    let alignment = match reader.u8()? {
        0 => Alignment::Left,
        1 => Alignment::Center,
        2 => Alignment::Right,
        value => return Err(format!("invalid paragraph alignment {value}")),
    };
    let wrap = reader.u8()? != 0;
    let modifiers = modifiers_from_mask(reader.u8()?);
    let style = Style::default()
        .fg(foreground)
        .bg(background)
        .add_modifier(modifiers);
    let mut paragraph = Paragraph::new(text).style(style).alignment(alignment);
    if wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    if bordered {
        paragraph = paragraph.block(block(title, border, background));
    }
    paragraph.render(area, buffer);
    Ok(())
}

fn render_tabs(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let selected = usize::from(reader.u16()?);
    let count = usize::from(reader.u16()?);
    if count == 0 || count > MAX_ITEMS {
        return Err(format!("invalid tab count {count}"));
    }
    let mut titles = Vec::with_capacity(count);
    for _ in 0..count {
        titles.push(Line::from(reader.string()?));
    }
    let foreground = reader.rgb()?;
    let background = reader.rgb()?;
    let selected_foreground = reader.rgb()?;
    let selected_background = reader.rgb()?;
    let border = reader.rgb()?;
    Tabs::new(titles)
        .select(selected % count)
        .style(Style::default().fg(foreground).bg(background))
        .highlight_style(
            Style::default()
                .fg(selected_foreground)
                .bg(selected_background)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" • ")
        .block(block(String::new(), border, background))
        .render(area, buffer);
    Ok(())
}

fn render_list(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let selected = usize::from(reader.u16()?);
    let active = reader.u8()? != 0;
    let marker = reader.string()?;
    let count = usize::from(reader.u16()?);
    if count == 0 || count > MAX_ITEMS {
        return Err(format!("invalid list item count {count}"));
    }
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let text = reader.string()?;
        let foreground = reader.rgb()?;
        items.push(ListItem::new(Span::styled(
            text,
            Style::default().fg(foreground),
        )));
    }
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    let selected_foreground = reader.rgb()?;
    let selected_background = reader.rgb()?;
    let highlight = if active { marker.as_str() } else { "  " };
    let highlight_style = if active {
        Style::default()
            .fg(selected_foreground)
            .bg(selected_background)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let list = List::new(items)
        .highlight_symbol(highlight)
        .highlight_style(highlight_style)
        .block(block(title, border, background));
    let mut state = ListState::default().with_selected(Some(selected % count));
    StatefulWidget::render(list, area, buffer, &mut state);
    Ok(())
}

fn render_gauge(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let label = reader.string()?;
    let ratio = f64::from(reader.u16()?.min(10_000)) / 10_000.0;
    let foreground = reader.rgb()?;
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    Gauge::default()
        .block(block(title, border, background))
        .gauge_style(
            Style::default()
                .fg(foreground)
                .bg(background)
                .add_modifier(Modifier::BOLD),
        )
        .ratio(ratio)
        .label(label)
        .render(area, buffer);
    Ok(())
}

fn render_table(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let selected = usize::from(reader.u16()?);
    let active = reader.u8()? != 0;
    let marker = reader.string()?;
    let columns = usize::from(reader.u16()?);
    if columns == 0 || columns > 64 {
        return Err(format!("invalid table column count {columns}"));
    }
    let mut widths = Vec::with_capacity(columns);
    for _ in 0..columns {
        widths.push(Constraint::Percentage(reader.u16()?));
    }
    let mut headers = Vec::with_capacity(columns);
    for _ in 0..columns {
        headers.push(reader.string()?);
    }
    let row_count = usize::from(reader.u16()?);
    if row_count == 0 || row_count > MAX_ITEMS {
        return Err(format!("invalid table row count {row_count}"));
    }
    let mut rows = Vec::with_capacity(row_count);
    for _ in 0..row_count {
        let mut cells = Vec::with_capacity(columns);
        for _ in 0..columns {
            cells.push(reader.string()?);
        }
        rows.push(Row::new(cells));
    }
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    let header_foreground = reader.rgb()?;
    let header_background = reader.rgb()?;
    let selected_foreground = reader.rgb()?;
    let selected_background = reader.rgb()?;
    let table = Table::new(rows, widths)
        .header(
            Row::new(headers)
                .style(
                    Style::default()
                        .fg(header_foreground)
                        .bg(header_background)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .row_highlight_style(if active {
            Style::default()
                .fg(selected_foreground)
                .bg(selected_background)
        } else {
            Style::default()
        })
        .highlight_symbol(if active { marker.as_str() } else { "  " })
        .column_spacing(1)
        .block(block(title, border, background));
    let mut state = TableState::default().with_selected(Some(selected % row_count));
    StatefulWidget::render(table, area, buffer, &mut state);
    Ok(())
}

fn render_sparkline(
    reader: &mut Reader<'_>,
    frame: Rect,
    buffer: &mut Buffer,
) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let count = usize::from(reader.u16()?);
    if count == 0 || count > MAX_ITEMS {
        return Err(format!("invalid sparkline value count {count}"));
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(u64::from(reader.u16()?));
    }
    let foreground = reader.rgb()?;
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    Sparkline::default()
        .data(&values)
        .style(Style::default().fg(foreground).bg(background))
        .block(block(title, border, background))
        .render(area, buffer);
    Ok(())
}

fn render_bar_chart(
    reader: &mut Reader<'_>,
    frame: Rect,
    buffer: &mut Buffer,
) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let bar_width = reader.u16()?;
    let bar_gap = reader.u16()?;
    let count = usize::from(reader.u16()?);
    if count == 0 || count > 128 {
        return Err(format!("invalid bar count {count}"));
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push((reader.string()?, u64::from(reader.u16()?)));
    }
    let foreground = reader.rgb()?;
    let value_foreground = reader.rgb()?;
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    let bars: Vec<Bar<'_>> = values
        .iter()
        .map(|(label, value)| {
            Bar::default()
                .value(*value)
                .label(Line::from(label.as_str()))
                .style(Style::default().fg(foreground))
                .value_style(
                    Style::default()
                        .fg(value_foreground)
                        .add_modifier(Modifier::BOLD),
                )
        })
        .collect();
    BarChart::default()
        .block(block(title, border, background))
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .render(area, buffer);
    Ok(())
}

fn render_chart(reader: &mut Reader<'_>, frame: Rect, buffer: &mut Buffer) -> Result<(), String> {
    let area = reader.rect(frame)?;
    let title = reader.string()?;
    let x_min = f64::from(reader.i32()?) / 100.0;
    let x_max = f64::from(reader.i32()?) / 100.0;
    let y_min = f64::from(reader.i32()?) / 100.0;
    let y_max = f64::from(reader.i32()?) / 100.0;
    if x_min >= x_max || y_min >= y_max {
        return Err("chart bounds must be increasing".into());
    }
    let count = usize::from(reader.u16()?);
    if count == 0 || count > 32 {
        return Err(format!("invalid chart dataset count {count}"));
    }
    let mut series = Vec::with_capacity(count);
    for _ in 0..count {
        let name = reader.string()?;
        let color = reader.rgb()?;
        let point_count = usize::from(reader.u16()?);
        if point_count == 0 || point_count > MAX_ITEMS {
            return Err(format!("invalid chart point count {point_count}"));
        }
        let mut points = Vec::with_capacity(point_count);
        for _ in 0..point_count {
            points.push((
                f64::from(reader.i32()?) / 100.0,
                f64::from(reader.i32()?) / 100.0,
            ));
        }
        series.push((name, color, points));
    }
    let background = reader.rgb()?;
    let border = reader.rgb()?;
    let datasets: Vec<Dataset<'_>> = series
        .iter()
        .map(|(name, color, points)| {
            Dataset::default()
                .name(name.as_str())
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(*color))
                .data(points)
        })
        .collect();
    Chart::new(datasets)
        .block(block(title, border, background))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels([
            Line::from(format!("{x_min:.0}")),
            Line::from(format!("{x_max:.0}")),
        ]))
        .y_axis(Axis::default().bounds([y_min, y_max]).labels([
            Line::from(format!("{y_min:.0}")),
            Line::from(format!("{y_max:.0}")),
        ]))
        .render(area, buffer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    fn u16_bytes(value: u16) -> [u8; 2] {
        value.to_le_bytes()
    }

    fn u32_bytes(value: u32) -> [u8; 4] {
        value.to_le_bytes()
    }

    fn text(output: &mut Vec<u8>, value: &str) {
        output.extend(u32_bytes(value.len() as u32));
        output.extend(value.as_bytes());
    }

    fn frame(width: u16, height: u16, commands: &[(u8, u8, Vec<u8>)]) -> Vec<u8> {
        let mut output = MAGIC.to_vec();
        output.extend(u16_bytes(VERSION));
        output.extend(u16_bytes(width));
        output.extend(u16_bytes(height));
        output.extend(u16_bytes(commands.len() as u16));
        for (opcode, flags, payload) in commands {
            output.push(*opcode);
            output.push(*flags);
            output.extend(u32_bytes(payload.len() as u32));
            output.extend(payload);
        }
        output
    }

    #[test]
    fn renders_a_unicode_paragraph_from_the_command_protocol() {
        let mut payload = Vec::new();
        payload.extend(u16_bytes(0));
        payload.extend(u16_bytes(0));
        payload.extend(u16_bytes(40));
        payload.extend(u16_bytes(10));
        payload.push(1);
        text(&mut payload, "Harbour");
        text(&mut payload, "Здравей • こんにちは");
        payload.extend([255, 255, 255]);
        payload.extend([10, 20, 30]);
        payload.extend([80, 210, 170]);
        payload.extend([1, 1, 1]);

        let rendered = render_commands(&frame(40, 10, &[(PARAGRAPH, 0, payload)]), false)
            .expect("valid paragraph command");
        let rendered = String::from_utf8(rendered).unwrap();
        assert!(rendered.contains("Здравей • こんにちは"));
        assert!(rendered.lines().all(|line| line.width() == 40));
    }

    #[test]
    fn renders_all_harbour_modifier_bits_as_ansi_sgr() {
        let mut payload = Vec::new();
        payload.extend(u16_bytes(0));
        payload.extend(u16_bytes(0));
        payload.extend(u16_bytes(40));
        payload.extend(u16_bytes(10));
        payload.push(0);
        text(&mut payload, "");
        text(&mut payload, "styled");
        payload.extend([240, 244, 255]);
        payload.extend([18, 24, 38]);
        payload.extend([114, 239, 221]);
        payload.push(0);
        payload.push(0);
        payload.push(
            MOD_BOLD
                | MOD_DIM
                | MOD_ITALIC
                | MOD_UNDERLINE
                | MOD_BLINK
                | MOD_REVERSE
                | MOD_CROSSED
                | MOD_RAPID_BLINK,
        );

        let rendered = render_commands(&frame(40, 10, &[(PARAGRAPH, 0, payload)]), true)
            .expect("valid styled paragraph command");
        let rendered = String::from_utf8(rendered).unwrap();
        for sgr in [
            "\x1b[1m", "\x1b[2m", "\x1b[3m", "\x1b[4m", "\x1b[5m", "\x1b[6m", "\x1b[7m", "\x1b[9m",
        ] {
            assert!(rendered.contains(sgr), "missing SGR sequence {sgr:?}");
        }
    }

    #[test]
    fn skips_unknown_optional_commands_but_rejects_required_ones() {
        let optional = frame(24, 9, &[(200, OPTIONAL, vec![1, 2, 3])]);
        assert!(render_commands(&optional, false).is_ok());

        let required = frame(24, 9, &[(200, 0, Vec::new())]);
        let error = render_commands(&required, false).unwrap_err();
        assert!(error.contains("unknown required command opcode 200"));
    }

    #[test]
    fn rejects_truncated_command_payloads() {
        let mut invalid = frame(24, 9, &[(CLEAR, 0, vec![0, 0])]);
        invalid.pop();
        let error = render_commands(&invalid, false).unwrap_err();
        assert!(error.contains("truncated command buffer"));
    }
}
