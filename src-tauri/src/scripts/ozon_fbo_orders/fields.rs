use crate::core::script_field::ScriptField;
use crate::scripts::ozon_util::{
    format_number_cell, format_ozon_dt, join_str_array, json_str, money_amount_currency, yes_no,
};
use serde_json::Value;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_fbo_orders.cabinet"), true),
        ScriptField::new("order_id", crate::core::i18n::t("fields.ozon_fbo_orders.order_id"), true),
        ScriptField::new("order_number", crate::core::i18n::t("fields.ozon_fbo_orders.order_number"), true),
        ScriptField::new("posting_number", crate::core::i18n::t("fields.ozon_fbo_orders.posting_number"), true),
        ScriptField::new("status", crate::core::i18n::t("fields.ozon_fbo_orders.status"), true),
        ScriptField::new("substatus", crate::core::i18n::t("fields.ozon_fbo_orders.substatus"), true),
        ScriptField::new("cancel_reason_id", crate::core::i18n::t("fields.ozon_fbo_orders.cancel_reason_id"), true),
        ScriptField::new("created_at", crate::core::i18n::t("fields.ozon_fbo_orders.created_at"), true),
        ScriptField::new("in_process_at", crate::core::i18n::t("fields.ozon_fbo_orders.in_process_at"), true),
        ScriptField::new("product_sku", crate::core::i18n::t("fields.ozon_fbo_orders.product_sku"), true),
        ScriptField::new("product_name", crate::core::i18n::t("fields.ozon_fbo_orders.product_name"), true),
        ScriptField::new("product_quantity", crate::core::i18n::t("fields.ozon_fbo_orders.product_quantity"), true),
        ScriptField::new("product_offer_id", crate::core::i18n::t("fields.ozon_fbo_orders.product_offer_id"), true),
        ScriptField::new("product_price", crate::core::i18n::t("fields.ozon_fbo_orders.product_price"), true),
        ScriptField::new("product_currency_code", crate::core::i18n::t("fields.ozon_fbo_orders.product_currency_code"), true),
        ScriptField::new("product_is_marketplace_buyout", crate::core::i18n::t("fields.ozon_fbo_orders.product_is_marketplace_buyout"), true),
        ScriptField::new("analytics_city", crate::core::i18n::t("fields.ozon_fbo_orders.analytics_city"), false),
        ScriptField::new("analytics_delivery_type", crate::core::i18n::t("fields.ozon_fbo_orders.analytics_delivery_type"), false),
        ScriptField::new("analytics_is_premium", crate::core::i18n::t("fields.ozon_fbo_orders.analytics_is_premium"), false),
        ScriptField::new(
            "analytics_payment_type_group_name",
            "Аналитика: Способ оплаты",
            false,
        ),
        ScriptField::new("analytics_warehouse_id", crate::core::i18n::t("fields.ozon_fbo_orders.analytics_warehouse_id"), false),
        ScriptField::new("analytics_warehouse_name", crate::core::i18n::t("fields.ozon_fbo_orders.analytics_warehouse_name"), false),
        ScriptField::new("analytics_is_legal", crate::core::i18n::t("fields.ozon_fbo_orders.analytics_is_legal"), false),
        ScriptField::new(
            "analytics_client_delivery_date_begin",
            "Аналитика: Дата начала доставки",
            false,
        ),
        ScriptField::new(
            "analytics_client_delivery_date_end",
            "Аналитика: Дата окончания доставки",
            false,
        ),
        ScriptField::new("financial_commission_amount", crate::core::i18n::t("fields.ozon_fbo_orders.financial_commission_amount"), false),
        ScriptField::new("financial_commission_percent", crate::core::i18n::t("fields.ozon_fbo_orders.financial_commission_percent"), false),
        ScriptField::new("financial_payout", crate::core::i18n::t("fields.ozon_fbo_orders.financial_payout"), false),
        ScriptField::new("financial_old_price", crate::core::i18n::t("fields.ozon_fbo_orders.financial_old_price"), false),
        ScriptField::new("financial_price", crate::core::i18n::t("fields.ozon_fbo_orders.financial_price"), false),
        ScriptField::new("financial_total_discount_value", crate::core::i18n::t("fields.ozon_fbo_orders.financial_total_discount_value"), false),
        ScriptField::new("financial_total_discount_percent", crate::core::i18n::t("fields.ozon_fbo_orders.financial_total_discount_percent"), false),
        ScriptField::new("financial_actions", crate::core::i18n::t("fields.ozon_fbo_orders.financial_actions"), false),
        ScriptField::new("financial_currency_code", crate::core::i18n::t("fields.ozon_fbo_orders.financial_currency_code"), false),
        ScriptField::new(
            "financial_commissions_currency_code",
            "Финансы: Валюта комиссий",
            false,
        ),
        ScriptField::new("legal_company_name", crate::core::i18n::t("fields.ozon_fbo_orders.legal_company_name"), false),
        ScriptField::new("legal_inn", crate::core::i18n::t("fields.ozon_fbo_orders.legal_inn"), false),
        ScriptField::new("legal_kpp", crate::core::i18n::t("fields.ozon_fbo_orders.legal_kpp"), false),
    ]
}

fn cell(map: &HashMap<String, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

pub fn build_rows_from_posting(
    cabinet: &str,
    posting: &Value,
    fields: &[ScriptField],
    include_analytics: bool,
    include_financial: bool,
    include_legal: bool,
) -> Vec<Vec<String>> {
    let fin_by_sku = financial_by_sku(posting);
    let products = posting
        .get("products")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    if products.is_empty() {
        return vec![build_row(
            cabinet,
            posting,
            fields,
            include_analytics,
            include_financial,
            include_legal,
            None,
            None,
        )];
    }

    products
        .iter()
        .map(|p| {
            let sku = p.get("sku").and_then(|x| x.as_i64());
            let fp = sku.and_then(|s| fin_by_sku.get(&s));
            build_row(
                cabinet,
                posting,
                fields,
                include_analytics,
                include_financial,
                include_legal,
                Some(p),
                fp.copied(),
            )
        })
        .collect()
}

fn financial_by_sku(posting: &Value) -> HashMap<i64, &Value> {
    let mut map = HashMap::new();
    if let Some(arr) = posting
        .pointer("/financial_data/products")
        .and_then(|a| a.as_array())
    {
        for fp in arr {
            if let Some(id) = fp.get("product_id").and_then(|x| x.as_i64()) {
                map.entry(id).or_insert(fp);
            }
        }
    }
    map
}

fn build_row(
    cabinet: &str,
    posting: &Value,
    fields: &[ScriptField],
    include_analytics: bool,
    include_financial: bool,
    include_legal: bool,
    product: Option<&Value>,
    financial_product: Option<&Value>,
) -> Vec<String> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("cabinet".into(), cabinet.to_string());
    map.insert("order_id".into(), json_str(posting.get("order_id")));
    map.insert("order_number".into(), json_str(posting.get("order_number")));
    map.insert("posting_number".into(), json_str(posting.get("posting_number")));
    map.insert("status".into(), json_str(posting.get("status")));
    map.insert("substatus".into(), json_str(posting.get("substatus")));
    map.insert("cancel_reason_id".into(), json_str(posting.get("cancel_reason_id")));
    map.insert(
        "created_at".into(),
        format_ozon_dt(&json_str(posting.get("created_at"))),
    );
    map.insert(
        "in_process_at".into(),
        format_ozon_dt(&json_str(posting.get("in_process_at"))),
    );

    if let Some(p) = product {
        map.insert("product_sku".into(), json_str(p.get("sku")));
        map.insert("product_name".into(), json_str(p.get("name")));
        map.insert("product_quantity".into(), json_str(p.get("quantity")));
        map.insert("product_offer_id".into(), json_str(p.get("offer_id")));
        // v3: price = { amount, currency }; старый v2: строка + currency_code.
        let (price, currency) = money_amount_currency(p.get("price"));
        map.insert(
            "product_price".into(),
            if price.is_empty() {
                json_str(p.get("price"))
            } else {
                price
            },
        );
        map.insert(
            "product_currency_code".into(),
            if currency.is_empty() {
                json_str(p.get("currency_code"))
            } else {
                currency
            },
        );
        map.insert(
            "product_is_marketplace_buyout".into(),
            yes_no(p.get("is_marketplace_buyout").and_then(|x| x.as_bool())),
        );
    }

    if include_analytics {
        if let Some(a) = posting.get("analytics_data") {
            map.insert("analytics_city".into(), json_str(a.get("city")));
            map.insert("analytics_delivery_type".into(), json_str(a.get("delivery_type")));
            map.insert(
                "analytics_is_premium".into(),
                yes_no(a.get("is_premium").and_then(|x| x.as_bool())),
            );
            map.insert(
                "analytics_payment_type_group_name".into(),
                json_str(a.get("payment_type_group_name")),
            );
            map.insert("analytics_warehouse_id".into(), json_str(a.get("warehouse_id")));
            map.insert("analytics_warehouse_name".into(), json_str(a.get("warehouse_name")));
            map.insert(
                "analytics_is_legal".into(),
                yes_no(a.get("is_legal").and_then(|x| x.as_bool())),
            );
            map.insert(
                "analytics_client_delivery_date_begin".into(),
                format_ozon_dt(&json_str(a.get("client_delivery_date_begin"))),
            );
            map.insert(
                "analytics_client_delivery_date_end".into(),
                format_ozon_dt(&json_str(a.get("client_delivery_date_end"))),
            );
        }
    }

    if include_financial {
        if let Some(fp) = financial_product {
            let (comm_amt, comm_cur) = money_amount_currency(fp.get("commission"));
            map.insert(
                "financial_commission_amount".into(),
                if comm_amt.is_empty() {
                    format_number_cell(fp.get("commission_amount"))
                } else {
                    comm_amt
                },
            );
            map.insert(
                "financial_commission_percent".into(),
                json_str(
                    fp.get("commission")
                        .and_then(|c| c.get("percent"))
                        .or_else(|| fp.get("commission_percent")),
                ),
            );
            map.insert("financial_payout".into(), format_number_cell(fp.get("payout")));
            map.insert(
                "financial_old_price".into(),
                format_number_cell(fp.get("old_price")),
            );
            map.insert("financial_price".into(), format_number_cell(fp.get("price")));
            map.insert(
                "financial_total_discount_value".into(),
                format_number_cell(fp.get("total_discount_value")),
            );
            map.insert(
                "financial_total_discount_percent".into(),
                format_number_cell(fp.get("total_discount_percent")),
            );
            map.insert(
                "financial_actions".into(),
                join_str_array(fp.get("actions"), "; "),
            );
            map.insert(
                "financial_currency_code".into(),
                if comm_cur.is_empty() {
                    json_str(fp.get("currency_code"))
                } else {
                    comm_cur.clone()
                },
            );
            map.insert(
                "financial_commissions_currency_code".into(),
                if comm_cur.is_empty() {
                    json_str(fp.get("commissions_currency_code"))
                } else {
                    comm_cur
                },
            );
        }
    }

    if include_legal {
        if let Some(l) = posting.get("legal_info") {
            map.insert("legal_company_name".into(), json_str(l.get("company_name")));
            map.insert("legal_inn".into(), json_str(l.get("inn")));
            map.insert("legal_kpp".into(), json_str(l.get("kpp")));
        }
    }

    selected.iter().map(|f| cell(&map, &f.name)).collect()
}

pub fn fields_for_export(fields: &[ScriptField], ctx_flags: (bool, bool, bool)) -> Vec<ScriptField> {
    let (analytics, financial, legal) = ctx_flags;
    fields
        .iter()
        .filter(|f| {
            if !f.selected {
                return false;
            }
            if f.name.starts_with("analytics_") {
                return analytics;
            }
            if f.name.starts_with("financial_") {
                return financial;
            }
            if f.name.starts_with("legal_") {
                return legal;
            }
            true
        })
        .cloned()
        .collect()
}

pub fn wants_prefix(fields: &[ScriptField], prefix: &str) -> bool {
    fields
        .iter()
        .any(|f| f.selected && f.name.starts_with(prefix))
}
