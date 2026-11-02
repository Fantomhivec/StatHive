//! Right columns of Calculator sheet.
use super::calc_layout::{
    fmt_photo_cell, paint_header, paint_header_formula, put_pair_at_box, put_rate, put_rate_bottom,
    put_wide_fmt_box, put_wide_fmt_box_align, MetricEdge,
};
use super::formulas::wb_only_label;
use super::styles::{
    COL_DOCS, COL_PARAMS, COL_RATES, FMT_SHEET_DATE, FMT_TEXT,
};
use super::write_sheets::merge_formula;
use crate::calculator::profile::CalculatorProfile;
use rust_xlsxwriter::{Formula, Worksheet};

pub(crate) struct RightCols {
    pub oz_hide: (u32, u16, u32, u16),
    pub right: u32,
    pub wb_hide: Vec<(u32, u16, u32, u16)>,
}

pub(crate) fn write_right_cols(
    sheet: &mut Worksheet,
    profile: &CalculatorProfile,
    l: &dyn Fn(&str, &str) -> String,
    lt: &dyn Fn(&str, &str) -> String,
) -> Result<RightCols, String> {
    let mut wb_hide: Vec<(u32, u16, u32, u16)> = Vec::new();
    let mut right: u32 = 7;
    const RC0: u16 = 5;
    const RC1: u16 = 8;

    let photo_f = l("Фото", "both");
    sheet
        .write_formula(19, 26, Formula::new(photo_f.as_str()))
        .map_err(|e| e.to_string())?;
    paint_header(sheet, right, RC0, RC1, "Фото", COL_PARAMS)?;
    right += 1;
    let photo_row = right;
    merge_formula(
        sheet,
        photo_row,
        RC0,
        photo_row,
        RC1,
        "IF($AA$20=\"\",\"—\",IFERROR(HYPERLINK($AA$20,\"Открыть фото\"),\"—\"))",
        &fmt_photo_cell(COL_PARAMS),
    )?;
    sheet.set_row_height(photo_row, 22).ok();
    right += 2;

    paint_header(sheet, right, RC0, RC1, "Параметры", COL_PARAMS)?;
    right += 1;
    put_pair_at_box(sheet, right, RC0, "Длина, см", &l("Длина", "both"), "Ширина, см", &l("Ширина", "both"), None, None, true, COL_PARAMS, false)?;
    right += 1;
    put_pair_at_box(sheet, right, RC0, "Высота, см", &l("Высота", "both"), "Вес, кг", &l("Вес", "both"), None, None, true, COL_PARAMS, false)?;
    right += 1;
    put_pair_at_box(sheet, right, RC0, "Объём, л", &l("Объём", "both"), "", "\"\"", None, None, true, COL_PARAMS, false)?;
    right += 1;
    // Подпись по центру, значение слева.
    put_wide_fmt_box_align(
        sheet,
        right,
        RC0,
        RC1,
        "ТН-ВЭД",
        &lt("ТН-ВЭД", "both"),
        true,
        false,
        Some(FMT_TEXT),
        Some(MetricEdge {
            accent: COL_PARAMS,
            left: true,
            right: true,
            bottom: true,
        }),
    )?;
    right += 2;

    let rates_end: u16 = 6;
    let rates_oz = right;
    paint_header(sheet, right, RC0, rates_end, "Ставки OZON", COL_RATES)?;
    right += 1;
    put_rate(sheet, right, RC0, "Налог %", 7, profile.ozon_tax_percent)?;
    right += 1;
    put_rate(sheet, right, RC0, "Эквайринг %", 8, profile.ozon_acquiring_percent)?;
    right += 1;
    put_rate(sheet, right, RC0, "Реклама %", 9, profile.ozon_adv_percent)?;
    right += 1;
    put_rate_bottom(sheet, right, RC0, "Доп. расходы %", 10, profile.ozon_extra_expenses_percent)?;
    let rates_oz_end = right;
    let oz_hide = (rates_oz, RC0, rates_oz_end, rates_end);
    right += 2;

    let docs_start = right;
    paint_header_formula(sheet, right, RC0, RC1, &wb_only_label("Документы"), COL_DOCS)?;
    right += 1;
    put_wide_fmt_box(
        sheet,
        right,
        RC0,
        RC1,
        &wb_only_label("Номер серт./декл."),
        &lt("Номер сертификата/декларации", "wb"),
        true,
        None,
        Some(MetricEdge {
            accent: COL_DOCS,
            left: true,
            right: true,
            bottom: false,
        }),
    )?;
    right += 1;
    put_wide_fmt_box(
        sheet,
        right,
        RC0,
        RC1,
        &wb_only_label("Дата регистрации"),
        &lt("Дата регистрации сертификата/декларации", "wb"),
        true,
        Some(FMT_SHEET_DATE),
        Some(MetricEdge {
            accent: COL_DOCS,
            left: true,
            right: true,
            bottom: false,
        }),
    )?;
    right += 1;
    put_wide_fmt_box(
        sheet,
        right,
        RC0,
        RC1,
        &wb_only_label("Дата окончания"),
        &lt("Дата окончания действия сертификата/декларации", "wb"),
        true,
        Some(FMT_SHEET_DATE),
        Some(MetricEdge {
            accent: COL_DOCS,
            left: true,
            right: true,
            bottom: false,
        }),
    )?;
    right += 1;
    put_wide_fmt_box(
        sheet,
        right,
        RC0,
        RC1,
        &wb_only_label("Статус проверки"),
        &lt("Статус проверки", "wb"),
        true,
        None,
        Some(MetricEdge {
            accent: COL_DOCS,
            left: true,
            right: true,
            bottom: true,
        }),
    )?;
    let docs_end = right;
    wb_hide.push((docs_start, RC0, docs_end, RC1));
    right += 2;

    let rates_wb = right;
    paint_header(sheet, right, RC0, rates_end, "Ставки WB", COL_RATES)?;
    right += 1;
    put_rate(sheet, right, RC0, "Налог %", 1, profile.tax_percent)?;
    right += 1;
    put_rate(sheet, right, RC0, "Эквайринг %", 2, profile.acquiring_percent)?;
    right += 1;
    put_rate(sheet, right, RC0, "Реклама %", 3, profile.adv_percent)?;
    right += 1;
    put_rate_bottom(sheet, right, RC0, "Доп. расходы %", 4, profile.extra_expenses_percent)?;
    let rates_wb_end = right;
    wb_hide.push((rates_wb, RC0, rates_wb_end, rates_end));

    Ok(RightCols {
        oz_hide,
        right,
        wb_hide,
    })
}
