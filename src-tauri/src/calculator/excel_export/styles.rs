//! Палитра и форматы — как CalculatorViewPage / Styles.cs.

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

pub const COL_ID: u32 = 0x5B_8D_EF;
pub const COL_STOCK: u32 = 0x3D_BA_8A;
pub const COL_FUNNEL: u32 = 0xC9_A2_27;
pub const COL_PRICE: u32 = 0x9B_7B_DE;
pub const COL_COST: u32 = 0xE0_7A_5F;
pub const COL_PARAMS: u32 = 0x6A_9F_B5;
pub const COL_DOCS: u32 = 0x7A_8A_9A;
pub const COL_RATES: u32 = 0x90_90_90;
pub const COL_PROFIT_BG: u32 = 0xE8_F5_E9;
pub const COL_PROFIT_BORDER: u32 = 0x3D_BA_8A;
pub const COL_INPUT: u32 = 0xFF_FF_C8;
pub const COL_LABEL: u32 = 0x2A_2A_2A;
pub const COL_BORDER: u32 = 0xC5_C5_C5;
pub const COL_WB_BADGE: u32 = 0xDB_EA_FE;
pub const COL_OZ_BADGE: u32 = 0xED_E9_FE;
pub const COL_DELTA_POS: u32 = 0x1B_7A_3A;
pub const COL_DELTA_NEG: u32 = 0xC6_28_28;
pub const COL_WHITE: u32 = 0xFF_FF_FF;

pub const FMT_MONEY: &str = "#,##0.00";
/// Целое (остатки, «В желаемое») — без дробной части.
pub const FMT_INT: &str = "#,##0";
pub const FMT_PCT_DELTA: &str = "+0.0%;-0.0%;0%";
pub const FMT_RATING: &str = "0.00";
pub const FMT_SHEET_DATE: &str = "dd.MM.yyyy";
/// Текст (ТН-ВЭД и т.п.) — `"@"`.
pub const FMT_TEXT: &str = "@";

pub fn rgb(c: u32) -> Color {
    Color::RGB(c)
}

pub fn tint(accent: u32, white_mix: f64) -> u32 {
    let w = white_mix.clamp(0.0, 1.0);
    let r = ((accent >> 16) & 0xFF) as f64;
    let g = ((accent >> 8) & 0xFF) as f64;
    let b = (accent & 0xFF) as f64;
    let nr = (r + (255.0 - r) * w) as u32;
    let ng = (g + (255.0 - g) * w) as u32;
    let nb = (b + (255.0 - b) * w) as u32;
    (nr << 16) | (ng << 8) | nb
}

pub fn fmt_label() -> Format {
    Format::new()
        .set_font_color(rgb(COL_LABEL))
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_BORDER))
        .set_align(FormatAlign::Center)
}

pub fn fmt_label_left() -> Format {
    Format::new()
        .set_font_color(rgb(COL_LABEL))
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_BORDER))
}

pub fn fmt_value() -> Format {
    Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_BORDER))
        .set_align(FormatAlign::Center)
}

pub fn fmt_value_left() -> Format {
    Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_BORDER))
}

pub fn fmt_value_num(num_fmt: &str) -> Format {
    fmt_value().set_num_format(num_fmt)
}

pub fn fmt_input() -> Format {
    Format::new()
        .set_bold()
        .set_font_size(14)
        .set_background_color(rgb(COL_INPUT))
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(0xCC_AA_00))
        .set_align(FormatAlign::Center)
        .set_num_format("@")
}

pub fn fmt_section_title(accent: u32) -> Format {
    // Medium top/right в Format (не только CF): на merge CF-рамки рвутся.
    Format::new()
        .set_bold()
        .set_font_color(rgb(accent))
        .set_background_color(rgb(tint(accent, 0.88)))
        .set_border_left(FormatBorder::Thick)
        .set_border_left_color(rgb(accent))
        .set_border_top(FormatBorder::Medium)
        .set_border_top_color(rgb(accent))
        .set_border_right(FormatBorder::Medium)
        .set_border_right_color(rgb(accent))
}

pub fn fmt_profit_header() -> Format {
    Format::new()
        .set_bold()
        .set_font_color(rgb(COL_PROFIT_BORDER))
        .set_background_color(rgb(COL_PROFIT_BG))
        .set_border_top(FormatBorder::Medium)
        .set_border_top_color(rgb(COL_PROFIT_BORDER))
        .set_border_left(FormatBorder::Medium)
        .set_border_left_color(rgb(COL_PROFIT_BORDER))
        .set_border_right(FormatBorder::Medium)
        .set_border_right_color(rgb(COL_PROFIT_BORDER))
}

pub fn fmt_profit_label() -> Format {
    Format::new()
        .set_bold()
        .set_font_size(10)
        .set_font_color(rgb(COL_LABEL))
        .set_background_color(rgb(COL_PROFIT_BG))
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_PROFIT_BORDER))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
}

pub fn fmt_profit_value(num_fmt: &str) -> Format {
    Format::new()
        .set_bold()
        .set_font_size(16)
        .set_background_color(rgb(COL_PROFIT_BG))
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_PROFIT_BORDER))
        .set_border_bottom(FormatBorder::Medium)
        .set_border_bottom_color(rgb(COL_PROFIT_BORDER))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_num_format(num_fmt)
}

pub fn fmt_profit_value_right(num_fmt: &str) -> Format {
    fmt_profit_value(num_fmt)
        .set_border_right(FormatBorder::Medium)
        .set_border_right_color(rgb(COL_PROFIT_BORDER))
}

pub fn fmt_profit_value_left(num_fmt: &str) -> Format {
    fmt_profit_value(num_fmt)
        .set_border_left(FormatBorder::Medium)
        .set_border_left_color(rgb(COL_PROFIT_BORDER))
}

pub fn fmt_rate_value() -> Format {
    Format::new()
        .set_bold()
        .set_background_color(rgb(COL_INPUT))
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_RATES))
        .set_border_right(FormatBorder::Medium)
        .set_border_right_color(rgb(COL_RATES))
        .set_align(FormatAlign::Center)
}

/// Label ставки у левого края блока (Medium left).
pub fn fmt_rate_label() -> Format {
    fmt_label()
        .set_border_left(FormatBorder::Medium)
        .set_border_left_color(rgb(COL_RATES))
}

/// Нижняя строка ставок — Medium bottom на периметре.
pub fn fmt_rate_label_bottom() -> Format {
    fmt_rate_label()
        .set_border_bottom(FormatBorder::Medium)
        .set_border_bottom_color(rgb(COL_RATES))
}

pub fn fmt_rate_value_bottom() -> Format {
    fmt_rate_value()
        .set_border_bottom(FormatBorder::Medium)
        .set_border_bottom_color(rgb(COL_RATES))
}

pub fn fmt_a1_id() -> Format {
    Format::new()
        .set_bold()
        .set_font_size(11)
        .set_font_color(rgb(COL_ID))
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_BORDER))
}

pub fn fmt_title_calc() -> Format {
    Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_color(rgb(COL_ID))
        .set_border(FormatBorder::Thin)
        .set_border_color(rgb(COL_ID))
}

