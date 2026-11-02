//! Хелперы раскладки листа «Калькулятор». Medium-рамка — только в Format при записи (BoxSection).

use super::styles::{
    fmt_label, fmt_label_left, fmt_section_title, fmt_value, fmt_value_left, fmt_value_num,
};
use super::write_sheets::{merge_formula, merge_write, write_formula, write_string};
use rust_xlsxwriter::{Color, Format, FormatBorder, Formula, Worksheet};

pub fn paint_header(
    ws: &mut Worksheet,
    row: u32,
    c0: u16,
    c1: u16,
    title: &str,
    accent: u32,
) -> Result<(), String> {
    merge_write(ws, row, c0, row, c1, title, &fmt_section_title(accent))
}

pub fn paint_header_formula(
    ws: &mut Worksheet,
    row: u32,
    c0: u16,
    c1: u16,
    formula: &str,
    accent: u32,
) -> Result<(), String> {
    merge_formula(ws, row, c0, row, c1, formula, &fmt_section_title(accent))
}

pub fn put_pair_box(
    ws: &mut Worksheet,
    row: u32,
    l1: &str,
    f1: &str,
    l2: &str,
    f2: &str,
    fmt1: Option<&str>,
    fmt2: Option<&str>,
    center: bool,
    accent: u32,
    bottom: bool,
) -> Result<(), String> {
    put_pair_at_box(ws, row, 0, l1, f1, l2, f2, fmt1, fmt2, center, accent, bottom)
}

pub fn put_pair_at_box(
    ws: &mut Worksheet,
    row: u32,
    start: u16,
    l1: &str,
    f1: &str,
    l2: &str,
    f2: &str,
    fmt1: Option<&str>,
    fmt2: Option<&str>,
    center: bool,
    accent: u32,
    bottom: bool,
) -> Result<(), String> {
    let has_second = !l2.is_empty() || f2 != "\"\"";
    // Всегда 4 колонки: Medium left на start, Medium right на start+3 (плейсхолдер справа).
    put_metric_edge(
        ws,
        row,
        start,
        l1,
        f1,
        fmt1,
        center,
        MetricEdge {
            accent,
            left: true,
            right: false,
            bottom,
        },
    )?;
    if has_second {
        put_metric_edge(
            ws,
            row,
            start + 2,
            l2,
            f2,
            fmt2.or(fmt1),
            center,
            MetricEdge {
                accent,
                left: false,
                right: true,
                bottom,
            },
        )?;
    } else {
        put_metric_edge(
            ws,
            row,
            start + 2,
            "",
            "",
            None,
            center,
            MetricEdge {
                accent,
                left: false,
                right: true,
                bottom,
            },
        )?;
    }
    Ok(())
}

pub fn put_wide_fmt_box(
    ws: &mut Worksheet,
    row: u32,
    c0: u16,
    c1: u16,
    label: &str,
    formula: &str,
    center: bool,
    num_fmt: Option<&str>,
    edge: Option<MetricEdge>,
) -> Result<(), String> {
    put_wide_fmt_box_align(ws, row, c0, c1, label, formula, center, center, num_fmt, edge)
}

/// Wide-метрика: выравнивание подписи и значения раздельно (ТН-ВЭД: label center, value left).
pub fn put_wide_fmt_box_align(
    ws: &mut Worksheet,
    row: u32,
    c0: u16,
    c1: u16,
    label: &str,
    formula: &str,
    label_center: bool,
    value_center: bool,
    num_fmt: Option<&str>,
    edge: Option<MetricEdge>,
) -> Result<(), String> {
    let edge = edge.unwrap_or_default();
    put_metric_edge_align(
        ws,
        row,
        c0,
        label,
        formula,
        num_fmt,
        label_center,
        value_center,
        MetricEdge {
            right: false,
            ..edge
        },
    )?;
    if c1 > c0 + 1 {
        let fmt = with_metric_edge(
            match num_fmt {
                Some(n) if value_center => fmt_value_num(n).set_text_wrap(),
                Some(n) => fmt_value_left().set_num_format(n).set_text_wrap(),
                None if value_center => fmt_value().set_text_wrap(),
                None => fmt_value_left().set_text_wrap(),
            },
            MetricEdge {
                left: false,
                ..edge
            },
        );
        ws.merge_range(row, c0 + 1, row, c1, "", &fmt)
            .map_err(|e| e.to_string())?;
        write_formula(ws, row, c0 + 1, formula, &fmt)?;
    }
    Ok(())
}

/// Внешние Medium на периметре блока (в Format ячейки).
#[derive(Clone, Copy, Default)]
pub struct MetricEdge {
    pub accent: u32,
    pub left: bool,
    pub right: bool,
    pub bottom: bool,
}

fn with_metric_edge(fmt: Format, edge: MetricEdge) -> Format {
    let mut f = fmt;
    let a = Color::RGB(edge.accent);
    if edge.left {
        f = f.set_border_left(FormatBorder::Medium).set_border_left_color(a);
    }
    if edge.right {
        f = f.set_border_right(FormatBorder::Medium).set_border_right_color(a);
    }
    if edge.bottom {
        f = f.set_border_bottom(FormatBorder::Medium).set_border_bottom_color(a);
    }
    f
}

fn put_metric_edge(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    label: &str,
    formula: &str,
    num_fmt: Option<&str>,
    center: bool,
    edge: MetricEdge,
) -> Result<(), String> {
    put_metric_edge_align(ws, row, col, label, formula, num_fmt, center, center, edge)
}

fn put_metric_edge_align(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    label: &str,
    formula: &str,
    num_fmt: Option<&str>,
    label_center: bool,
    value_center: bool,
    edge: MetricEdge,
) -> Result<(), String> {
    let lf = with_metric_edge(
        if label_center {
            fmt_label()
        } else {
            fmt_label_left()
        },
        MetricEdge {
            right: false,
            ..edge
        },
    );
    if label.starts_with("IF(") {
        write_formula(ws, row, col, label, &lf)?;
    } else {
        // Пустая подпись — плейсхолдер под будущую метрику (нужны borders).
        write_string(ws, row, col, label, &lf)?;
    }
    let vf = with_metric_edge(
        match (num_fmt, value_center) {
            (Some(n), true) => fmt_value_num(n),
            (Some(n), false) => fmt_value_left().set_num_format(n),
            (None, true) => fmt_value(),
            (None, false) => fmt_value_left(),
        },
        MetricEdge {
            left: false,
            ..edge
        },
    );
    if formula.is_empty() || formula == "\"\"" {
        write_string(ws, row, col + 1, "", &vf)?;
    } else {
        write_formula(ws, row, col + 1, formula, &vf)?;
    }
    Ok(())
}

pub fn put_rate(
    ws: &mut Worksheet,
    row: u32,
    label_col: u16,
    label: &str,
    aa_row: u32,
    value: f64,
) -> Result<(), String> {
    put_rate_edge(ws, row, label_col, label, aa_row, value, false)
}

pub fn put_rate_bottom(
    ws: &mut Worksheet,
    row: u32,
    label_col: u16,
    label: &str,
    aa_row: u32,
    value: f64,
) -> Result<(), String> {
    put_rate_edge(ws, row, label_col, label, aa_row, value, true)
}

fn put_rate_edge(
    ws: &mut Worksheet,
    row: u32,
    label_col: u16,
    label: &str,
    aa_row: u32,
    value: f64,
    bottom: bool,
) -> Result<(), String> {
    use super::styles::{
        fmt_rate_label, fmt_rate_label_bottom, fmt_rate_value, fmt_rate_value_bottom,
    };
    let lf = if bottom {
        fmt_rate_label_bottom()
    } else {
        fmt_rate_label()
    };
    let vf = if bottom {
        fmt_rate_value_bottom()
    } else {
        fmt_rate_value()
    };
    write_string(ws, row, label_col, label, &lf)?;
    let val_col = label_col + 1;
    ws.write_number_with_format(row, val_col, value, &vf)
        .map_err(|e| e.to_string())?;
    let cell = format!("{}{}", col_letter(val_col), row + 1);
    ws.write_formula(aa_row, 26, Formula::new(cell.as_str()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn col_letter(col0: u16) -> String {
    let mut n = col0 as u32 + 1;
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

/// Ссылка «Открыть фото» с Medium-периметром блока Фото.
pub fn fmt_photo_cell(accent: u32) -> Format {
    with_metric_edge(
        fmt_value(),
        MetricEdge {
            accent,
            left: true,
            right: true,
            bottom: true,
        },
    )
}
