//! Лист «Калькулятор» — блоки/стили CalculatorSheet.cs.
//! Medium в Format; Ozon-низ «Затрат» — CF close на логистике + demote order_cost keep_top.

use super::calc_cf::{
    add_cost_ozon_close_cf, add_delta_cf, add_marketplace_cf, add_oz_hide_cf, add_wb_hide_cf,
    add_wb_hide_content_cf, add_wb_hide_keep_top_cf,
};
use super::formulas::{
    build_profit_excel_formula, lookup_formula,
    lookup_formula_text, SHEET_CALC, SHEET_MAP,
};
use super::styles::{
    fmt_a1_id, fmt_input, fmt_label, fmt_profit_header, fmt_profit_label, fmt_profit_value,
    fmt_profit_value_left, fmt_profit_value_right, fmt_title_calc, COL_COST, FMT_MONEY, FMT_PCT_DELTA,
};
use super::write_sheets::{merge_write, write_formula, write_string};
use crate::calculator::profile::CalculatorProfile;
use rust_xlsxwriter::{Format, Formula, Workbook};

pub fn write_calculator_sheet(
    wb: &mut Workbook,
    profile: &CalculatorProfile,
    wb_rows: usize,
    oz_rows: usize,
) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name(SHEET_CALC).map_err(|e| e.to_string())?;

    let wb_last = (wb_rows + 1).max(2);
    let oz_last = (oz_rows + 1).max(2);
    let map_last = (wb_rows + oz_rows + 1).max(2);
    let l = |header: &str, scope: &str| lookup_formula(header, scope, wb_last, oz_last);
    let lt = |header: &str, scope: &str| lookup_formula_text(header, scope, wb_last, oz_last);

    sheet.write_string(0, 26, "rates").map_err(|e| e.to_string())?;
    sheet.write_number(1, 26, profile.tax_percent).map_err(|e| e.to_string())?;
    sheet.write_number(2, 26, profile.acquiring_percent).map_err(|e| e.to_string())?;
    sheet.write_number(3, 26, profile.adv_percent).map_err(|e| e.to_string())?;
    sheet.write_number(4, 26, profile.extra_expenses_percent).map_err(|e| e.to_string())?;
    sheet.write_number(7, 26, profile.ozon_tax_percent).map_err(|e| e.to_string())?;
    sheet.write_number(8, 26, profile.ozon_acquiring_percent).map_err(|e| e.to_string())?;
    sheet.write_number(9, 26, profile.ozon_adv_percent).map_err(|e| e.to_string())?;
    sheet.write_number(10, 26, profile.ozon_extra_expenses_percent).map_err(|e| e.to_string())?;
    sheet.set_column_hidden(26).ok();
    sheet.set_column_hidden(27).ok();

    write_string(sheet, 0, 0, "Артикул", &fmt_a1_id())?;
    write_string(sheet, 0, 1, "", &fmt_input())?;
    write_string(sheet, 1, 0, "Площадка", &fmt_label())?;
    write_formula(
        sheet,
        1,
        1,
        &format!(
            "IFERROR(INDEX('{SHEET_MAP}'!$B$2:$B${map_last},MATCH($B$1,'{SHEET_MAP}'!$A$2:$A${map_last},0)),\"\")"
        ),
        &Format::new()
            .set_bold()
            .set_align(rust_xlsxwriter::FormatAlign::Center),
    )?;
    add_marketplace_cf(sheet, 1, 1)?;
    merge_write(sheet, 2, 0, 2, 6, "Калькулятор", &fmt_title_calc())?;

    merge_write(sheet, 3, 0, 3, 6, "Прибыль", &fmt_profit_header())?;
    merge_write(sheet, 4, 0, 4, 1, "Прибыль, руб", &fmt_profit_label())?;
    merge_write(sheet, 4, 2, 4, 3, "Прибыль, %", &fmt_profit_label())?;
    merge_write(sheet, 4, 4, 4, 5, "Прибыль факт, руб", &fmt_profit_label())?;
    write_string(sheet, 4, 6, "Прибыль факт, %", &fmt_profit_label())?;
    merge_write(sheet, 5, 0, 5, 1, "", &fmt_profit_value_left(FMT_MONEY))?;
    merge_write(sheet, 5, 2, 5, 3, "", &fmt_profit_value(FMT_PCT_DELTA))?;
    merge_write(sheet, 5, 4, 5, 5, "", &fmt_profit_value(FMT_MONEY))?;
    write_string(sheet, 5, 6, "", &fmt_profit_value_right(FMT_PCT_DELTA))?;
    sheet.set_row_height(4, 18).ok();
    sheet.set_row_height(5, 28).ok();


    let left = super::calc_sheet_left::write_left_cols(sheet, &l)?;
    let mut wb_hide = left.wb_hide;
    let wb_hide_content = left.wb_hide_content;
    let price_row = left.price_row;
    let comm_row = left.comm_row;
    let logistics_row = left.logistics_row;
    let order_cost_row = left.order_cost_row;
    let order_cost_hide = left.order_cost_hide;
    let left_bottom = left.left_bottom;

    let right_cols = super::calc_sheet_right::write_right_cols(sheet, profile, &l, &lt)?;
    wb_hide.extend(right_cols.wb_hide);
    let oz_hide = right_cols.oz_hide;
    let right = right_cols.right;


    sheet.write_formula(1, 27, Formula::new("IF($B$2=\"OZON\",$AA$8,$AA$2)/100")).map_err(|e| e.to_string())?;
    sheet.write_formula(2, 27, Formula::new("IF($B$2=\"OZON\",$AA$9,$AA$3)/100")).map_err(|e| e.to_string())?;
    sheet.write_formula(3, 27, Formula::new("IF($B$2=\"OZON\",$AA$10,$AA$4)/100")).map_err(|e| e.to_string())?;
    sheet.write_formula(4, 27, Formula::new("IF($B$2=\"OZON\",$AA$11,$AA$5)/100")).map_err(|e| e.to_string())?;
    sheet.write_formula(5, 27, Formula::new("IF($B$2=\"OZON\",$AA$10,$AA$4)")).map_err(|e| e.to_string())?;

    let price_ref = format!("B{}", price_row + 1);
    let purchase_ref = format!("D{}", price_row + 1);
    let comm_fbo = format!("B{}", comm_row + 1);
    let comm_fbs = format!("D{}", comm_row + 1);
    let logistics_ref = format!("B{}", logistics_row + 1);
    let order_cost_ref = format!("B{}", order_cost_row + 1);
    let wb_profit = build_profit_excel_formula(
        &profile.profit_formula, &price_ref, &purchase_ref, &comm_fbo, &comm_fbs, &logistics_ref,
    );
    let oz_profit = build_profit_excel_formula(
        &profile.ozon_profit_formula, &price_ref, &purchase_ref, &comm_fbo, &comm_fbs, &logistics_ref,
    );

    write_formula(sheet, 5, 0, &format!("IF($B$2=\"OZON\",{oz_profit},{wb_profit})"), &fmt_profit_value_left(FMT_MONEY))?;
    write_formula(sheet, 5, 2, &format!("IF(OR({price_ref}=\"\",N({price_ref})=0),\"\",IFERROR(A6/N({price_ref}),\"\"))"), &fmt_profit_value(FMT_PCT_DELTA))?;
    write_formula(sheet, 5, 4, &format!("IF(A6=\"\",\"\",IFERROR(A6+N({price_ref})*($AB$6/100)-IF($B$2=\"OZON\",0,N({order_cost_ref})),\"\"))"), &fmt_profit_value(FMT_MONEY))?;
    write_formula(sheet, 5, 6, &format!("IF(OR({price_ref}=\"\",N({price_ref})=0,E6=\"\"),\"\",IFERROR(E6/N({price_ref}),\"\"))"), &fmt_profit_value_right(FMT_PCT_DELTA))?;
    add_delta_cf(sheet, "A6")?;
    add_delta_cf(sheet, "C6")?;
    add_delta_cf(sheet, "E6")?;
    add_delta_cf(sheet, "G6")?;

    for (r0, c0, r1, c1) in wb_hide_content {
        add_wb_hide_content_cf(sheet, r0, c0, r1, c1)?;
    }
    for (r0, c0, r1, c1) in wb_hide {
        add_wb_hide_cf(sheet, r0, c0, r1, c1)?;
    }
    add_wb_hide_keep_top_cf(
        sheet,
        order_cost_hide.0,
        order_cost_hide.1,
        order_cost_hide.2,
        order_cost_hide.3,
    )?;
    add_oz_hide_cf(sheet, oz_hide.0, oz_hide.1, oz_hide.2, oz_hide.3)?;
    add_cost_ozon_close_cf(sheet, logistics_row, 0, 3, COL_COST)?;

    let _ = left_bottom.max(right);
    const BASE: f64 = 12.0;
    sheet.set_column_width(0, 24.0).ok();
    sheet.set_column_width(1, BASE * 3.0 * 0.8).ok();
    sheet.set_column_width(2, BASE * 1.25 * 1.15).ok();
    sheet.set_column_width(3, BASE * 3.0 * 0.8).ok();
    sheet.set_column_width(4, 2.5).ok();
    sheet.set_column_width(5, BASE * 1.35).ok();
    sheet.set_column_width(6, BASE * 3.0 * 0.9 * 0.5).ok();
    sheet.set_column_width(7, BASE).ok();
    sheet.set_column_width(8, BASE * 2.0).ok();
    let wrap = Format::new().set_text_wrap();
    sheet.set_column_format(1, &wrap).ok();
    sheet.set_column_format(3, &wrap).ok();
    sheet.set_column_format(6, &wrap).ok();
    sheet.set_column_format(8, &wrap).ok();
    sheet.set_freeze_panes(6, 0).ok();
    Ok(())
}
