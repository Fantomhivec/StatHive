use crate::core::script_field::ScriptField;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_product_report.cabinet"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_product_report.offer_id"), true),
        ScriptField::new("ozon_product_id", crate::core::i18n::t("fields.ozon_product_report.ozon_product_id"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_product_report.sku"), true),
        ScriptField::new("barcode", crate::core::i18n::t("fields.ozon_product_report.barcode"), true),
        ScriptField::new("product_name", crate::core::i18n::t("fields.ozon_product_report.product_name"), true),
        ScriptField::new("content_rating", crate::core::i18n::t("fields.ozon_product_report.content_rating"), true),
        ScriptField::new("brand", crate::core::i18n::t("fields.ozon_product_report.brand"), true),
        ScriptField::new("product_status", crate::core::i18n::t("fields.ozon_product_report.product_status"), true),
        ScriptField::new("labels", crate::core::i18n::t("fields.ozon_product_report.labels"), true),
        ScriptField::new("reviews", crate::core::i18n::t("fields.ozon_product_report.reviews"), true),
        ScriptField::new("rating", crate::core::i18n::t("fields.ozon_product_report.rating"), true),
        ScriptField::new("hide_reasons", crate::core::i18n::t("fields.ozon_product_report.hide_reasons"), true),
        ScriptField::new("created_at", crate::core::i18n::t("fields.ozon_product_report.created_at"), true),
        ScriptField::new("category", crate::core::i18n::t("fields.ozon_product_report.category"), true),
        ScriptField::new("type", crate::core::i18n::t("fields.ozon_product_report.type"), true),
        ScriptField::new("volume_l", crate::core::i18n::t("fields.ozon_product_report.volume_l"), true),
        ScriptField::new("volumetric_weight_kg", crate::core::i18n::t("fields.ozon_product_report.volumetric_weight_kg"), true),
        ScriptField::new(
            "fbo_available",
            "Доступно к продаже по схеме FBO, шт.",
            true,
        ),
        ScriptField::new("reserved", crate::core::i18n::t("fields.ozon_product_report.reserved"), true),
        ScriptField::new(
            "fbs_available",
            "Доступно к продаже по схеме FBS, шт.",
            true,
        ),
        ScriptField::new(
            "realfbs_available",
            "Доступно к продаже по схеме realFBS, шт.",
            true,
        ),
        ScriptField::new(
            "reserved_my_warehouses",
            "Зарезервировано на моих складах, шт",
            true,
        ),
        ScriptField::new("price", crate::core::i18n::t("fields.ozon_product_report.price"), true),
        ScriptField::new("old_price", crate::core::i18n::t("fields.ozon_product_report.old_price"), true),
        ScriptField::new("vat", crate::core::i18n::t("fields.ozon_product_report.vat"), true),
        ScriptField::new("errors", crate::core::i18n::t("fields.ozon_product_report.errors"), true),
        ScriptField::new("warnings", crate::core::i18n::t("fields.ozon_product_report.warnings"), true),
    ]
}

/// Строки отчёта без шапки: сопоставление колонок CSV/XLSX по Title поля.
pub fn project_rows(
    cabinet: &str,
    raw_rows: &[Vec<String>],
    fields: &[ScriptField],
) -> Vec<Vec<String>> {
    if raw_rows.is_empty() {
        return Vec::new();
    }
    let header = &raw_rows[0];
    let mut header_index: HashMap<String, usize> = HashMap::new();
    for (i, h) in header.iter().enumerate() {
        let key = h.trim().to_string();
        if !key.is_empty() && !header_index.contains_key(&key) {
            header_index.insert(key, i);
        }
    }

    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut out = Vec::new();
    for row in raw_rows.iter().skip(1) {
        let mut projected = Vec::with_capacity(selected.len());
        for f in &selected {
            if f.name == "cabinet" {
                projected.push(cabinet.to_string());
                continue;
            }
            let val = header_index
                .get(f.title.trim())
                .or_else(|| header_index.get(f.name.as_str()))
                .and_then(|&i| row.get(i))
                .cloned()
                .unwrap_or_default();
            projected.push(val);
        }
        out.push(projected);
    }
    out
}
