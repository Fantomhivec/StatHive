//! Left columns of Calculator sheet.
use super::calc_cf::add_delta_cf;
use super::calc_layout::{
    paint_header, paint_header_formula, put_pair_box, put_wide_fmt_box, MetricEdge,
};
use super::formulas::{
    conv_formula, delta_formula, delta_formula_both, wb_only_label,
};
use super::styles::{
    COL_COST, COL_FUNNEL, COL_ID, COL_PRICE, COL_STOCK, FMT_INT, FMT_MONEY, FMT_PCT_DELTA,
    FMT_RATING,
};
use rust_xlsxwriter::Worksheet;

pub(crate) struct LeftCols {
    pub price_row: u32,
    pub comm_row: u32,
    pub logistics_row: u32,
    pub order_cost_row: u32,
    pub order_cost_hide: (u32, u16, u32, u16),
    pub left_bottom: u32,
    pub wb_hide: Vec<(u32, u16, u32, u16)>,
    pub wb_hide_content: Vec<(u32, u16, u32, u16)>,
}

pub(crate) fn write_left_cols(
    sheet: &mut Worksheet,
    l: &dyn Fn(&str, &str) -> String,
) -> Result<LeftCols, String> {
    let mut left: u32 = 7;
    let mut wb_hide: Vec<(u32, u16, u32, u16)> = Vec::new();
    let mut wb_hide_content: Vec<(u32, u16, u32, u16)> = Vec::new();

    paint_header(sheet, left, 0, 3, "Основная информация", COL_ID)?;
    left += 1;
    put_pair_box(sheet, left, "Кабинет", &l("Кабинет", "both"), "Бренд", &l("Бренд", "both"), None, None, false, COL_ID, false)?;
    left += 1;
    put_pair_box(
        sheet,
        left,
        "Артикул ПСТ",
        &format!("IF($B$2=\"OZON\",{},{})", l("Offer ID", "ozon"), l("Артикул ПСТ", "wb")),
        "Категория",
        &l("Категория", "both"),
        None,
        None,
        false,
        COL_ID,
        false,
    )?;
    left += 1;
    put_wide_fmt_box(
        sheet,
        left,
        0,
        3,
        "Наименование",
        &l("Наименование", "both"),
        false,
        None,
        Some(MetricEdge {
            accent: COL_ID,
            left: true,
            right: true,
            bottom: true,
        }),
    )?;
    left += 2;

    paint_header(sheet, left, 0, 3, "Остатки и рейтинг", COL_STOCK)?;
    left += 1;
    put_pair_box(sheet, left, "Остаток FBO", &l("Остаток FBO", "both"), "Остаток FBS", &l("Остаток FBS", "both"), None, None, true, COL_STOCK, false)?;
    left += 1;
    put_pair_box(
        sheet,
        left,
        "Рейтинг",
        &l("Рейтинг", "both"),
        "Отзывы",
        &l("Отзывы", "both"),
        Some(FMT_RATING),
        None,
        true,
        COL_STOCK,
        true,
    )?;
    left += 2;

    // Заказы сразу под заголовком; клики/корзина — ниже Затрат.
    paint_header_formula(sheet, left, 0, 3, "IF($B$2=\"OZON\",\"Заказы\",\"Воронка\")", COL_FUNNEL)?;
    left += 1;
    let orders = format!("B{}", left + 1);
    let orders_prev = format!("D{}", left + 1);
    put_pair_box(sheet, left, "Заказы", &l("Заказы", "both"), "Заказы (прошл.)", &l("Заказы (прошл.)", "both"), None, None, true, COL_FUNNEL, false)?;
    left += 1;
    let orders_delta = format!("B{}", left + 1);
    let orders_delta_row = left;
    put_pair_box(
        sheet,
        left,
        "Динамика заказов",
        &delta_formula_both(&orders, &orders_prev),
        &wb_only_label("Выкупы"),
        &l("Выкупы", "wb"),
        Some(FMT_PCT_DELTA),
        None,
        true,
        COL_FUNNEL,
        true,
    )?;
    wb_hide_content.push((orders_delta_row, 2, orders_delta_row, 3));
    add_delta_cf(sheet, &orders_delta)?;
    left += 2;

    paint_header(sheet, left, 0, 3, "Цены", COL_PRICE)?;
    left += 1;
    put_pair_box(
        sheet,
        left,
        "Цена",
        &format!("IF($B$2=\"OZON\",{},{})", l("Цена после скидки", "ozon"), l("Цена", "wb")),
        "Скидка, %",
        &l("Скидка", "both"),
        Some(FMT_MONEY),
        None,
        true,
        COL_PRICE,
        false,
    )?;
    left += 1;
    let price_row = left;
    put_pair_box(
        sheet,
        left,
        "Цена после скидки / акций",
        &format!("IF($B$2=\"OZON\",{},{})", l("Цена после акций", "ozon"), l("Цена после скидки", "wb")),
        "Закуп",
        &l("Закуп", "both"),
        Some(FMT_MONEY),
        Some(FMT_MONEY),
        true,
        COL_PRICE,
        true,
    )?;
    left += 2;

    paint_header(sheet, left, 0, 3, "Затраты", COL_COST)?;
    left += 1;
    let comm_row = left;
    put_pair_box(sheet, left, "Комиссия FBO %", &l("Комиссия FBO", "both"), "Комиссия FBS %", &l("Комиссия FBS", "both"), None, None, true, COL_COST, false)?;
    left += 1;
    let logistics_row = left;
    // bottom=false: низ секции на WB — «Стоимость заказа» (не резать блок внутренней Medium).
    put_pair_box(
        sheet,
        left,
        "Логистика",
        &l("Логистика", "both"),
        &wb_only_label("Расходы 2 нед."),
        &l("Расходы 2 нед.", "wb"),
        Some(FMT_MONEY),
        Some(FMT_MONEY),
        true,
        COL_COST,
        false,
    )?;
    left += 1;
    let order_cost_row = left;
    put_pair_box(
        sheet,
        left,
        &wb_only_label("Стоимость заказа"),
        &l("Стоимость заказа", "wb"),
        "",
        "\"\"",
        Some(FMT_MONEY),
        None,
        true,
        COL_COST,
        true,
    )?;
    // Ozon: низ блока = логистика (CF close); order_cost demote L/R/B keep_top.
    // WB: Format без изменений (низ = order_cost).
    wb_hide_content.push((logistics_row, 2, logistics_row, 3));
    let order_cost_hide = (order_cost_row, 0u16, order_cost_row, 3u16);
    left += 2;

    // WB-only воронка после Затрат.
    let wb_funnel_start = left;
    paint_header_formula(sheet, left, 0, 3, &wb_only_label("Воронка WB"), COL_FUNNEL)?;
    left += 1;
    let clicks = format!("B{}", left + 1);
    let clicks_prev = format!("D{}", left + 1);
    put_pair_box(sheet, left, &wb_only_label("Клики"), &l("Клики", "wb"), &wb_only_label("Клики (прошл.)"), &l("Клики (прошл.)", "wb"), None, None, true, COL_FUNNEL, false)?;
    left += 1;
    let clicks_delta = format!("B{}", left + 1);
    // fmt2 явно FMT_INT: иначе or(fmt1) наследует FMT_PCT_DELTA у «В желаемое».
    put_pair_box(
        sheet,
        left,
        &wb_only_label("Динамика кликов"),
        &delta_formula(&clicks, &clicks_prev),
        &wb_only_label("В желаемое"),
        &l("В желаемое", "wb"),
        Some(FMT_PCT_DELTA),
        Some(FMT_INT),
        true,
        COL_FUNNEL,
        false,
    )?;
    left += 1;
    let cart = format!("B{}", left + 1);
    let cart_prev = format!("D{}", left + 1);
    put_pair_box(sheet, left, &wb_only_label("Корзина"), &l("Корзина", "wb"), &wb_only_label("Корзина (прошл.)"), &l("Корзина (прошл.)", "wb"), None, None, true, COL_FUNNEL, false)?;
    left += 1;
    let cart_delta = format!("B{}", left + 1);
    put_pair_box(sheet, left, &wb_only_label("Динамика корзины"), &delta_formula(&cart, &cart_prev), &wb_only_label("Есть реклама"), &l("Есть реклама", "wb"), Some(FMT_PCT_DELTA), None, true, COL_FUNNEL, false)?;
    left += 1;
    put_pair_box(sheet, left, &wb_only_label("Выкупы (прошл.)"), &l("Выкупы (прошл.)", "wb"), &wb_only_label("Конверсия клик→корзина"), &conv_formula(&cart, &clicks), None, Some(FMT_PCT_DELTA), true, COL_FUNNEL, false)?;
    left += 1;
    put_pair_box(
        sheet,
        left,
        &wb_only_label("Конверсия корзина→заказ"),
        &conv_formula(&orders, &cart),
        &wb_only_label("Конверсия клик→заказ"),
        &conv_formula(&orders, &clicks),
        Some(FMT_PCT_DELTA),
        Some(FMT_PCT_DELTA),
        true,
        COL_FUNNEL,
        true,
    )?;
    let wb_funnel_end = left;
    wb_hide.push((wb_funnel_start, 0, wb_funnel_end, 3));
    add_delta_cf(sheet, &clicks_delta)?;
    add_delta_cf(sheet, &cart_delta)?;
    left += 2;
    let left_bottom = left;

    Ok(LeftCols {
        price_row,
        comm_row,
        logistics_row,
        order_cost_row,
        order_cost_hide,
        left_bottom,
        wb_hide,
        wb_hide_content,
    })
}
