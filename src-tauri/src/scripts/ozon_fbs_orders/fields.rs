use crate::core::script_field::ScriptField;
use crate::scripts::ozon_util::{
    format_number_cell, format_ozon_dt, join_str_array, json_str, money_amount_currency, yes_no,
};
use serde_json::Value;
use std::collections::HashMap;

pub use crate::scripts::ozon_fbo_orders::fields::wants_prefix;

pub fn default_fields() -> Vec<ScriptField> {
    let fields = vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_fbs_orders.cabinet"), true),
        ScriptField::new("order_id", crate::core::i18n::t("fields.ozon_fbs_orders.order_id"), true),
        ScriptField::new("order_number", crate::core::i18n::t("fields.ozon_fbs_orders.order_number"), true),
        ScriptField::new("posting_number", crate::core::i18n::t("fields.ozon_fbs_orders.posting_number"), true),
        ScriptField::new("parent_posting_number", crate::core::i18n::t("fields.ozon_fbs_orders.parent_posting_number"), false),
        ScriptField::new("status", crate::core::i18n::t("fields.ozon_fbs_orders.status"), true),
        ScriptField::new("substatus", crate::core::i18n::t("fields.ozon_fbs_orders.substatus"), true),
        ScriptField::new("tracking_number", crate::core::i18n::t("fields.ozon_fbs_orders.tracking_number"), false),
        ScriptField::new("delivering_date", crate::core::i18n::t("fields.ozon_fbs_orders.delivering_date"), false),
        ScriptField::new("shipment_date", crate::core::i18n::t("fields.ozon_fbs_orders.shipment_date"), false),
        ScriptField::new("in_process_at", crate::core::i18n::t("fields.ozon_fbs_orders.in_process_at"), true),
        ScriptField::new("product_sku", crate::core::i18n::t("fields.ozon_fbs_orders.product_sku"), true),
        ScriptField::new("product_name", crate::core::i18n::t("fields.ozon_fbs_orders.product_name"), true),
        ScriptField::new("product_quantity", crate::core::i18n::t("fields.ozon_fbs_orders.product_quantity"), true),
        ScriptField::new("product_offer_id", crate::core::i18n::t("fields.ozon_fbs_orders.product_offer_id"), true),
        ScriptField::new("product_price", crate::core::i18n::t("fields.ozon_fbs_orders.product_price"), true),
        ScriptField::new("product_currency_code", crate::core::i18n::t("fields.ozon_fbs_orders.product_currency_code"), true),
        ScriptField::new("product_is_marketplace_buyout", crate::core::i18n::t("fields.ozon_fbs_orders.product_is_marketplace_buyout"), false),
        ScriptField::new("product_is_blr_traceable", crate::core::i18n::t("fields.ozon_fbs_orders.product_is_blr_traceable"), false),
        ScriptField::new("delivery_method_id", crate::core::i18n::t("fields.ozon_fbs_orders.delivery_method_id"), false),
        ScriptField::new("delivery_method_name", crate::core::i18n::t("fields.ozon_fbs_orders.delivery_method_name"), false),
        ScriptField::new("customer_id", crate::core::i18n::t("fields.ozon_fbs_orders.customer_id"), false),
        ScriptField::new("customer_name", crate::core::i18n::t("fields.ozon_fbs_orders.customer_name"), false),
        ScriptField::new("customer_phone", crate::core::i18n::t("fields.ozon_fbs_orders.customer_phone"), false),
        ScriptField::new("cancellation_cancel_reason", crate::core::i18n::t("fields.ozon_fbs_orders.cancellation_cancel_reason"), false),
        ScriptField::new("cancellation_cancel_reason_id", crate::core::i18n::t("fields.ozon_fbs_orders.cancellation_cancel_reason_id"), false),
        ScriptField::new("cancellation_initiator", crate::core::i18n::t("fields.ozon_fbs_orders.cancellation_initiator"), false),
        ScriptField::new("cancellation_type", crate::core::i18n::t("fields.ozon_fbs_orders.cancellation_type"), false),
        ScriptField::new("analytics_city", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_city"), false),
        ScriptField::new("analytics_region", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_region"), false),
        ScriptField::new("analytics_delivery_type", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_delivery_type"), false),
        ScriptField::new("analytics_is_premium", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_is_premium"), false),
        ScriptField::new(
            "analytics_payment_type_group_name",
            "Аналитика: Способ оплаты",
            false,
        ),
        ScriptField::new("analytics_warehouse_id", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_warehouse_id"), false),
        ScriptField::new("analytics_warehouse_name", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_warehouse_name"), false),
        ScriptField::new("analytics_is_legal", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_is_legal"), false),
        ScriptField::new(
            "analytics_delivery_date_begin",
            "Аналитика: Дата начала доставки",
            false,
        ),
        ScriptField::new(
            "analytics_delivery_date_end",
            "Аналитика: Дата окончания доставки",
            false,
        ),
        ScriptField::new(
            "analytics_client_delivery_date_begin",
            "Аналитика: Дата начала доставки (клиент)",
            false,
        ),
        ScriptField::new(
            "analytics_client_delivery_date_end",
            "Аналитика: Дата окончания доставки (клиент)",
            false,
        ),
        ScriptField::new("analytics_tpl_provider", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_tpl_provider"), false),
        ScriptField::new("analytics_tpl_provider_id", crate::core::i18n::t("fields.ozon_fbs_orders.analytics_tpl_provider_id"), false),
        ScriptField::new("financial_commission_amount", crate::core::i18n::t("fields.ozon_fbs_orders.financial_commission_amount"), false),
        ScriptField::new("financial_commission_percent", crate::core::i18n::t("fields.ozon_fbs_orders.financial_commission_percent"), false),
        ScriptField::new("financial_payout", crate::core::i18n::t("fields.ozon_fbs_orders.financial_payout"), false),
        ScriptField::new("financial_old_price", crate::core::i18n::t("fields.ozon_fbs_orders.financial_old_price"), false),
        ScriptField::new("financial_price", crate::core::i18n::t("fields.ozon_fbs_orders.financial_price"), false),
        ScriptField::new("financial_total_discount_value", crate::core::i18n::t("fields.ozon_fbs_orders.financial_total_discount_value"), false),
        ScriptField::new("financial_total_discount_percent", crate::core::i18n::t("fields.ozon_fbs_orders.financial_total_discount_percent"), false),
        ScriptField::new("financial_actions", crate::core::i18n::t("fields.ozon_fbs_orders.financial_actions"), false),
        ScriptField::new("financial_currency_code", crate::core::i18n::t("fields.ozon_fbs_orders.financial_currency_code"), false),
        ScriptField::new(
            "financial_commissions_currency_code",
            "Финансы: Валюта комиссий",
            false,
        ),
        ScriptField::new("financial_cluster_from", crate::core::i18n::t("fields.ozon_fbs_orders.financial_cluster_from"), false),
        ScriptField::new("financial_cluster_to", crate::core::i18n::t("fields.ozon_fbs_orders.financial_cluster_to"), false),
        ScriptField::new("legal_company_name", crate::core::i18n::t("fields.ozon_fbs_orders.legal_company_name"), false),
        ScriptField::new("legal_inn", crate::core::i18n::t("fields.ozon_fbs_orders.legal_inn"), false),
        ScriptField::new("legal_kpp", crate::core::i18n::t("fields.ozon_fbs_orders.legal_kpp"), false),
        ScriptField::new("barcodes_lower", crate::core::i18n::t("fields.ozon_fbs_orders.barcodes_lower"), false),
        ScriptField::new("barcodes_upper", crate::core::i18n::t("fields.ozon_fbs_orders.barcodes_upper"), false),
    ];
    fields
}

pub fn filter_export_fields(
    fields: &[ScriptField],
    flags: (bool, bool, bool, bool),
) -> Vec<ScriptField> {
    let (analytics, financial, legal, barcodes) = flags;
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
            if f.name.starts_with("barcodes_") {
                return barcodes;
            }
            true
        })
        .cloned()
        .collect()
}

fn cell(map: &HashMap<String, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

pub fn build_rows_from_posting(
    cabinet: &str,
    posting: &Value,
    fields: &[ScriptField],
    flags: (bool, bool, bool, bool),
) -> Vec<Vec<String>> {
    let (analytics, financial, legal, barcodes) = flags;
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
            analytics,
            financial,
            legal,
            barcodes,
            None,
            None,
        )];
    }
    products
        .iter()
        .map(|p| {
            let fp = find_financial_product(posting, p);
            build_row(
                cabinet,
                posting,
                fields,
                analytics,
                financial,
                legal,
                barcodes,
                Some(p),
                fp,
            )
        })
        .collect()
}

fn find_financial_product<'a>(posting: &'a Value, product: &'a Value) -> Option<&'a Value> {
    let sku = product.get("sku")?.as_i64()?;
    posting
        .pointer("/financial_data/products")
        .and_then(|a| a.as_array())?
        .iter()
        .find(|fp| fp.get("product_id").and_then(|x| x.as_i64()) == Some(sku))
}

fn build_row(
    cabinet: &str,
    posting: &Value,
    fields: &[ScriptField],
    analytics: bool,
    financial: bool,
    legal: bool,
    barcodes: bool,
    product: Option<&Value>,
    financial_product: Option<&Value>,
) -> Vec<String> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("cabinet".into(), cabinet.to_string());
    for (k, v) in [
        ("order_id", posting.get("order_id")),
        ("order_number", posting.get("order_number")),
        ("posting_number", posting.get("posting_number")),
        ("parent_posting_number", posting.get("parent_posting_number")),
        ("status", posting.get("status")),
        ("substatus", posting.get("substatus")),
        ("tracking_number", posting.get("tracking_number")),
    ] {
        map.insert(k.into(), json_str(v));
    }
    map.insert(
        "delivering_date".into(),
        format_ozon_dt(&json_str(posting.get("delivering_date"))),
    );
    map.insert(
        "shipment_date".into(),
        format_ozon_dt(&json_str(posting.get("shipment_date"))),
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
        // v4: price = { amount, currency }; v3: price string + currency_code.
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
        map.insert(
            "product_is_blr_traceable".into(),
            yes_no(p.get("is_blr_traceable").and_then(|x| x.as_bool())),
        );
    }

    if let Some(d) = posting.get("delivery_method") {
        map.insert("delivery_method_id".into(), json_str(d.get("id")));
        map.insert("delivery_method_name".into(), json_str(d.get("name")));
    }
    if let Some(c) = posting.get("customer") {
        map.insert("customer_id".into(), json_str(c.get("customer_id")));
        map.insert("customer_name".into(), json_str(c.get("name")));
        map.insert("customer_phone".into(), json_str(c.get("phone")));
    }
    if let Some(c) = posting.get("cancellation") {
        map.insert("cancellation_cancel_reason".into(), json_str(c.get("cancel_reason")));
        map.insert(
            "cancellation_cancel_reason_id".into(),
            json_str(c.get("cancel_reason_id")),
        );
        map.insert("cancellation_initiator".into(), json_str(c.get("cancellation_initiator")));
        map.insert("cancellation_type".into(), json_str(c.get("cancellation_type")));
    }

    if analytics {
        if let Some(a) = posting.get("analytics_data") {
            map.insert("analytics_city".into(), json_str(a.get("city")));
            map.insert("analytics_region".into(), json_str(a.get("region")));
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
            map.insert(
                "analytics_warehouse_name".into(),
                json_str(a.get("warehouse").or_else(|| a.get("warehouse_name"))),
            );
            map.insert(
                "analytics_is_legal".into(),
                yes_no(a.get("is_legal").and_then(|x| x.as_bool())),
            );
            for (fk, ak) in [
                ("analytics_delivery_date_begin", "delivery_date_begin"),
                ("analytics_delivery_date_end", "delivery_date_end"),
                ("analytics_client_delivery_date_begin", "client_delivery_date_begin"),
                ("analytics_client_delivery_date_end", "client_delivery_date_end"),
            ] {
                map.insert(fk.into(), format_ozon_dt(&json_str(a.get(ak))));
            }
            map.insert("analytics_tpl_provider".into(), json_str(a.get("tpl_provider")));
            map.insert("analytics_tpl_provider_id".into(), json_str(a.get("tpl_provider_id")));
        }
    }

    if financial {
        if let Some(fp) = financial_product {
            // v4: commission = { amount, currency, percent }; v3: commission_amount / commission_percent.
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
            let (_, cust_cur) = money_amount_currency(fp.get("customer_price"));
            map.insert(
                "financial_currency_code".into(),
                if cust_cur.is_empty() {
                    json_str(fp.get("currency_code"))
                } else {
                    cust_cur
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
        if let Some(fd) = posting.get("financial_data") {
            map.insert("financial_cluster_from".into(), json_str(fd.get("cluster_from")));
            map.insert("financial_cluster_to".into(), json_str(fd.get("cluster_to")));
        }
    }

    if legal {
        if let Some(l) = posting.get("legal_info") {
            map.insert("legal_company_name".into(), json_str(l.get("company_name")));
            map.insert("legal_inn".into(), json_str(l.get("inn")));
            map.insert("legal_kpp".into(), json_str(l.get("kpp")));
        }
    }

    if barcodes {
        if let Some(b) = posting.get("barcodes") {
            map.insert("barcodes_lower".into(), json_str(b.get("lower_barcode")));
            map.insert("barcodes_upper".into(), json_str(b.get("upper_barcode")));
        }
    }

    selected.iter().map(|f| cell(&map, &f.name)).collect()
}

pub fn export_fields_from_ctx(ctx_fields: &[ScriptField], ctx: &crate::core::script::ScriptContext) -> Vec<ScriptField> {
    use crate::scripts::ozon_util::param_bool;
    let analytics = param_bool(ctx, "ozon_fbs_orders_include_analytics_data")
        || wants_prefix(ctx_fields, "analytics_");
    let financial = param_bool(ctx, "ozon_fbs_orders_include_financial_data")
        || wants_prefix(ctx_fields, "financial_");
    let legal = param_bool(ctx, "ozon_fbs_orders_include_legal_info")
        || wants_prefix(ctx_fields, "legal_");
    let barcodes = param_bool(ctx, "ozon_fbs_orders_include_barcodes")
        || wants_prefix(ctx_fields, "barcodes_");
    filter_export_fields(ctx_fields, (analytics, financial, legal, barcodes))
}
