//! Conditional formats листа «Калькулятор» (badge / delta / hide).
//! Medium периметр секций — в Format; исключение: Ozon-низ «Затрат» (логистика) —
//! CF по ячейкам L/B / B / B / R+B, иначе одна CF bottom на диапазон затирает правый край.

use super::styles::{
    COL_BORDER, COL_DELTA_NEG, COL_DELTA_POS, COL_OZ_BADGE, COL_WB_BADGE, COL_WHITE,
};
use rust_xlsxwriter::{
    Color, ConditionalFormatFormula, Format, FormatBorder, Formula, Worksheet,
};

pub fn add_marketplace_cf(ws: &mut Worksheet, row: u32, col: u16) -> Result<(), String> {
    let wb = ConditionalFormatFormula::new()
        .set_rule(Formula::new("B2=\"WB\""))
        .set_format(
            Format::new()
                .set_background_color(Color::RGB(COL_WB_BADGE))
                .set_bold(),
        );
    ws.add_conditional_format(row, col, row, col, &wb)
        .map_err(|e| e.to_string())?;
    let oz = ConditionalFormatFormula::new()
        .set_rule(Formula::new("B2=\"OZON\""))
        .set_format(
            Format::new()
                .set_background_color(Color::RGB(COL_OZ_BADGE))
                .set_bold(),
        );
    ws.add_conditional_format(row, col, row, col, &oz)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn add_delta_cf(ws: &mut Worksheet, addr: &str) -> Result<(), String> {
    let (row, col) = parse_a1(addr).ok_or_else(|| format!("bad addr {addr}"))?;
    let pos_rule = format!("AND(ISNUMBER({addr}),{addr}>0)");
    let pos = ConditionalFormatFormula::new()
        .set_rule(Formula::new(pos_rule.as_str()))
        .set_format(
            Format::new()
                .set_font_color(Color::RGB(COL_DELTA_POS))
                .set_bold(),
        );
    ws.add_conditional_format(row, col, row, col, &pos)
        .map_err(|e| e.to_string())?;
    let neg_rule = format!("AND(ISNUMBER({addr}),{addr}<0)");
    let neg = ConditionalFormatFormula::new()
        .set_rule(Formula::new(neg_rule.as_str()))
        .set_format(
            Format::new()
                .set_font_color(Color::RGB(COL_DELTA_NEG))
                .set_bold(),
        );
    ws.add_conditional_format(row, col, row, col, &neg)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Целый чужой блок: белый текст + Thin вместо Medium (без заливки — сетка Excel остаётся).
fn hide_dxf() -> Format {
    Format::new()
        .set_font_color(Color::RGB(COL_WHITE))
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(COL_BORDER))
}

/// Строка под низом секции на Ozon: demote L/R/B, top не трогаем (Medium bottom сверху жив).
fn hide_dxf_keep_top() -> Format {
    Format::new()
        .set_font_color(Color::RGB(COL_WHITE))
        .set_border_left(FormatBorder::Thin)
        .set_border_right(FormatBorder::Thin)
        .set_border_bottom(FormatBorder::Thin)
        .set_border_left_color(Color::RGB(COL_BORDER))
        .set_border_right_color(Color::RGB(COL_BORDER))
        .set_border_bottom_color(Color::RGB(COL_BORDER))
}

/// Контент внутри общей секции: только белый шрифт — Medium периметра не трогаем.
fn hide_content_dxf() -> Format {
    Format::new().set_font_color(Color::RGB(COL_WHITE))
}

fn oz_rule(fmt: Format) -> ConditionalFormatFormula {
    ConditionalFormatFormula::new()
        .set_rule(Formula::new("$B$2=\"OZON\""))
        .set_format(fmt)
}

pub fn add_wb_hide_cf(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16) -> Result<(), String> {
    ws.add_conditional_format(r0, c0, r1, c1, &oz_rule(hide_dxf()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Demote L/R/B без top — для «Стоимость заказа» под Ozon-низом «Затрат».
pub fn add_wb_hide_keep_top_cf(
    ws: &mut Worksheet,
    r0: u32,
    c0: u16,
    r1: u32,
    c1: u16,
) -> Result<(), String> {
    ws.add_conditional_format(r0, c0, r1, c1, &oz_rule(hide_dxf_keep_top()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn add_wb_hide_content_cf(
    ws: &mut Worksheet,
    r0: u32,
    c0: u16,
    r1: u32,
    c1: u16,
) -> Result<(), String> {
    ws.add_conditional_format(r0, c0, r1, c1, &oz_rule(hide_content_dxf()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Скрыть Ozon-блок при WB (не при пустой площадке).
pub fn add_oz_hide_cf(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16) -> Result<(), String> {
    let rule = ConditionalFormatFormula::new()
        .set_rule(Formula::new("$B$2=\"WB\""))
        .set_format(hide_dxf());
    ws.add_conditional_format(r0, c0, r1, c1, &rule)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Ozon: низ «Затрат» на строке логистики — Medium по ячейкам (A: L+B, B–C: B, D: R+B).
/// Нельзя одним CF bottom на A:D — Excel затирает правый Medium у периметра.
pub fn add_cost_ozon_close_cf(
    ws: &mut Worksheet,
    row: u32,
    c0: u16,
    c1: u16,
    accent: u32,
) -> Result<(), String> {
    let a = Color::RGB(accent);
    let bottom = || {
        Format::new()
            .set_border_bottom(FormatBorder::Medium)
            .set_border_bottom_color(a)
    };
    let left_bottom = bottom()
        .set_border_left(FormatBorder::Medium)
        .set_border_left_color(a);
    let right_bottom = bottom()
        .set_border_right(FormatBorder::Medium)
        .set_border_right_color(a);

    ws.add_conditional_format(row, c0, row, c0, &oz_rule(left_bottom))
        .map_err(|e| e.to_string())?;
    if c1 > c0 + 1 {
        ws.add_conditional_format(row, c0 + 1, row, c1 - 1, &oz_rule(bottom()))
            .map_err(|e| e.to_string())?;
    }
    if c1 > c0 {
        ws.add_conditional_format(row, c1, row, c1, &oz_rule(right_bottom))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn parse_a1(addr: &str) -> Option<(u32, u16)> {
    let bytes = addr.as_bytes();
    let mut i = 0;
    let mut col: u32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        col = col * 26 + (bytes[i].to_ascii_uppercase() - b'A' + 1) as u32;
        i += 1;
    }
    if i == 0 || col == 0 {
        return None;
    }
    let row: u32 = addr[i..].parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, (col - 1) as u16))
}
