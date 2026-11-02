//! Принудительные поля шаблонов калькулятора (паритет ApplyCalculatorScriptExtras).

use crate::calculator::catalog::{self, TableDef};
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util;
use crate::core::log::AppLog;
use crate::core::script_field::ScriptField;
use crate::templates::Template;
use chrono::{Duration, Local};

/// Включить поля, нужные своду/остаткам, даже если в DefaultFields Selected=false.
pub fn apply_calculator_script_extras(
    profile: &CalculatorProfile,
    table: &TableDef,
    tpl: &mut Template,
    defaults: &[ScriptField],
    log: &AppLog,
) {
    if table.key.eq_ignore_ascii_case("Array") {
        force_select(
            tpl,
            defaults,
            &[
                "cabinet",
                "nmID",
                "photos",
                "subjectID",
                "subjectName",
                "dimensions.length",
                "dimensions.width",
                "dimensions.height",
                "dimensions.weightBrutto",
                "sizes.chrtID",
                "tnved",
                "doc_number",
                "cert_reg_date",
                "cert_expire_date",
                "cert_check_status",
            ],
            "[Калькулятор] Массив: photos, subjectID, габариты/вес, chrtID, ТН-ВЭД, документы — для карточки/остатков.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("Funnel") {
        // : DateTemplateType=2weeks + RefreshDatesFromTemplateIfNeeded (вчера−13…вчера).
        let end = Local::now().date_naive() - Duration::days(1);
        let start = end - Duration::days(13);
        tpl.date_from = start.format("%Y-%m-%d").to_string();
        tpl.date_to = end.format("%Y-%m-%d").to_string();
        tpl.date_template_type = "2weeks".into();
        force_select(
            tpl,
            defaults,
            &[
                "past.openCount",
                "past.cartCount",
                "past.orderCount",
                "past.buyoutCount",
                "past.cancelCount",
                "selected.cancelCount",
                "selected.addToWishlist",
                "stocks.wb",
            ],
            "[Калькулятор] Воронка: прошлый период, отмены, желаемое, остатки WB.",
            log,
        );
        let span = (end - start).num_days() + 1;
        run_log::calc_info(
            log,
            &format!(
                "[Калькулятор] Воронка: период {}–{} ({span} дн.); прошлый период той же длины → всего ~{} дн. статистики.",
                start.format("%d.%m.%Y"),
                end.format("%d.%m.%Y"),
                span * 2
            ),
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("Advertising") {
        // A=cabinet … D=advertId — источник для Инфо (A2:A / D2:D).
        force_ordered_select(
            tpl,
            defaults,
            &["cabinet", "typeName", "statusName", "advertId", "changeTime"],
            &[],
            "[Калькулятор] Реклама/Кампании: cabinet+type+status+advertId+changeTime (ID в D).",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("Advertising.Info") {
        force_only_select(
            tpl,
            defaults,
            &["cabinet", "advertId", "nmId"],
            "[Калькулятор] Реклама/Инфо: cabinet+advertId+nmId; источник ID — Кампании!A2:A / D2:D.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("Advertising.Expenses") {
        let end = Local::now().date_naive() - Duration::days(1);
        let start = end - Duration::days(13);
        tpl.date_from = start.format("%Y-%m-%d").to_string();
        tpl.date_to = end.format("%Y-%m-%d").to_string();
        tpl.date_template_type = "2weeks".into();
        run_log::calc_info(
            log,
            &format!(
                "[Калькулятор] Реклама/Расходы: период {}–{} (2 недели).",
                tpl.date_from, tpl.date_to
            ),
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("OzProductReport") {
        force_ordered_select(
            tpl,
            defaults,
            &["cabinet", "sku"],
            &[
                "offer_id",
                "ozon_product_id",
                "product_name",
                "brand",
                "category",
                "type",
                "rating",
                "reviews",
                "price",
                "old_price",
                "fbo_available",
                "fbs_available",
                "volume_l",
            ],
            "[Калькулятор] Отчёт Ozon: SKU в колонке B.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("OzPrices") {
        wire_ozon_sku_source(profile, tpl, log, &table.label_tr());
        force_select(
            tpl,
            defaults,
            &[
                "sku",
                "price",
                "old_price",
                "net_price",
                "marketing_seller_price",
                "sales_percent_fbo",
                "sales_percent_fbs",
                "fbs_direct_flow_trans_min_amount",
                "fbs_direct_flow_trans_max_amount",
                "fbs_deliv_to_customer_amount",
            ],
            "[Калькулятор] Цены Ozon: по SKU.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("OzStocks") {
        wire_ozon_sku_source(profile, tpl, log, &table.label_tr());
        force_select(
            tpl,
            defaults,
            &["sku", "present", "reserved", "has_stock", "source"],
            "[Калькулятор] Остатки Ozon: по SKU.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("OzFbsStocks") {
        wire_ozon_sku_source(profile, tpl, log, &table.label_tr());
        force_select(
            tpl,
            defaults,
            &["sku", "present", "reserved", "shipment_type", "type"],
            "[Калькулятор] Остатки FBS Ozon: по SKU.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("OzAttributes") {
        wire_ozon_sku_source(profile, tpl, log, &table.label_tr());
        force_select(
            tpl,
            defaults,
            &[
                "sku",
                "depth",
                "height",
                "width",
                "weight",
                "dimension_unit",
                "weight_unit",
                "primary_image",
                "brand_attribute",
                "name",
            ],
            "[Калькулятор] Атрибуты Ozon: по SKU.",
            log,
        );
        return;
    }

    if table.key.eq_ignore_ascii_case("OzFboOrders")
        || table.key.eq_ignore_ascii_case("OzFbsOrders")
    {
        // ~28 дней (пресет 4week в ).
        let end = Local::now().date_naive() - Duration::days(1);
        let start = end - Duration::days(27);
        tpl.date_from = start.format("%Y-%m-%d").to_string();
        tpl.date_to = end.format("%Y-%m-%d").to_string();
        tpl.date_template_type = "4week".into();
        run_log::calc_info(
            log,
            &format!(
                "[Калькулятор] {}: период {}–{} (28 дн.).",
                table.label_tr(), tpl.date_from, tpl.date_to
            ),
        );
    }
}

fn wire_ozon_sku_source(
    profile: &CalculatorProfile,
    tpl: &mut Template,
    log: &AppLog,
    table_label: &str,
) {
    tpl.id_mode = "sku".into();
    tpl.sku_source_range = "B2:B".into();
    tpl.id_source_range = "B2:B".into();
    let report_url = profile
        .table_links
        .iter()
        .find(|(k, v)| k.eq_ignore_ascii_case("OzProductReport") && !v.trim().is_empty())
        .map(|(_, v)| v.clone());
    let Some(url) = report_url else {
        run_log::calc_info(
            log,
            &format!(
                "[Калькулятор] {table_label}: нет ссылки на «Отчёт по товарам Ozon» — SKU-источник не задан."
            ),
        );
        return;
    };
    let Some(report_id) = sheet_util::extract_spreadsheet_id(&url) else {
        run_log::calc_info(
            log,
            &format!("[Калькулятор] {table_label}: не удалось извлечь ID отчёта Ozon."),
        );
        return;
    };
    let report_def = catalog::find_table("OzProductReport");
    tpl.second_spreadsheet_id = report_id;
    tpl.second_sheet_name = report_def
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| "Ozon отчёт товары универсал".into());
    run_log::calc_info(
        log,
        &format!(
            "[Калькулятор] {table_label}: SKU из отчёта ({}, B2:B).",
            tpl.second_spreadsheet_id
        ),
    );
}

fn force_select(
    tpl: &mut Template,
    defaults: &[ScriptField],
    force_names: &[&str],
    message: &str,
    log: &AppLog,
) {
    let base = if defaults.is_empty() {
        tpl.selected_fields.clone()
    } else {
        defaults.to_vec()
    };
    tpl.selected_fields = base
        .into_iter()
        .map(|f| {
            let selected = f.selected
                || force_names
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(f.name.as_str()));
            ScriptField {
                name: f.name,
                title: f.title,
                selected,
            }
        })
        .collect();
    run_log::calc_info(log, message);
}

/// Только перечисленные поля (Selected=true), остальные выключены — порядок как в `names`.
fn force_only_select(
    tpl: &mut Template,
    defaults: &[ScriptField],
    names: &[&str],
    message: &str,
    log: &AppLog,
) {
    let base = if defaults.is_empty() {
        tpl.selected_fields.clone()
    } else {
        defaults.to_vec()
    };
    let mut ordered = Vec::new();
    for name in names {
        if let Some(f) = base.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
            ordered.push(ScriptField {
                name: f.name.clone(),
                title: f.title.clone(),
                selected: true,
            });
        }
    }
    tpl.selected_fields = ordered;
    run_log::calc_info(log, message);
}

fn force_ordered_select(
    tpl: &mut Template,
    defaults: &[ScriptField],
    first: &[&str],
    also: &[&str],
    message: &str,
    log: &AppLog,
) {
    let base = if defaults.is_empty() {
        tpl.selected_fields.clone()
    } else {
        defaults.to_vec()
    };
    let mut keep: std::collections::HashSet<String> = first
        .iter()
        .chain(also.iter())
        .map(|s| (*s).to_string())
        .collect();
    for f in &base {
        if f.selected {
            keep.insert(f.name.clone());
        }
    }
    let mut ordered = Vec::new();
    for name in first {
        if let Some(f) = base.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
            ordered.push(ScriptField {
                name: f.name.clone(),
                title: f.title.clone(),
                selected: true,
            });
        }
    }
    for f in &base {
        if first
            .iter()
            .any(|n| n.eq_ignore_ascii_case(f.name.as_str()))
        {
            continue;
        }
        let selected = keep.iter().any(|n| n.eq_ignore_ascii_case(&f.name));
        if selected {
            ordered.push(ScriptField {
                name: f.name.clone(),
                title: f.title.clone(),
                selected: true,
            });
        }
    }
    tpl.selected_fields = ordered;
    run_log::calc_info(log, message);
}
